//! Case- and diacritic-insensitive folding for search.
//!
//! Searching must work the way people actually type. Nobody reaches for the
//! Vietnamese keyboard layout mid-search: they type `tieng viet` and expect
//! `Tiếng Việt.mp4`. Every filename is therefore folded once at index time and
//! every query folded the same way, so matching is a plain substring test on
//! two strings that have already been reduced to the same shape.
//!
//! The fold is: NFD decomposition → drop combining marks → map the letters
//! that refuse to decompose → lowercase.
//!
//! The decomposition step handles most of Vietnamese for free, because NFD
//! splits a precomposed letter into its base plus marks:
//!
//! ```text
//!   ế  U+1EBF  ->  e + U+0302 (circumflex) + U+0301 (acute)  ->  e
//!   ự  U+1EF1  ->  u + U+031B (horn)       + U+0323 (dot)    ->  u
//!   ơ  U+01A1  ->  o + U+031B (horn)                         ->  o
//! ```
//!
//! **`đ` is the exception, and it is the one that gets missed.** U+0111 has no
//! decomposition at all — the stroke is part of the glyph, not a combining
//! mark — so NFD leaves it untouched and it must be mapped by hand. Without
//! that single line, `da nang` fails to match `Đà Nẵng`.

use unicode_normalization::UnicodeNormalization;

/// Fold `s` for searching.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    fold_into(s, &mut out);
    out
}

/// Fold `s`, appending to `out`.
///
/// Reusing one buffer across a whole index build avoids an allocation per file.
pub fn fold_into(s: &str, out: &mut String) {
    // Most filenames are pure ASCII, and for those the whole normalisation
    // machinery is wasted work. Worth checking first: this path runs once per
    // file over hundreds of thousands of files.
    if s.is_ascii() {
        out.extend(s.chars().map(|c| c.to_ascii_lowercase()));
        return;
    }

    // Decompose, drop the marks, then compose again.
    //
    // The final NFC pass is not cosmetic. NFD splits far more than Latin
    // diacritics — a Hangul syllable such as `한` becomes three separate Jamo,
    // none of which are combining marks, so they survive the filter. Leaving
    // the result decomposed would triple the arena cost of every CJK filename
    // and hand back text that no longer matches what the user sees. Composing
    // again restores `한`, while `ế` stays `e` because its marks are gone and
    // there is nothing left to recombine.
    let stripped: String = s
        .nfd()
        .filter(|&c| !is_combining_mark(c))
        .flat_map(|c| {
            let mapped = undecomposable(c);
            LowerOrMapped::new(c, mapped)
        })
        .collect();
    out.extend(stripped.nfc());
}

/// Yields either a fixed replacement string or the lowercase form of a char.
///
/// A small hand-rolled iterator so the fold stays a single pass without
/// boxing a trait object per character.
enum LowerOrMapped {
    Mapped(std::str::Chars<'static>),
    Lower(std::char::ToLowercase),
}

impl LowerOrMapped {
    fn new(c: char, mapped: Option<&'static str>) -> Self {
        match mapped {
            Some(m) => LowerOrMapped::Mapped(m.chars()),
            None => LowerOrMapped::Lower(c.to_lowercase()),
        }
    }
}

impl Iterator for LowerOrMapped {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self {
            LowerOrMapped::Mapped(it) => it.next(),
            LowerOrMapped::Lower(it) => it.next(),
        }
    }
}

/// Letters with no NFD decomposition, whose diacritic is part of the glyph.
///
/// `đ`/`Đ` is the one that matters here — it is a distinct Vietnamese letter,
/// and leaving it unmapped breaks every search for a word containing it. The
/// rest cost nothing and make folding behave sensibly for European filenames.
fn undecomposable(c: char) -> Option<&'static str> {
    Some(match c {
        'đ' | 'Đ' => "d",
        'ø' | 'Ø' => "o",
        'ł' | 'Ł' => "l",
        'æ' | 'Æ' => "ae",
        'œ' | 'Œ' => "oe",
        'ß' => "ss",
        'ð' | 'Ð' => "d",
        'þ' | 'Þ' => "th",
        _ => return None,
    })
}

/// Combining marks, which NFD has just separated from their base letter.
fn is_combining_mark(c: char) -> bool {
    matches!(c as u32,
        0x0300..=0x036F   // Combining Diacritical Marks — all of Vietnamese
        | 0x1AB0..=0x1AFF // …Extended
        | 0x1DC0..=0x1DFF // …Supplement
        | 0x20D0..=0x20FF // …for Symbols
        | 0xFE20..=0xFE2F // Combining Half Marks
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_headline_case() {
        // If this ever breaks, the app is useless to a Vietnamese speaker.
        assert_eq!(fold("Tiếng Việt Đà Nẵng"), "tieng viet da nang");
    }

    #[test]
    fn d_with_stroke_must_be_mapped_by_hand() {
        // NFD does not decompose U+0111 / U+0110, so this is the one letter
        // that normalisation alone will not fix.
        assert_eq!(fold("đ"), "d");
        assert_eq!(fold("Đ"), "d");
        assert_eq!(fold("Đường Đi Đẹp"), "duong di dep");
    }

    #[test]
    fn horned_vowels_decompose() {
        // ơ and ư carry U+031B, which NFD does split off.
        assert_eq!(fold("ơ ư Ơ Ư"), "o u o u");
        assert_eq!(fold("Được"), "duoc");
        assert_eq!(fold("Tương Lai"), "tuong lai");
    }

    #[test]
    fn stacked_marks_are_all_removed() {
        // Vietnamese stacks a vowel-quality mark and a tone mark on one letter.
        assert_eq!(fold("ế ề ể ễ ệ"), "e e e e e");
        assert_eq!(fold("ự ừ ứ ử ữ"), "u u u u u");
        assert_eq!(fold("ằ ắ ẳ ẵ ặ"), "a a a a a");
        assert_eq!(fold("ỗ ộ ố ồ ổ"), "o o o o o");
    }

    #[test]
    fn every_vietnamese_letter_folds_to_ascii() {
        let alphabet = "aăâbcdđeêghiklmnoôơpqrstuưvxy";
        let folded = fold(alphabet);
        assert!(
            folded.is_ascii(),
            "folded alphabet still has non-ASCII: {folded:?}"
        );
        assert_eq!(folded, "aaabcddeeghiklmnooopqrstuuvxy");
    }

    #[test]
    fn all_tone_marks_on_a_single_vowel() {
        // The six Vietnamese tones on `a`: level, grave, hook, tilde, acute, dot.
        assert_eq!(fold("a à ả ã á ạ"), "a a a a a a");
    }

    #[test]
    fn realistic_filenames() {
        assert_eq!(
            fold("Phim Tài Liệu - Vịnh Hạ Long (2024).mp4"),
            "phim tai lieu - vinh ha long (2024).mp4"
        );
        assert_eq!(
            fold("Bài 13_ UROLOGIST_ What Is The Normal Size.mp3"),
            "bai 13_ urologist_ what is the normal size.mp3"
        );
    }

    #[test]
    fn ascii_fast_path_matches_the_general_path() {
        // The ASCII shortcut must not diverge from the full implementation.
        for s in [
            "Hello World.MP4",
            "S01E02-1080p.MKV",
            "",
            "12345",
            "A_b-c.d",
        ] {
            let mut general = String::new();
            for c in s.nfd() {
                if !is_combining_mark(c) {
                    general.extend(c.to_lowercase());
                }
            }
            assert_eq!(fold(s), general, "diverged on {s:?}");
        }
    }

    #[test]
    fn leaves_scripts_without_case_or_marks_alone() {
        // Folding must not mangle names it has nothing to say about.
        assert_eq!(fold("日本語.mp4"), "日本語.mp4");
        assert_eq!(fold("한국어"), "한국어");
    }

    #[test]
    fn european_letters_that_do_not_decompose() {
        assert_eq!(fold("Straße"), "strasse");
        assert_eq!(fold("Bjørn"), "bjorn");
        assert_eq!(fold("Łódź"), "lodz");
    }

    #[test]
    fn handles_empty_and_marks_only_input() {
        assert_eq!(fold(""), "");
        assert_eq!(
            fold("\u{0301}\u{0323}"),
            "",
            "bare marks fold away entirely"
        );
    }

    #[test]
    fn is_idempotent() {
        // Folding an already-folded string must change nothing, or query
        // folding could disagree with index folding.
        for s in ["Tiếng Việt", "Đà Nẵng", "Straße", "日本語", "Hello.MP4"] {
            let once = fold(s);
            assert_eq!(fold(&once), once, "not idempotent for {s:?}");
        }
    }
}
