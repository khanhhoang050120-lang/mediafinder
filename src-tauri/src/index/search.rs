//! Parallel substring search with ranking and top-K selection.
//!
//! Three things the original design did not account for:
//!
//! * **Results must be ranked before they are cut.** Taking "the first 100
//!   matches" means the first 100 in MFT order, which is effectively random.
//!   Typing `avatar` has to put `Avatar.mkv` above
//!   `my_avatar_backup_2019.mkv`, and only a score can do that.
//!
//! * **The cut has to be deterministic.** `par_iter().filter().take(n)` does
//!   not even compile — `take` needs an `IndexedParallelIterator`, which
//!   `filter` does not produce — and `take_any` returns whichever items
//!   finished first, so the same query would give different answers on
//!   consecutive runs. Every hit is scored, then ordered by `(score, index)`,
//!   which is total and reproducible.
//!
//! * **A query is several words.** `avatar 2009 mkv` should narrow, not fail.
//!   Tokens are ANDed, and each contributes its own score.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicU64, Ordering};

use memchr::memmem::Finder;
use rayon::prelude::*;

use super::fold::fold;
use super::model::{Index, MediaKind};

/// Scores, highest first. Absolute values do not matter, only the ordering.
mod score {
    /// The whole filename, extension aside, is the token.
    pub const EXACT: i32 = 1000;
    /// The name starts with the token.
    pub const PREFIX: i32 = 800;
    /// The token starts a word inside the name.
    pub const WORD_START: i32 = 600;
    /// The token appears somewhere in the middle of a word.
    pub const SUBSTRING: i32 = 400;

    /// The token was not in the filename at all, but the folder path contains
    /// it at a word boundary.
    pub const DIR_WORD_START: i32 = 250;
    /// The token appears somewhere in the folder path.
    pub const DIR_SUBSTRING: i32 = 200;

    /// Shorter names are more specific, so nudge them up. Capped low enough
    /// that it can never outrank a better match class.
    pub const MAX_LENGTH_BONUS: i32 = 50;
}

/// Characters that begin a word inside a filename.
fn is_word_boundary(b: u8) -> bool {
    matches!(
        b,
        b' ' | b'_' | b'-' | b'.' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b',' | b'\''
            | b'+' | b'&' | b'@' | b'#' | b'~' | b'!' | b';' | b'\\' | b'/'
    )
}

/// Split a folded query into search tokens.
///
/// Splits on **every** non-alphanumeric character, not just whitespace.
///
/// Whitespace alone is not enough, and the failure is quiet. Pasting a real
/// title — `The anglerfish: The original approach to deep-sea fishing` — used
/// to yield the token `anglerfish:` with the colon still attached, which
/// matches nothing, because the file on disk is called `...The-anglerfish-...`.
/// The single most distinctive word in the query was lost to one punctuation
/// mark. Splitting `deep-sea` into `deep` and `sea` likewise lets it match
/// `deep_sea`, `deep sea` and `Deep-Sea` alike.
///
/// Matching is still substring-based, so a token never has to sit at a word
/// boundary in the filename — only in the query.
pub fn tokenize(folded: &str) -> Vec<&str> {
    folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect()
}

/// How much one matched token outweighs any quality-of-match difference.
///
/// Large enough that a file matching more of the query always outranks one
/// matching fewer, however good those fewer matches are.
const MATCHED_TOKEN_WEIGHT: i32 = 100_000;

/// One scored match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub index: u32,
    pub score: i32,
    /// How many of the query's tokens this entry actually contains.
    pub matched: u16,
}

// Ordered so a `BinaryHeap<Reverse<Hit>>` keeps the *worst* hit at the top and
// can evict it in O(log k). The index tiebreak makes the order total, which is
// what makes results reproducible across runs and across thread counts.
impl Ord for Hit {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .cmp(&other.score)
            .then_with(|| other.index.cmp(&self.index))
    }
}

impl PartialOrd for Hit {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub limit: usize,
    /// Restrict to these kinds. Empty means no restriction.
    pub kinds: Vec<MediaKind>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 5_000,
            kinds: Vec::new(),
        }
    }
}

/// Entries per parallel chunk.
///
/// Large enough that rayon's per-task overhead disappears against the scan
/// itself, small enough that work still spreads evenly across cores.
const CHUNK: usize = 16_384;

/// Why the results are what they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Relaxed {
    pub total_tokens: usize,
    /// The most tokens any single result managed to match.
    pub best_matched: usize,
}

#[derive(Debug, Default)]
pub struct SearchOutcome {
    pub hits: Vec<Hit>,
    /// `Some` when nothing matched the whole query and these are the closest
    /// partial matches instead.
    pub relaxed: Option<Relaxed>,
}

/// Search `index` for `query`.
///
/// Runs in two passes:
///
/// 1. **Strict** — every token must appear. Precise, and what a short query
///    wants: `avatar 2009` should not drag in every file containing `2009`.
///
/// 2. **Relaxed**, only if strict found nothing and the query has more than
///    one token. Results are ranked by how many tokens they matched.
///
/// The fallback exists because filenames are rarely the thing they describe.
/// Pasting a real video title finds nothing when the downloader truncated it:
/// `The anglerfish: The original approach to deep-sea fishing` is on disk as
/// `...The-anglerfish-The-original-approach-to-_Media_VqPMP9X-89o_001_1080p.mp4`,
/// where `deep`, `sea` and `fishing` simply do not exist. Demanding all nine
/// tokens returns nothing at all, which reads as "you don't have this file" —
/// the worst possible answer, because the file is right there. Six of nine
/// tokens is a very good answer, and the caller is told the results are
/// partial so the UI can say so.
///
/// `cancel` and `generation` let a search abandon its work the moment a newer
/// keystroke supersedes it. This is what replaces an input debounce — the
/// search starts immediately and the stale one gets out of the way, instead of
/// every query waiting out a timer.
pub fn search(
    index: &Index,
    query: &str,
    opts: &SearchOptions,
    cancel: &AtomicU64,
    generation: u64,
) -> SearchOutcome {
    let folded = fold(query);
    let tokens = tokenize(&folded);
    if tokens.is_empty() || index.is_empty() || opts.limit == 0 {
        return SearchOutcome::default();
    }

    let strict = run_pass(index, &tokens, opts, cancel, generation, 0);
    if !strict.is_empty() || tokens.len() < MIN_TOKENS_TO_RELAX {
        return SearchOutcome {
            hits: strict,
            relaxed: None,
        };
    }

    // Nothing contained the whole query. Take the closest matches instead.
    //
    // The floor of half the query stops a nine-word search returning every
    // file containing the word "the".
    let min_matched = tokens.len().div_ceil(2);
    let mut partial = run_pass(index, &tokens, opts, cancel, generation, min_matched);
    if partial.is_empty() {
        return SearchOutcome::default();
    }

    // Then keep only the *best* — the closest matches, not merely close ones.
    //
    // Measured on the real library, the query that prompted this returned two
    // files at 6 of 9 tokens followed by 171 at 5 of 9, all of them unrelated
    // files that merely happen to mention "deep sea". The two good answers
    // were correct and first, but burying them under a hundredfold of noise
    // makes the result list useless to look at. Hits are already ordered by
    // matched count, so this is a truncation, not another scan.
    let best_matched = partial[0].matched;
    if let Some(cut) = partial.iter().position(|h| h.matched < best_matched) {
        partial.truncate(cut);
    }

    SearchOutcome {
        hits: partial,
        relaxed: Some(Relaxed {
            total_tokens: tokens.len(),
            best_matched: best_matched as usize,
        }),
    }
}

/// Below this many tokens, a query that matches nothing simply matches nothing.
///
/// One or two words is a deliberate query — the user knows exactly what they
/// typed, and quietly widening it would return noise instead of an honest
/// empty result. Longer queries are usually a pasted title, and a filename is
/// rarely a faithful copy of the title it came from.
const MIN_TOKENS_TO_RELAX: usize = 3;

fn run_pass(
    index: &Index,
    tokens: &[&str],
    opts: &SearchOptions,
    cancel: &AtomicU64,
    generation: u64,
    // Fewest tokens an entry must match to count. `0` means "all of them".
    min_matched: usize,
) -> Vec<Hit> {

    // Built once and shared: constructing a Finder compiles a skip table, and
    // doing that per candidate rather than per token would dominate the scan.
    let finders: Vec<Finder> = tokens.iter().map(|t| Finder::new(t.as_bytes())).collect();
    let kinds = index.kinds();
    let dir_ids = index.dir_ids();

    // Score every directory once, up front.
    //
    // Searching the folder path as well as the filename is not optional: a
    // real library turned out to be organised as
    // `…\DATA TẠO VID HƯNG\HAN QUOC\13\BÀI 13 …\154.mp3`, where the filename
    // carries no meaning at all and every searchable word lives in the path.
    // Matching filenames only found nothing.
    //
    // Doing it per directory rather than per file is what keeps it cheap:
    // 116k files share 4k directories, so this is roughly 28x less work than
    // testing each file's path individually.
    let dir_scores = score_directories(index, tokens, &finders);

    let starts: Vec<usize> = (0..index.len()).step_by(CHUNK).collect();

    let partials: Vec<Vec<Hit>> = starts
        .into_par_iter()
        .map(|start| {
            if cancel.load(Ordering::Relaxed) != generation {
                return Vec::new();
            }
            let end = (start + CHUNK).min(index.len());
            // A bounded min-heap keeps only the best `limit` hits per chunk,
            // so a query matching half the library never materialises half the
            // library.
            //
            // Capacity is clamped to the chunk size: a chunk cannot yield more
            // hits than it holds entries, and reserving `limit` outright would
            // let a caller asking for a huge limit trigger an absurd
            // allocation per chunk.
            let cap = opts.limit.min(end - start).saturating_add(1);
            let mut heap: BinaryHeap<Reverse<Hit>> = BinaryHeap::with_capacity(cap);

            for i in start..end {
                if !opts.kinds.is_empty() && !opts.kinds.contains(&kinds[i]) {
                    continue;
                }
                let dir_row = &dir_scores[dir_ids[i] as usize * tokens.len()..][..tokens.len()];
                let Some((matched, score)) =
                    score_entry(index.folded(i), tokens, &finders, dir_row, min_matched)
                else {
                    continue;
                };
                let hit = Hit {
                    index: i as u32,
                    // Token count dominates so a file matching more of the
                    // query always wins. In the strict pass every hit matched
                    // every token, so this is a constant offset there and
                    // leaves the ordering untouched.
                    score: matched as i32 * MATCHED_TOKEN_WEIGHT + score,
                    matched,
                };
                if heap.len() < opts.limit {
                    heap.push(Reverse(hit));
                } else if let Some(&Reverse(worst)) = heap.peek() {
                    if hit > worst {
                        heap.pop();
                        heap.push(Reverse(hit));
                    }
                }
            }
            heap.into_iter().map(|Reverse(h)| h).collect()
        })
        .collect();

    if cancel.load(Ordering::Relaxed) != generation {
        return Vec::new();
    }

    let mut all: Vec<Hit> = partials.into_iter().flatten().collect();

    // Cut before sorting.
    //
    // Every chunk contributes up to `limit` hits, so this vector can hold
    // `chunks * limit` entries — around 155k for a 500k index at the default
    // limit. Sorting all of that to keep 5k of it was measurably the single
    // most expensive step in the whole search: at limit 5000 selection cost
    // 1.3 ms against 0.94 ms for the scan itself.
    //
    // `select_nth_unstable_by` partitions in O(n) so only the survivors get
    // sorted, turning ~2.6M comparisons into ~215k.
    if all.len() > opts.limit {
        all.select_nth_unstable_by(opts.limit - 1, |a, b| b.cmp(a));
        all.truncate(opts.limit);
    }
    // Total order, so the same query always yields the same list.
    all.sort_unstable_by(|a, b| b.cmp(a));
    all
}

/// Score every directory against every token, once per query.
///
/// Returns a flat `dir_count * token_count` table where `NO_MATCH` means the
/// token is absent from that directory path.
fn score_directories(index: &Index, tokens: &[&str], finders: &[Finder]) -> Vec<i32> {
    let mut table = vec![NO_MATCH; index.dir_count() * tokens.len()];
    for d in 0..index.dir_count() {
        let path = index.dir_folded(d);
        for (t, finder) in finders.iter().enumerate() {
            if let Some(pos) = finder.find(path) {
                table[d * tokens.len() + t] =
                    if pos == 0 || is_word_boundary(path[pos - 1]) {
                        score::DIR_WORD_START
                    } else {
                        score::DIR_SUBSTRING
                    };
            }
        }
    }
    table
}

/// Sentinel for "this token is not in this directory path".
const NO_MATCH: i32 = -1;

/// Score one entry, returning `(tokens matched, score)`.
///
/// `min_matched` of `0` means every token is required, and a missing one bails
/// out immediately — that early exit is what keeps the common strict pass
/// cheap on long queries. Any other value keeps the entry as long as it
/// reaches that many tokens.
///
/// `dir_row` is this entry's slice of the pre-computed directory table.
fn score_entry(
    folded: &[u8],
    tokens: &[&str],
    finders: &[Finder],
    dir_row: &[i32],
    min_matched: usize,
) -> Option<(u16, i32)> {
    let require_all = min_matched == 0;
    let mut total = 0i32;
    let mut matched = 0u16;

    for ((token, finder), &dir_score) in tokens.iter().zip(finders).zip(dir_row) {
        // The filename is always preferred; the folder path is the fallback.
        // Every directory score sits below every filename score, so a file
        // actually named for the query can never be pushed down by one that
        // merely lives in a folder named for it.
        match score_token(folded, token.as_bytes(), finder) {
            Some(s) => {
                total += s;
                matched += 1;
            }
            None if dir_score != NO_MATCH => {
                total += dir_score;
                matched += 1;
            }
            None if require_all => return None,
            None => {}
        }
    }

    let floor = if require_all { tokens.len() } else { min_matched };
    if (matched as usize) < floor.max(1) {
        return None;
    }
    Some((matched, total + length_bonus(folded.len())))
}

fn score_token(folded: &[u8], token: &[u8], finder: &Finder) -> Option<i32> {
    let pos = finder.find(folded)?;

    if pos == 0 {
        // Compare against the stem so `avatar` scores as an exact match on
        // `avatar.mkv` — users type names, not extensions.
        let stem_len = folded
            .iter()
            .rposition(|&b| b == b'.')
            .unwrap_or(folded.len());
        return Some(if token.len() == stem_len || token.len() == folded.len() {
            score::EXACT
        } else {
            score::PREFIX
        });
    }

    if is_word_boundary(folded[pos - 1]) {
        return Some(score::WORD_START);
    }
    Some(score::SUBSTRING)
}

/// Favour shorter names: `Avatar.mkv` beats `my_avatar_backup_2019.mkv`.
fn length_bonus(len: usize) -> i32 {
    score::MAX_LENGTH_BONUS.saturating_sub((len / 4) as i32).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::model::IndexBuilder;

    fn build(names: &[&str]) -> Index {
        let mut b = IndexBuilder::new();
        let d = b.add_dir(r"D:\Media");
        for n in names {
            b.add_file(n, MediaKind::Video, d);
        }
        b.finish()
    }

    fn run(index: &Index, query: &str) -> Vec<String> {
        let cancel = AtomicU64::new(0);
        search(index, query, &SearchOptions::default(), &cancel, 0)
            .hits
            .into_iter()
            .map(|h| index.name(h.index as usize).to_string())
            .collect()
    }

    #[test]
    fn finds_a_plain_substring() {
        let ix = build(&["holiday.mp4", "work.mp4", "beach holiday.mp4"]);
        let got = run(&ix, "holiday");
        assert_eq!(got.len(), 2);
        assert!(got.contains(&"holiday.mp4".to_string()));
    }

    #[test]
    fn ranks_exact_above_prefix_above_embedded() {
        let ix = build(&[
            "my_avatar_backup_2019.mkv",
            "avatar_extended_edition.mkv",
            "avatar.mkv",
        ]);
        // The acceptance criterion from the plan.
        assert_eq!(
            run(&ix, "avatar"),
            vec![
                "avatar.mkv",
                "avatar_extended_edition.mkv",
                "my_avatar_backup_2019.mkv",
            ]
        );
    }

    #[test]
    fn word_start_beats_mid_word() {
        let ix = build(&["xxcatxx.mp4", "my cat.mp4"]);
        assert_eq!(run(&ix, "cat"), vec!["my cat.mp4", "xxcatxx.mp4"]);
    }

    #[test]
    fn shorter_names_win_a_tie() {
        let ix = build(&["cat and a very long descriptive title.mp4", "cat two.mp4"]);
        assert_eq!(run(&ix, "cat")[0], "cat two.mp4");
    }

    #[test]
    fn multiple_tokens_are_anded() {
        let ix = build(&[
            "avatar 2009 1080p.mkv",
            "avatar 2022.mkv",
            "titanic 2009.mkv",
        ]);
        assert_eq!(run(&ix, "avatar 2009"), vec!["avatar 2009 1080p.mkv"]);
    }

    #[test]
    fn tokens_may_appear_in_any_order() {
        let ix = build(&["2009 avatar remastered.mkv"]);
        assert_eq!(run(&ix, "avatar 2009").len(), 1);
    }

    #[test]
    fn matches_vietnamese_typed_without_diacritics() {
        // The reason folding exists at all.
        let ix = build(&["Tiếng Việt.mp4", "English.mp4"]);
        assert_eq!(run(&ix, "tieng viet"), vec!["Tiếng Việt.mp4"]);
        assert_eq!(run(&ix, "TIENG"), vec!["Tiếng Việt.mp4"]);
    }

    #[test]
    fn matches_vietnamese_typed_with_diacritics_too() {
        let ix = build(&["Đà Nẵng 2024.mkv"]);
        assert_eq!(run(&ix, "Đà Nẵng").len(), 1);
        assert_eq!(run(&ix, "da nang").len(), 1);
        assert_eq!(run(&ix, "DA NANG").len(), 1);
    }

    #[test]
    fn finds_files_whose_meaning_lives_in_the_folder_path() {
        // Modelled on the real layout found on D: — descriptive Vietnamese
        // folder names, numeric filenames. Filename-only search returned
        // nothing at all for this library.
        let mut b = IndexBuilder::new();
        let d = b.add_dir(r"D:\Sounds Edit\HƯNG\DATA TẠO VID HƯNG\HAN QUOC\13");
        b.add_file("154.mp3", MediaKind::Audio, d);
        b.add_file("155.mp3", MediaKind::Audio, d);
        let other = b.add_dir(r"D:\Misc");
        b.add_file("999.mp3", MediaKind::Audio, other);
        let ix = b.finish();

        assert_eq!(run(&ix, "han quoc").len(), 2);
        assert_eq!(run(&ix, "hung").len(), 2);
        // Folded, so no diacritics needed on the folder name either.
        assert_eq!(run(&ix, "tao vid").len(), 2);
    }

    #[test]
    fn real_world_vietnamese_directory_names() {
        // Directory names taken verbatim from the scanned D: volume, with the
        // queries typed the way someone actually types them: no diacritics,
        // no matching case. Synthetic test data kept missing this class of
        // problem, so these are pinned to the real thing.
        let mut b = IndexBuilder::new();
        for (path, file) in [
            (r"D:\Sounds Edit\NHẠC GO", "01.mp3"),
            (r"D:\Sounds Edit\Nhạc nền", "02.mp3"),
            (r"D:\Sounds Edit\HƯNG\NHẠC NỀN\Nhạc\Năng Động", "03.mp3"),
            (r"D:\Sounds Edit\HƯNG\WISE\DATA TẠO VID HƯNG\HAN QUOC\13", "154.mp3"),
            (r"D:\Phim\Không dấu gì cả", "04.mp4"),
        ] {
            let d = b.add_dir(path);
            b.add_file(file, MediaKind::Audio, d);
        }
        let ix = b.finish();

        assert_eq!(run(&ix, "nhac").len(), 3, "NHẠC GO / Nhạc nền / NHẠC NỀN");
        assert_eq!(run(&ix, "nhac nen").len(), 2, "Nhạc nền / NHẠC NỀN");
        assert_eq!(run(&ix, "nang dong").len(), 1, "Năng Động");
        assert_eq!(run(&ix, "hung").len(), 2, "HƯNG appears in two paths");
        assert_eq!(run(&ix, "tao vid").len(), 1, "DATA TẠO VID HƯNG");
        assert_eq!(run(&ix, "han quoc").len(), 1);
        assert_eq!(run(&ix, "khong dau").len(), 1);

        // And nothing that is genuinely absent may be conjured up. An
        // independent filesystem check confirmed no `Tiếng Việt` folder
        // exists on that volume, so zero results there is correct behaviour.
        assert!(run(&ix, "tieng viet").is_empty());
    }

    #[test]
    fn a_filename_match_always_outranks_a_folder_match() {
        let mut b = IndexBuilder::new();
        let in_folder = b.add_dir(r"D:\holiday videos");
        b.add_file("00123.mp4", MediaKind::Video, in_folder);
        let plain = b.add_dir(r"D:\Misc");
        b.add_file("holiday.mp4", MediaKind::Video, plain);
        let ix = b.finish();

        // The file actually named `holiday` must come first, however
        // suggestive the other one's folder is.
        assert_eq!(run(&ix, "holiday"), vec!["holiday.mp4", "00123.mp4"]);
    }

    #[test]
    fn tokens_may_be_split_across_folder_and_filename() {
        let mut b = IndexBuilder::new();
        let d = b.add_dir(r"D:\Phim\2024");
        b.add_file("avatar.mkv", MediaKind::Video, d);
        let e = b.add_dir(r"D:\Phim\2019");
        b.add_file("avatar.mkv", MediaKind::Video, e);
        let ix = b.finish();

        // `2024` comes from the folder, `avatar` from the filename.
        let hits = run(&ix, "avatar 2024");
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn a_token_in_neither_name_nor_folder_still_rejects() {
        let mut b = IndexBuilder::new();
        let d = b.add_dir(r"D:\Phim\2024");
        b.add_file("avatar.mkv", MediaKind::Video, d);
        let ix = b.finish();

        assert!(run(&ix, "avatar 1999").is_empty());
        assert!(run(&ix, "avatar nonexistent").is_empty());
    }

    /// The exact filename reported from the user's D: volume.
    const ANGLERFISH: &str =
        "YTSave_YouTube_The-anglerfish-The-original-approach-to-_Media_VqPMP9X-89o_001_1080p.mp4";

    #[test]
    fn punctuation_never_swallows_a_word() {
        // `anglerfish:` used to stay glued to its colon and match nothing,
        // silently discarding the most distinctive word in the query.
        assert_eq!(
            tokenize("the anglerfish: the original approach"),
            vec!["the", "anglerfish", "the", "original", "approach"]
        );
        // A hyphenated pair becomes two tokens, so it matches `deep_sea`,
        // `deep sea` and `Deep-Sea` alike.
        assert_eq!(tokenize("deep-sea fishing"), vec!["deep", "sea", "fishing"]);
        assert_eq!(tokenize("s01e02 (1080p) [x265]"), vec!["s01e02", "1080p", "x265"]);
        assert!(tokenize("   ...   ").is_empty());
    }

    #[test]
    fn tokenizer_keeps_non_latin_words_whole() {
        // `is_alphanumeric` is Unicode-aware, so scripts without spaces or
        // case are not shredded into single characters.
        assert_eq!(tokenize("日本語 の 動画"), vec!["日本語", "の", "動画"]);
        assert_eq!(tokenize("tieng viet"), vec!["tieng", "viet"]);
    }

    #[test]
    fn a_pasted_title_finds_the_truncated_file() {
        // The reported failure, end to end. The downloader cut the title at
        // `to-`, so `deep`, `sea` and `fishing` are not in the filename at
        // all; demanding every token returns nothing while the file sits
        // right there.
        let ix = build(&[ANGLERFISH, "unrelated clip.mp4"]);
        let cancel = AtomicU64::new(0);
        let out = search(
            &ix,
            "The anglerfish: The original approach to deep-sea fishing",
            &SearchOptions::default(),
            &cancel,
            0,
        );

        assert_eq!(out.hits.len(), 1, "the file must be found");
        assert_eq!(ix.name(out.hits[0].index as usize), ANGLERFISH);

        let relaxed = out.relaxed.expect("results are partial and must say so");
        assert_eq!(relaxed.total_tokens, 9);
        assert_eq!(relaxed.best_matched, 6, "the, anglerfish, the, original, approach, to");
    }

    #[test]
    fn relaxed_results_are_ranked_by_how_much_of_the_query_they_match() {
        let ix = build(&[
            "anglerfish only.mp4",                  // 1 of 5 tokens
            "the anglerfish original approach.mp4", // 4 of 5
            "the anglerfish original.mp4",          // 3 of 5
        ]);
        let cancel = AtomicU64::new(0);
        let out = search(
            &ix,
            "the anglerfish original approach missingword",
            &SearchOptions::default(),
            &cancel,
            0,
        );

        let names: Vec<&str> = out.hits.iter().map(|h| ix.name(h.index as usize)).collect();
        assert_eq!(
            names,
            vec!["the anglerfish original approach.mp4"],
            "only the closest match survives, not merely close ones"
        );
        assert_eq!(out.hits[0].matched, 4);
        assert_eq!(out.relaxed.unwrap().best_matched, 4);
    }

    #[test]
    fn every_file_tied_for_the_best_match_is_kept() {
        let ix = build(&[
            "the anglerfish original A.mp4", // the/anglerfish/original = 3 of 5
            "the anglerfish original B.mp4", // 3 of 5
            "the anglerfish.mp4",            // 2 of 5 — below the floor, dropped
        ]);
        let cancel = AtomicU64::new(0);
        let out = search(
            &ix,
            "the anglerfish original approach missingword",
            &SearchOptions::default(),
            &cancel,
            0,
        );
        assert_eq!(out.hits.len(), 2, "ties at the best count are all kept");
        assert!(out.hits.iter().all(|h| h.matched == 3));
    }

    #[test]
    fn relaxing_still_requires_half_the_query() {
        // Without a floor, a nine-word query would return every file that
        // happens to contain the word "the" — thousands of results, all
        // useless, burying the one good answer.
        let ix = build(&[
            "the.mp4",                              // 1 of 5 — must be dropped
            "the anglerfish original.mp4",          // 3 of 5 — must be kept
        ]);
        let cancel = AtomicU64::new(0);
        let out = search(
            &ix,
            "the anglerfish original approach missingword",
            &SearchOptions::default(),
            &cancel,
            0,
        );

        assert_eq!(out.hits.len(), 1, "the 1-of-5 match must not survive");
        assert_eq!(ix.name(out.hits[0].index as usize), "the anglerfish original.mp4");
    }

    #[test]
    fn short_queries_are_never_relaxed() {
        // Two words is a deliberate query. Quietly widening it would answer a
        // question the user did not ask; an honest empty result is better.
        let ix = build(&["avatar 2024.mkv"]);
        let cancel = AtomicU64::new(0);
        for q in ["avatar 1999", "avatar nonexistent"] {
            let out = search(&ix, q, &SearchOptions::default(), &cancel, 0);
            assert!(out.hits.is_empty(), "{q:?} should return nothing");
            assert!(out.relaxed.is_none());
        }
    }

    #[test]
    fn a_query_that_fully_matches_never_falls_back() {
        // The fallback must not loosen a query that already works — `avatar
        // 2009` should not start dragging in every file containing `2009`.
        let ix = build(&["avatar 2009.mkv", "titanic 2009.mkv", "avatar 2022.mkv"]);
        let cancel = AtomicU64::new(0);
        let out = search(&ix, "avatar 2009", &SearchOptions::default(), &cancel, 0);

        assert_eq!(out.hits.len(), 1);
        assert!(out.relaxed.is_none(), "strict results must not be marked partial");
    }

    #[test]
    fn a_single_token_that_matches_nothing_stays_empty() {
        // With one token there is nothing to relax: reporting a partial match
        // would be meaningless.
        let ix = build(&["holiday.mp4"]);
        let cancel = AtomicU64::new(0);
        let out = search(&ix, "zzzznothing", &SearchOptions::default(), &cancel, 0);
        assert!(out.hits.is_empty());
        assert!(out.relaxed.is_none());
    }

    #[test]
    fn relaxed_still_returns_nothing_when_no_token_matches_at_all() {
        let ix = build(&["holiday.mp4"]);
        let cancel = AtomicU64::new(0);
        let out = search(&ix, "zzzz qqqq wwww", &SearchOptions::default(), &cancel, 0);
        assert!(out.hits.is_empty());
        assert!(out.relaxed.is_none());
    }

    #[test]
    fn matched_count_outranks_a_better_quality_match() {
        // An exact match on one token must still lose to a file containing two
        // of them, otherwise a lucky filename hijacks the top of the list.
        let ix = build(&["anglerfish.mp4", "the anglerfish deep clip.mp4"]);
        let cancel = AtomicU64::new(0);
        let out = search(
            &ix,
            "anglerfish deep missingword",
            &SearchOptions::default(),
            &cancel,
            0,
        );
        assert_eq!(
            ix.name(out.hits[0].index as usize),
            "the anglerfish deep clip.mp4"
        );
    }

    #[test]
    fn empty_or_whitespace_query_returns_nothing() {
        let ix = build(&["a.mp4"]);
        assert!(run(&ix, "").is_empty());
        assert!(run(&ix, "   ").is_empty());
    }

    #[test]
    fn respects_the_kind_filter() {
        let mut b = IndexBuilder::new();
        let d = b.add_dir(r"D:\M");
        b.add_file("holiday.mp4", MediaKind::Video, d);
        b.add_file("holiday.jpg", MediaKind::Image, d);
        b.add_file("holiday.mp3", MediaKind::Audio, d);
        let ix = b.finish();

        let cancel = AtomicU64::new(0);
        let opts = SearchOptions {
            kinds: vec![MediaKind::Image],
            ..Default::default()
        };
        let hits = search(&ix, "holiday", &opts, &cancel, 0).hits;
        assert_eq!(hits.len(), 1);
        assert_eq!(ix.name(hits[0].index as usize), "holiday.jpg");
    }

    #[test]
    fn honours_the_limit() {
        let names: Vec<String> = (0..500).map(|i| format!("clip{i}.mp4")).collect();
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let ix = build(&refs);

        let cancel = AtomicU64::new(0);
        let opts = SearchOptions {
            limit: 10,
            ..Default::default()
        };
        assert_eq!(search(&ix, "clip", &opts, &cancel, 0).hits.len(), 10);
    }

    #[test]
    fn a_superseded_search_bails_out() {
        let ix = build(&["a.mp4", "b.mp4"]);
        let cancel = AtomicU64::new(7); // a newer keystroke already arrived
        assert!(search(&ix, "mp4", &SearchOptions::default(), &cancel, 3).hits.is_empty());
    }

    #[test]
    fn results_are_identical_across_runs() {
        // Guards the determinism bug: with unordered parallel selection the
        // same query returns a different list each time.
        let names: Vec<String> = (0..3000).map(|i| format!("clip {i} test.mp4")).collect();
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let ix = build(&refs);

        let cancel = AtomicU64::new(0);
        let opts = SearchOptions {
            limit: 50,
            ..Default::default()
        };
        let first = search(&ix, "clip", &opts, &cancel, 0).hits;
        for _ in 0..12 {
            assert_eq!(
                search(&ix, "clip", &opts, &cancel, 0).hits,
                first,
                "search is not deterministic"
            );
        }
    }

    #[test]
    fn scores_never_descend_through_the_result_list() {
        let names: Vec<String> = (0..2000)
            .map(|i| {
                if i % 3 == 0 {
                    format!("cat{i}.mp4")
                } else {
                    format!("a_long_prefix_{i}_cat_suffix.mp4")
                }
            })
            .collect();
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let ix = build(&refs);

        let cancel = AtomicU64::new(0);
        let hits = search(&ix, "cat", &SearchOptions::default(), &cancel, 0).hits;
        assert!(hits.len() > 100);
        for w in hits.windows(2) {
            assert!(w[0].score >= w[1].score, "results are out of order");
        }
    }

    #[test]
    fn spans_many_chunks_correctly() {
        // More entries than CHUNK, so the parallel merge is actually exercised
        // and nothing is dropped at a chunk boundary.
        let names: Vec<String> = (0..CHUNK * 2 + 7)
            .map(|i| format!("file{i}.mp4"))
            .collect();
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let ix = build(&refs);

        let cancel = AtomicU64::new(0);
        let opts = SearchOptions {
            limit: usize::MAX >> 1,
            ..Default::default()
        };
        // Exactly one entry is named `file9999.mp4`.
        let hits = search(&ix, "file9999.mp4", &opts, &cancel, 0).hits;
        assert_eq!(hits.len(), 1);
        assert_eq!(ix.name(hits[0].index as usize), "file9999.mp4");
    }

    #[test]
    fn query_folding_matches_index_folding() {
        // Both sides must go through the same fold, or diacritics silently
        // stop matching.
        let ix = build(&["Đường Về Nhà.mp4"]);
        for q in ["duong ve nha", "Đường Về Nhà", "DUONG", "đường"] {
            assert_eq!(run(&ix, q).len(), 1, "query {q:?} failed to match");
        }
    }
}
