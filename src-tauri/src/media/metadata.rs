//! File statistics and media properties.
//!
//! Two very different costs live here, and keeping them apart is the whole
//! design:
//!
//! | | How | Per file | 117k files |
//! |---|---|---|---|
//! | Size, modified time | `GetFileAttributesEx` — metadata only | ~20 µs | **~2 s** |
//! | Width, height, duration | `IPropertyStore` — **opens the file** | 5–50 ms | **10–100 min** |
//!
//! The cheap one runs inside the scan, so every entry has it the moment the
//! index loads. The expensive one cannot: adding an hour to a scan the user is
//! watching would be absurd. It runs in the background afterwards, saves as it
//! goes, and filters work on however much of it is done — with the UI saying
//! how much that is.

use std::ffi::c_void;

use serde::{Deserialize, Serialize};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Storage::FileSystem::{
    GetFileAttributesExW, GetFileExInfoStandard, WIN32_FILE_ATTRIBUTE_DATA,
};
use windows::Win32::UI::Shell::PropertiesSystem::{
    IPropertyStore, SHGetPropertyStoreFromParsingName, GPS_READWRITE,
};

/// Size and last-write time, read without opening the file.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FileStats {
    pub size: u64,
    /// Unix seconds.
    pub mtime: i64,
}

/// What the shell knows about a media file's content.
///
/// Zero means "not known" throughout — an image has no duration, an audio file
/// has no dimensions, and a codec Windows cannot read yields nothing at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaProps {
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
}

impl MediaProps {
    /// True when nothing at all could be read, so the caller can distinguish
    /// "asked and got nothing" from "not asked yet".
    pub fn is_empty(&self) -> bool {
        self.width == 0 && self.height == 0 && self.duration_ms == 0
    }
}

/// Read size and modification time.
///
/// `GetFileAttributesEx` answers from the MFT record the scan just walked, so
/// this is fast enough to run over an entire library inside the scan itself.
pub fn file_stats(path: &str) -> Option<FileStats> {
    let wide = HSTRING::from(path);
    let mut data = WIN32_FILE_ATTRIBUTE_DATA::default();

    unsafe {
        GetFileAttributesExW(
            PCWSTR(wide.as_ptr()),
            GetFileExInfoStandard,
            &mut data as *mut _ as *mut c_void,
        )
        .ok()?;
    }

    Some(FileStats {
        size: ((data.nFileSizeHigh as u64) << 32) | data.nFileSizeLow as u64,
        mtime: filetime_to_unix(data.ftLastWriteTime.dwHighDateTime, data.ftLastWriteTime.dwLowDateTime),
    })
}

/// FILETIME (100 ns ticks since 1601) to Unix seconds.
fn filetime_to_unix(high: u32, low: u32) -> i64 {
    const TICKS_TO_UNIX_EPOCH: i64 = 116_444_736_000_000_000;
    const TICKS_PER_SECOND: i64 = 10_000_000;
    let ticks = (((high as u64) << 32) | low as u64) as i64;
    if ticks < TICKS_TO_UNIX_EPOCH {
        return 0;
    }
    (ticks - TICKS_TO_UNIX_EPOCH) / TICKS_PER_SECOND
}

// Property keys, spelled out rather than imported.
//
// windows-rs exposes these as `PKEY_*` constants only when the relevant
// feature set is enabled, and the definitions are simply a GUID and an index.
// Writing them here keeps the dependency surface small and makes it obvious
// which properties are being asked for.
mod pkey {
    use windows::core::GUID;
    use windows::Win32::Foundation::PROPERTYKEY;

    /// `PKEY_Video_FrameWidth` / `FrameHeight` — {64440491-…}
    const VIDEO: GUID = GUID::from_u128(0x64440491_4C8B_11D1_8B70_080036B11A03);
    /// `PKEY_Image_HorizontalSize` / `VerticalSize` — {6444048F-…}
    const IMAGE: GUID = GUID::from_u128(0x6444048F_4C8B_11D1_8B70_080036B11A03);
    /// `PKEY_Media_Duration` — {64440490-…}
    const MEDIA: GUID = GUID::from_u128(0x64440490_4C8B_11D1_8B70_080036B11A03);

    pub const VIDEO_FRAME_WIDTH: PROPERTYKEY = PROPERTYKEY { fmtid: VIDEO, pid: 3 };
    pub const VIDEO_FRAME_HEIGHT: PROPERTYKEY = PROPERTYKEY { fmtid: VIDEO, pid: 4 };
    pub const IMAGE_HORIZONTAL_SIZE: PROPERTYKEY = PROPERTYKEY { fmtid: IMAGE, pid: 3 };
    pub const IMAGE_VERTICAL_SIZE: PROPERTYKEY = PROPERTYKEY { fmtid: IMAGE, pid: 4 };
    pub const MEDIA_DURATION: PROPERTYKEY = PROPERTYKEY { fmtid: MEDIA, pid: 3 };
}

/// Read width, height and duration from the shell property system.
///
/// **This opens the file.** It is the expensive half of this module and must
/// never run on a path the user is waiting on.
///
/// COM must already be initialised on the calling thread.
pub fn media_props(path: &str) -> Option<MediaProps> {
    let wide = HSTRING::from(path);

    unsafe {
        // GPS_READWRITE rather than the read-only mode: the read-only handler
        // is faster but does not consult the codec-provided property handlers
        // that know a video's frame size, so it returns nothing for exactly
        // the files this feature exists for.
        let store: IPropertyStore =
            SHGetPropertyStoreFromParsingName(PCWSTR(wide.as_ptr()), None, GPS_READWRITE).ok()?;

        let mut props = MediaProps {
            width: read_u32(&store, pkey::VIDEO_FRAME_WIDTH)
                .or_else(|| read_u32(&store, pkey::IMAGE_HORIZONTAL_SIZE))
                .unwrap_or(0),
            height: read_u32(&store, pkey::VIDEO_FRAME_HEIGHT)
                .or_else(|| read_u32(&store, pkey::IMAGE_VERTICAL_SIZE))
                .unwrap_or(0),
            duration_ms: 0,
        };

        // Duration is reported in 100 ns units, the same tick as FILETIME.
        if let Some(ticks) = read_u64(&store, pkey::MEDIA_DURATION) {
            props.duration_ms = ticks / 10_000;
        }

        Some(props)
    }
}

unsafe fn read_u32(
    store: &IPropertyStore,
    key: windows::Win32::Foundation::PROPERTYKEY,
) -> Option<u32> {
    let value = store.GetValue(&key).ok()?;
    let n = windows::Win32::System::Variant::VariantToUInt32(&variant_of(&value)).ok()?;
    (n > 0).then_some(n)
}

unsafe fn read_u64(
    store: &IPropertyStore,
    key: windows::Win32::Foundation::PROPERTYKEY,
) -> Option<u64> {
    let value = store.GetValue(&key).ok()?;
    let n = windows::Win32::System::Variant::VariantToUInt64(&variant_of(&value)).ok()?;
    (n > 0).then_some(n)
}

/// A `PROPVARIANT` and a `VARIANT` share a layout for the scalar cases this
/// module reads, which is what lets the `VariantTo*` helpers do the widening
/// and type coercion instead of matching on `vt` by hand.
unsafe fn variant_of(
    p: &windows::Win32::System::Com::StructuredStorage::PROPVARIANT,
) -> windows::Win32::System::Variant::VARIANT {
    std::mem::transmute_copy(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filetime_converts_to_unix_seconds() {
        // 1970-01-01T00:00:00Z is exactly the epoch offset.
        assert_eq!(filetime_to_unix(0x019DB1DE, 0xD53E8000), 0);
        // Anything before 1601 + epoch offset is meaningless; report 0 rather
        // than a negative time that would sort before every real file.
        assert_eq!(filetime_to_unix(0, 0), 0);
    }

    #[test]
    fn filetime_round_trips_a_known_moment() {
        // 2020-01-01T00:00:00Z = 1577836800 unix.
        let ticks = (1_577_836_800i64 * 10_000_000) + 116_444_736_000_000_000;
        let high = (ticks as u64 >> 32) as u32;
        let low = (ticks as u64 & 0xFFFF_FFFF) as u32;
        assert_eq!(filetime_to_unix(high, low), 1_577_836_800);
    }

    #[test]
    fn stats_for_a_missing_file_are_none_not_a_panic() {
        assert!(file_stats(r"D:\definitely\not\here\nope.mp4").is_none());
    }

    #[test]
    fn stats_read_a_real_file() {
        // This crate's own source is guaranteed to exist while tests run.
        let me = std::env::current_exe().expect("test exe");
        let stats = file_stats(&me.to_string_lossy()).expect("stats for own exe");
        assert!(stats.size > 0, "một tệp thực thi không thể rỗng");
        assert!(stats.mtime > 1_500_000_000, "thời gian sửa phải hợp lý");
    }

    #[test]
    fn empty_props_are_distinguishable_from_real_ones() {
        assert!(MediaProps::default().is_empty());
        assert!(!MediaProps {
            width: 1920,
            height: 1080,
            duration_ms: 0
        }
        .is_empty());
        assert!(!MediaProps {
            width: 0,
            height: 0,
            duration_ms: 5000
        }
        .is_empty());
    }
}
