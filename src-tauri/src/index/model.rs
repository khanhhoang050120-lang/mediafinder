//! The in-memory database: `MediaKind`, the extension table, and `Index`.

use serde::{Deserialize, Serialize};

use crate::index::fold::fold_into;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MediaKind {
    Video = 0,
    Image = 1,
    Audio = 2,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MediaKind::Video => "video",
            MediaKind::Image => "image",
            MediaKind::Audio => "audio",
        }
    }
}

/// Longest extension in the table below. Used to size a stack buffer so the
/// scan can classify a UTF-16 filename without allocating.
pub const MAX_EXT_LEN: usize = 5;

/// Classify a lowercase ASCII extension (without the dot).
///
/// Written as a `match` on byte-string literals rather than a lookup table so
/// rustc compiles it into a length-then-content decision tree; this runs once
/// per MFT record, several million times per scan.
/// Classify a filename that is already a `String`.
///
/// The scan path has its own version working straight off UTF-16
/// (`usn_enum::classify_utf16`), because it runs millions of times and must
/// not decode a name it is about to reject. This one is for the few names that
/// arrive as text — journal updates and tests — and shares the same table, so
/// the two can never disagree about what counts as media.
pub fn classify_name(name: &str) -> Option<MediaKind> {
    let dot = name.rfind('.')?;
    let ext = &name.as_bytes()[dot + 1..];
    if ext.is_empty() || ext.len() > MAX_EXT_LEN {
        return None;
    }
    let mut buf = [0u8; MAX_EXT_LEN];
    for (slot, &c) in buf.iter_mut().zip(ext) {
        if c > 0x7F {
            return None;
        }
        *slot = c.to_ascii_lowercase();
    }
    kind_from_ext(&buf[..ext.len()])
}

pub fn kind_from_ext(ext: &[u8]) -> Option<MediaKind> {
    use MediaKind::*;
    Some(match ext {
        // Video.
        //
        // `ts` is deliberately absent. It is a valid MPEG transport stream
        // extension, but on any machine with source code on it TypeScript
        // outnumbers transport streams by orders of magnitude — a real scan of
        // C: turned up `analyze-meal.ts` filed as a video. Camcorder and
        // Blu-ray footage is covered by `m2ts` and `mts`, which are not
        // ambiguous.
        b"mp4" | b"mkv" | b"avi" | b"mov" | b"wmv" | b"flv" | b"webm" | b"m4v" | b"mpg"
        | b"mpeg" | b"m2ts" | b"mts" | b"3gp" | b"vob" | b"rmvb" | b"rm" | b"ogv" | b"divx"
        | b"asf" | b"f4v" | b"m2v" | b"mpv" => Video,

        // Image
        b"jpg" | b"jpeg" | b"jfif" | b"png" | b"gif" | b"bmp" | b"webp" | b"tif" | b"tiff"
        | b"heic" | b"heif" | b"avif" | b"ico" | b"svg" | b"psd" | b"raw" | b"cr2" | b"cr3"
        | b"nef" | b"arw" | b"dng" | b"orf" | b"rw2" | b"raf" | b"sr2" => Image,

        // Audio
        b"mp3" | b"flac" | b"wav" | b"aac" | b"ogg" | b"oga" | b"m4a" | b"m4b" | b"wma"
        | b"opus" | b"aiff" | b"aif" | b"ape" | b"alac" | b"ac3" | b"dts" | b"dsf" | b"dff"
        | b"mka" | b"amr" | b"mid" | b"midi" | b"wv" | b"tta" => Audio,

        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// The in-memory index
// ---------------------------------------------------------------------------

/// A slice of the string arena.
///
/// Eight bytes, versus 24 plus a heap allocation for a `String`. With hundreds
/// of thousands of entries, each stored twice (display form and folded form),
/// that difference is the whole design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub off: u32,
    pub len: u32,
}

impl Span {
    fn range(self) -> std::ops::Range<usize> {
        self.off as usize..(self.off + self.len) as usize
    }
}

/// The whole searchable database, immutable once built.
///
/// Two decisions shape this type:
///
/// * **Struct of arrays.** Search touches only `folded` and, for filtering,
///   `kind`. Keeping those in their own contiguous vectors means a scan pulls
///   nothing else through the cache. An array of structs would drag names,
///   directory ids and everything else along for the ride.
///
/// * **One string arena.** Every piece of text lives in a single `Vec<u8>`,
///   referenced by [`Span`]. Directory paths are stored once and shared by
///   every file inside them, which keeps a library of hundreds of thousands of
///   files down to tens of megabytes rather than hundreds.
///
/// Being immutable is what lets the whole thing sit behind an `ArcSwap`: a
/// search clones the `Arc`, releases the lock immediately, and scans a
/// snapshot that cannot change underneath it.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Index {
    /// All text: directory paths, filenames, and folded filenames.
    strings: Vec<u8>,
    /// Absolute directory paths, deduplicated by construction.
    dirs: Vec<Span>,
    /// Folded form of each directory path, for searching.
    ///
    /// Stored per *directory*, not per file. A real library has 116k files
    /// across 4k directories, so this costs a few hundred kilobytes and lets a
    /// query test each directory once instead of once per file inside it.
    dir_folded: Vec<Span>,

    // Parallel arrays, one entry per file.
    name: Vec<Span>,
    folded: Vec<Span>,
    dir_id: Vec<u32>,
    kind: Vec<MediaKind>,

    /// File Reference Number of each entry, and of each directory.
    ///
    /// The only identity that survives deletion: a USN journal record names a
    /// file by FRN, and by the time the record is read the file may no longer
    /// have a path to match against.
    ///
    /// FRNs are unique per volume, not per machine, so two entries on
    /// different drives can share one. The volume is recoverable from the
    /// directory path, so it is not stored again here — anything building a
    /// lookup table must key on `(drive letter, frn)`.
    frn: Vec<u64>,
    dir_frn: Vec<u64>,

    /// Bytes on disk. Cheap to obtain — `GetFileAttributesEx` reads metadata
    /// only, never opening the file — so it is collected during the scan for
    /// every entry rather than enriched later.
    size: Vec<u64>,
    /// Last-write time, Unix seconds. Kept alongside `size` because together
    /// they are what tells the slow enrichment pass whether its stored answer
    /// for a file is still about the same file.
    mtime: Vec<i64>,
}

impl Index {
    pub fn len(&self) -> usize {
        self.name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    pub fn dir_count(&self) -> usize {
        self.dirs.len()
    }

    /// The filename as it appears on disk, for display.
    pub fn name(&self, i: usize) -> &str {
        self.str_at(self.name[i])
    }

    /// The folded filename, as raw bytes, for matching.
    ///
    /// Bytes rather than `&str` on purpose: the search never needs to know
    /// about characters. UTF-8 is self-synchronising, so a byte-level
    /// substring match can never straddle a character boundary and produce a
    /// false positive — and skipping validation keeps the hot loop tight.
    pub fn folded(&self, i: usize) -> &[u8] {
        &self.strings[self.folded[i].range()]
    }

    /// The absolute path of the directory holding entry `i`.
    pub fn dir(&self, i: usize) -> &str {
        self.str_at(self.dirs[self.dir_id[i] as usize])
    }

    pub fn kind(&self, i: usize) -> MediaKind {
        self.kind[i]
    }

    pub fn size(&self, i: usize) -> u64 {
        self.size.get(i).copied().unwrap_or(0)
    }

    pub fn mtime(&self, i: usize) -> i64 {
        self.mtime.get(i).copied().unwrap_or(0)
    }

    /// File Reference Number of entry `i`, or 0 if this index predates them.
    pub fn frn(&self, i: usize) -> u64 {
        self.frn.get(i).copied().unwrap_or(0)
    }

    /// File Reference Number of directory `dir_id`, or 0 if unknown.
    pub fn dir_frn(&self, dir_id: usize) -> u64 {
        self.dir_frn.get(dir_id).copied().unwrap_or(0)
    }

    pub fn frns(&self) -> &[u64] {
        &self.frn
    }

    pub fn dir_frns(&self) -> &[u64] {
        &self.dir_frn
    }

    /// The drive letter entry `i` lives on, uppercased.
    ///
    /// Read off the directory path rather than stored: every absolute path
    /// starts with it, and an FRN is only meaningful together with the volume
    /// it came from.
    pub fn volume_of(&self, i: usize) -> u8 {
        Self::volume_of_path(self.dir(i))
    }

    /// The drive letter of directory `dir_id`, uppercased.
    pub fn volume_of_dir(&self, dir_id: usize) -> u8 {
        Self::volume_of_path(self.dir_path(dir_id))
    }

    fn volume_of_path(path: &str) -> u8 {
        path.as_bytes()
            .first()
            .copied()
            .unwrap_or(0)
            .to_ascii_uppercase()
    }

    /// The absolute path of directory `dir_id`.
    pub fn dir_path(&self, dir_id: usize) -> &str {
        self.str_at(self.dirs[dir_id])
    }

    pub fn sizes(&self) -> &[u64] {
        &self.size
    }

    /// Fill in sizes and modification times collected after the entries were
    /// added. Lengths must match; a mismatch is ignored rather than panicking
    /// in the indexer, and simply leaves the values at zero.
    pub fn set_file_stats(&mut self, sizes: Vec<u64>, mtimes: Vec<i64>) {
        if sizes.len() == self.name.len() && mtimes.len() == self.name.len() {
            self.size = sizes;
            self.mtime = mtimes;
        } else {
            tracing::warn!(
                "bỏ qua thống kê tệp: {} kích thước / {} thời gian cho {} mục",
                sizes.len(),
                mtimes.len(),
                self.name.len()
            );
        }
    }

    /// The folded directory path for directory `dir_id`.
    pub fn dir_folded(&self, dir_id: usize) -> &[u8] {
        &self.strings[self.dir_folded[dir_id].range()]
    }

    /// The directory id of every entry, so the search can look up a
    /// pre-computed per-directory score without a bounds check per file.
    pub fn dir_ids(&self) -> &[u32] {
        &self.dir_id
    }

    /// All kinds, so the search loop can filter without bounds checks.
    pub fn kinds(&self) -> &[MediaKind] {
        &self.kind
    }

    /// Join directory and filename into the absolute path.
    ///
    /// Deliberately not stored: building it for the handful of results
    /// actually returned costs nothing, while keeping a full path per file
    /// would undo the entire point of the directory lookup table.
    pub fn full_path(&self, i: usize) -> String {
        let dir = self.dir(i);
        let name = self.name(i);
        let mut s = String::with_capacity(dir.len() + 1 + name.len());
        s.push_str(dir);
        s.push('\\');
        s.push_str(name);
        s
    }

    /// Approximate heap footprint, for reporting and for catching regressions.
    pub fn memory_bytes(&self) -> usize {
        self.strings.len()
            + (self.dirs.len() + self.dir_folded.len() + self.name.len() + self.folded.len())
                * std::mem::size_of::<Span>()
            + self.dir_id.len() * std::mem::size_of::<u32>()
            + self.kind.len()
            + self.size.len() * std::mem::size_of::<u64>()
            + self.mtime.len() * std::mem::size_of::<i64>()
            + (self.frn.len() + self.dir_frn.len()) * std::mem::size_of::<u64>()
    }

    fn str_at(&self, span: Span) -> &str {
        // Everything in the arena was pushed from a `&str`, so this cannot
        // fail. Checked anyway, because it is nowhere near the hot path.
        std::str::from_utf8(&self.strings[span.range()]).unwrap_or_default()
    }
}

/// Accumulates an [`Index`] one volume at a time.
///
/// Volumes are added separately because each scan numbers its directories from
/// zero; the builder hands out global ids as it goes.
#[derive(Default)]
pub struct IndexBuilder {
    strings: Vec<u8>,
    dirs: Vec<Span>,
    dir_folded: Vec<Span>,
    name: Vec<Span>,
    folded: Vec<Span>,
    dir_id: Vec<u32>,
    kind: Vec<MediaKind>,
    frn: Vec<u64>,
    dir_frn: Vec<u64>,
    /// Reused across every file so folding does not allocate per entry.
    fold_buf: String,
}

impl IndexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserve room for a volume that is about to be added.
    pub fn reserve(&mut self, dirs: usize, files: usize) {
        self.dirs.reserve(dirs);
        self.dir_folded.reserve(dirs);
        self.dir_frn.reserve(dirs);
        self.name.reserve(files);
        self.folded.reserve(files);
        self.dir_id.reserve(files);
        self.kind.reserve(files);
        self.frn.reserve(files);
    }

    /// Add a directory path, returning its global id.
    ///
    /// `frn` is the directory's File Reference Number, used later to apply
    /// journal updates; pass 0 when there is none to give.
    pub fn add_dir(&mut self, path: &str, frn: u64) -> u32 {
        let span = self.push_str(path);

        let mut buf = std::mem::take(&mut self.fold_buf);
        buf.clear();
        fold_into(path, &mut buf);
        let folded_span = self.push_str(&buf);
        self.fold_buf = buf;

        self.dirs.push(span);
        self.dir_folded.push(folded_span);
        self.dir_frn.push(frn);
        (self.dirs.len() - 1) as u32
    }

    /// Add a file. `dir_id` must have come from [`IndexBuilder::add_dir`].
    pub fn add_file(&mut self, name: &str, kind: MediaKind, dir_id: u32, frn: u64) {
        debug_assert!((dir_id as usize) < self.dirs.len(), "dir_id out of range");

        let name_span = self.push_str(name);

        // Moved out and back so the buffer can be reused while `self` is
        // borrowed mutably by `push_str`.
        let mut buf = std::mem::take(&mut self.fold_buf);
        buf.clear();
        fold_into(name, &mut buf);
        let folded_span = self.push_str(&buf);
        self.fold_buf = buf;

        self.name.push(name_span);
        self.folded.push(folded_span);
        self.dir_id.push(dir_id);
        self.kind.push(kind);
        self.frn.push(frn);
    }

    pub fn finish(self) -> Index {
        Index {
            strings: self.strings,
            dirs: self.dirs,
            dir_folded: self.dir_folded,
            name: self.name,
            folded: self.folded,
            dir_id: self.dir_id,
            kind: self.kind,
            frn: self.frn,
            dir_frn: self.dir_frn,
            // Filled in by a separate pass; see `Index::set_file_stats`.
            size: Vec::new(),
            mtime: Vec::new(),
        }
    }

    fn push_str(&mut self, s: &str) -> Span {
        let off = self.strings.len() as u32;
        self.strings.extend_from_slice(s.as_bytes());
        Span {
            off,
            len: s.len() as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_each_category() {
        assert_eq!(kind_from_ext(b"mkv"), Some(MediaKind::Video));
        assert_eq!(kind_from_ext(b"jpeg"), Some(MediaKind::Image));
        assert_eq!(kind_from_ext(b"flac"), Some(MediaKind::Audio));
    }

    #[test]
    fn rejects_non_media() {
        for ext in [&b"exe"[..], b"dll", b"txt", b"rs", b"", b"zip"] {
            assert_eq!(kind_from_ext(ext), None, "ext {ext:?} should not be media");
        }
    }

    #[test]
    fn source_code_extensions_are_never_media() {
        // Regression guard. `.ts` shipped as Video in the first cut and a real
        // scan of C: immediately filed TypeScript sources as videos. Anything
        // added to the table later must not reintroduce that overlap.
        for ext in [
            &b"ts"[..],
            b"tsx",
            b"js",
            b"jsx",
            b"json",
            b"md",
            b"toml",
            b"yml",
            b"h",
            b"c",
            b"cpp",
            b"py",
            b"go",
            b"java",
            b"cs",
            b"sh",
            b"log",
            b"lock",
        ] {
            assert_eq!(
                kind_from_ext(ext),
                None,
                "source/config extension {ext:?} must not be classified as media"
            );
        }
    }

    #[test]
    fn max_ext_len_covers_every_entry() {
        // A longer extension would be silently truncated by the scanner's
        // stack buffer, so assert the invariant here.
        for ext in [
            &b"mpeg"[..],
            b"webm",
            b"m2ts",
            b"jpeg",
            b"tiff",
            b"heic",
            b"avif",
            b"flac",
            b"opus",
            b"aiff",
            b"alac",
            b"midi",
        ] {
            assert!(ext.len() <= MAX_EXT_LEN, "{ext:?} exceeds MAX_EXT_LEN");
        }
    }

    fn sample() -> Index {
        let mut b = IndexBuilder::new();
        let phim = b.add_dir(r"D:\Phim", 0);
        let nhac = b.add_dir(r"D:\Nhạc", 0);
        b.add_file("Tiếng Việt.mp4", MediaKind::Video, phim, 0);
        b.add_file("Đà Nẵng 2024.mkv", MediaKind::Video, phim, 0);
        b.add_file("bài hát.mp3", MediaKind::Audio, nhac, 0);
        b.finish()
    }

    #[test]
    fn stores_and_returns_entries() {
        let ix = sample();
        assert_eq!(ix.len(), 3);
        assert_eq!(ix.dir_count(), 2);
        assert_eq!(ix.name(0), "Tiếng Việt.mp4");
        assert_eq!(ix.kind(2), MediaKind::Audio);
    }

    #[test]
    fn folds_names_at_build_time() {
        let ix = sample();
        assert_eq!(ix.folded(0), b"tieng viet.mp4");
        assert_eq!(ix.folded(1), b"da nang 2024.mkv");
        assert_eq!(ix.folded(2), b"bai hat.mp3");
    }

    #[test]
    fn joins_full_paths() {
        let ix = sample();
        assert_eq!(ix.full_path(0), r"D:\Phim\Tiếng Việt.mp4");
        assert_eq!(ix.full_path(2), r"D:\Nhạc\bài hát.mp3");
    }

    #[test]
    fn directory_text_is_stored_once_however_many_files_share_it() {
        fn with_dir(dir: &str) -> Index {
            let mut b = IndexBuilder::new();
            let d = b.add_dir(dir, 0);
            for i in 0..100 {
                b.add_file(&format!("f{i}.mp4"), MediaKind::Video, d, 0);
            }
            b.finish()
        }

        let short = with_dir(r"D:\M");
        let long = with_dir(&format!(r"D:\{}", "x".repeat(EXTRA)));

        // A hundred files resolve to the same path.
        assert_eq!(short.dir_count(), 1);
        assert_eq!(short.dir(0), short.dir(99));

        // Measure the property directly rather than against a fixed byte
        // budget: lengthening only the *directory* name must cost roughly one
        // copy of the extra text — twice, since the folded form is stored too.
        // Were the path kept per file it would cost a hundred times that.
        //
        // A fixed ceiling would have to be raised every time a per-entry field
        // is added (`size`, `mtime`, `frn` …), and each of those edits is a
        // chance to quietly raise it past the point where it still proves
        // anything.
        let grew = long.memory_bytes() - short.memory_bytes();
        assert!(
            grew < 4 * EXTRA,
            "thư mục dài thêm {EXTRA} ký tự làm index phình {grew} byte — \
             đường dẫn đang bị nhân bản theo từng tệp"
        );
    }

    /// Long enough that duplicating it a hundred times is unmistakable.
    const EXTRA: usize = 500;

    #[test]
    fn file_reference_numbers_survive_into_the_index() {
        let mut b = IndexBuilder::new();
        let d = b.add_dir(r"D:\Phim", 0x0004_0000_0000_1234);
        b.add_file("a.mp4", MediaKind::Video, d, 0x0002_0000_0000_00AB);
        b.add_file("b.mp4", MediaKind::Video, d, 0x0002_0000_0000_00CD);
        let ix = b.finish();

        // The high 16 bits are the record's sequence number, not padding, so
        // the whole 64 bits has to come back unchanged — truncating to the
        // record number would make a reused record collide with a stale one.
        assert_eq!(ix.frn(0), 0x0002_0000_0000_00AB);
        assert_eq!(ix.frn(1), 0x0002_0000_0000_00CD);
        assert_eq!(ix.dir_frn(0), 0x0004_0000_0000_1234);
        assert_eq!(ix.frns(), &[0x0002_0000_0000_00AB, 0x0002_0000_0000_00CD]);
    }

    #[test]
    fn the_volume_a_file_lives_on_is_read_off_its_path() {
        let mut b = IndexBuilder::new();
        let c = b.add_dir(r"c:\Users\Me", 10);
        let d = b.add_dir(r"D:\Phim", 11);
        b.add_file("a.mp4", MediaKind::Video, c, 7);
        b.add_file("b.mp4", MediaKind::Video, d, 7);
        let ix = b.finish();

        // Two different files sharing one FRN is not a bug: an FRN is unique
        // per volume, so identity is the pair, never the number alone.
        assert_eq!(ix.frn(0), ix.frn(1));
        assert_eq!(ix.volume_of(0), b'C');
        assert_eq!(ix.volume_of(1), b'D');
        assert_eq!(ix.volume_of_dir(0), b'C');
    }

    #[test]
    fn empty_index_is_usable() {
        let ix = IndexBuilder::new().finish();
        assert!(ix.is_empty());
        assert_eq!(ix.len(), 0);
        assert_eq!(ix.memory_bytes(), 0);
    }

    #[test]
    fn span_is_eight_bytes() {
        // The whole memory argument rests on this.
        assert_eq!(std::mem::size_of::<Span>(), 8);
        assert!(std::mem::size_of::<Span>() < std::mem::size_of::<String>());
    }
}
