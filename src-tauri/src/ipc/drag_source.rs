//! Dragging files out of the window, as a native OLE drag source.
//!
//! The web side cannot do this: a WebView drag offers the formats a web page
//! can offer, and everything that accepts files reads `CF_HDROP`. So the drag
//! is started here instead.
//!
//! # Why this is not the `drag` crate
//!
//! It was, briefly. That crate does the same thing and does it well — except
//! it calls `dunce::canonicalize` on every path first. On a **mapped network
//! drive** that resolves to `\\?\UNC\server\share\…`, and dunce deliberately
//! never simplifies UNC (a UNC path takes `..` literally, so stripping the
//! prefix could change what the path means). `ILCreateFromPathW` refuses that
//! form, the crate unwraps the resulting `None`, and because the panic happens
//! inside the window procedure — where it **cannot unwind** — the whole
//! application aborts.
//!
//! Measured on this machine:
//!
//! ```text
//! F:\…\clip.mov                        → shell accepts
//! \\?\UNC\192.168.1.214\f\…\clip.mov   → shell refuses
//! ```
//!
//! 87% of this user's library is on a NAS, so "dragging crashes the app" was
//! the normal case rather than an edge case.
//!
//! What was left to write turned out to be small, because **the shell builds
//! the data object**: `SHCreateShellItemArrayFromIDLists` plus
//! `BindToHandler(BHID_DataObject)` yields a full shell `IDataObject` — the one
//! that carries `CF_HDROP`, `Shell IDList Array`, `FileNameW` and the rest.
//! Only `IDropSource`, three short methods, has to be implemented here.
//!
//! Dropping the dependency also removed a second `windows` crate version from
//! the build (see `docs/config.md`, CONF-006).

use std::path::Path;

use windows::core::{implement, Result, BOOL, HSTRING, PCWSTR};
use windows::Win32::Foundation::{
    DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, S_OK,
};
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::Ole::{
    DoDragDrop, IDropSource, IDropSource_Impl, OleInitialize, DROPEFFECT, DROPEFFECT_COPY,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    BHID_DataObject, ILCreateFromPathW, ILFree, SHCreateShellItemArrayFromIDLists,
};

/// The left mouse button, as reported to `QueryContinueDrag`.
const MK_LBUTTON: u32 = 0x0001;

/// Tells Windows whether the drag is still going.
///
/// The whole state machine of a drag lives in these two callbacks, and both are
/// as short as they look: the drag ends when the button comes up, and is
/// cancelled when Escape is pressed. Everything else — the cursor, the image
/// under it, the highlighting of drop targets — Windows does.
#[implement(IDropSource)]
struct DropSource;

impl IDropSource_Impl for DropSource_Impl {
    fn QueryContinueDrag(
        &self,
        escape_pressed: BOOL,
        key_state: MODIFIERKEYS_FLAGS,
    ) -> windows::core::HRESULT {
        if escape_pressed.as_bool() {
            return DRAGDROP_S_CANCEL;
        }
        if key_state.0 & MK_LBUTTON == 0 {
            // Button released: this is the drop.
            return DRAGDROP_S_DROP;
        }
        S_OK
    }

    fn GiveFeedback(&self, _effect: DROPEFFECT) -> windows::core::HRESULT {
        // Let Windows draw the cursor. Doing it here would mean reimplementing
        // the copy/move/no-entry cursors and getting them wrong on some theme.
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

/// A list of shell ids that frees itself.
///
/// Each `ILCreateFromPathW` hands back memory the caller owns. A drag is
/// something a user repeats all day, so leaking one list per drag is a leak
/// that grows with use.
struct Pidls(Vec<*const ITEMIDLIST>);

impl Drop for Pidls {
    fn drop(&mut self) {
        for &p in &self.0 {
            if !p.is_null() {
                unsafe { ILFree(Some(p)) };
            }
        }
    }
}

/// Can the shell turn this path into an item it can carry?
///
/// Asked before starting a drag so a path the shell refuses — a file deleted
/// since the last scan — is dropped from the list rather than failing the drag.
pub fn shell_accepts(path: &Path) -> bool {
    let wide = HSTRING::from(path.as_os_str());
    unsafe {
        let pidl = ILCreateFromPathW(PCWSTR(wide.as_ptr()));
        if pidl.is_null() {
            return false;
        }
        ILFree(Some(pidl));
        true
    }
}

/// The shell's data object for `paths` — what a drop target actually reads.
///
/// Split out of `drag_files` so it can be tested without a mouse. `DoDragDrop`
/// needs a real drag gesture and cannot run unattended, but everything that can
/// refuse a particular set of paths is decided here.
pub fn data_object_for(paths: &[&Path]) -> Result<IDataObject> {
    unsafe {
        // The shell calls below need COM on this thread, so the function makes
        // that true itself rather than leaving it as a precondition a caller
        // can forget. Idempotent, and cheap enough to repeat per drag.
        let _ = OleInitialize(None);

        // Note what is *not* here: no canonicalisation. The path as the index
        // stores it — `F:\…` for a mapped drive — is exactly the form the shell
        // resolves. Resolving it to its true `\?\UNC\…` location first is what
        // broke the previous implementation.
        let pidls = Pidls(
            paths
                .iter()
                .map(|p| {
                    let wide = HSTRING::from(p.as_os_str());
                    ILCreateFromPathW(PCWSTR(wide.as_ptr())).cast_const()
                })
                .collect(),
        );
        if pidls.0.iter().any(|p| p.is_null()) {
            return Err(windows::core::Error::from_win32());
        }

        let items = SHCreateShellItemArrayFromIDLists(&pidls.0)?;
        // The shell's own data object: `CF_HDROP`, `Shell IDList Array`,
        // `FileNameW`, `FileContents` and the rest, all built by Windows.
        items.BindToHandler(None, &BHID_DataObject)
    }
}

/// Drag `paths` out of the application.
///
/// **Blocks until the drop finishes.** `DoDragDrop` runs its own modal message
/// loop, so this must be called on the thread that owns the window; the window
/// sits still for the duration, exactly as Explorer's does.
///
/// Always a copy. A search tool has no business relocating the files it found:
/// dropping a result into a folder must leave the original where it was.
pub fn drag_files(paths: &[&Path]) -> Result<()> {
    unsafe {
        let data = data_object_for(paths)?;
        let source: IDropSource = DropSource.into();
        let mut effect = DROPEFFECT::default();
        let _ = DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect);
        Ok(())
    }
}
