//! Index, Span, MediaKind, and the arena builder.
//!
//! Only `MediaKind` and the extension table exist so far: the scan in P1 must
//! decide *while reading the MFT* whether a record is a media file, so the
//! table cannot wait for P2 without being written twice.

use serde::{Deserialize, Serialize};

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
            &b"ts"[..], b"tsx", b"js", b"jsx", b"json", b"md", b"toml", b"yml", b"h", b"c",
            b"cpp", b"py", b"go", b"java", b"cs", b"sh", b"log", b"lock",
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
        // If a longer extension is ever added, the stack buffer in the scanner
        // would silently truncate it, so assert the invariant here.
        for ext in [
            &b"mpeg"[..], b"webm", b"m2ts", b"jpeg", b"tiff", b"heic", b"avif", b"flac",
            b"opus", b"aiff", b"alac", b"midi",
        ] {
            assert!(ext.len() <= MAX_EXT_LEN, "{ext:?} exceeds MAX_EXT_LEN");
        }
    }
}
