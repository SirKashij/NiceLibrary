//! Asks the user for a Game's executable, or an image for its Cover, or
//! where to save an Export File, with Windows' own file dialogs.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr;

use nicelibrary_core::{ExecutablePicker, ExportFilePicker, ImagePicker};
use windows_sys::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows_sys::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, GetSaveFileNameW, OFN_DONTADDTORECENT, OFN_EXPLORER,
    OFN_FILEMUSTEXIST, OFN_HIDEREADONLY, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};

/// Room for the longest path Windows can hand back, in UTF-16 units.
const PATH_CAPACITY: usize = 32_768;

/// Shows Windows' "Open" dialog for executables (`*.exe`). Its title and
/// buttons are Windows' own, in the language of Windows.
pub struct WindowsExecutablePicker;

impl ExecutablePicker for WindowsExecutablePicker {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        show_dialog_on_own_thread("executable-picker", "*.exe")
    }
}

/// Shows Windows' "Open" dialog for images a Cover may be (`*.png`, `*.jpg`,
/// `*.jpeg`). Its title and buttons are Windows' own, in the language of
/// Windows.
pub struct WindowsImagePicker;

impl ImagePicker for WindowsImagePicker {
    fn choose_image(&self) -> Result<Option<PathBuf>, String> {
        show_dialog_on_own_thread("image-picker", "*.png;*.jpg;*.jpeg")
    }
}

/// Shows Windows' "Save as" dialog for an Export File (`*.json`), with the
/// suggested name filled in. It asks before replacing a file that exists,
/// and adds `.json` when the user types a name without an extension. Its
/// title and buttons are Windows' own, in the language of Windows.
pub struct WindowsExportFilePicker;

impl ExportFilePicker for WindowsExportFilePicker {
    fn choose_export_file(&self, suggested_name: &str) -> Result<Option<PathBuf>, String> {
        let suggested = suggested_name.to_owned();
        on_own_thread("export-file-picker", move || show_save_dialog(&suggested))
    }
}

/// The dialog for files matching `patterns` (separated by `;`), on a thread
/// named `name`.
fn show_dialog_on_own_thread(name: &str, patterns: &'static str) -> Result<Option<PathBuf>, String> {
    on_own_thread(name, move || show_dialog(patterns))
}

/// Runs the dialog `show` on a thread named `name`.
fn on_own_thread(
    name: &str,
    show: impl FnOnce() -> Result<Option<PathBuf>, String> + Send + 'static,
) -> Result<Option<PathBuf>, String> {
    // A thread of its own: the dialog needs a single-threaded COM
    // apartment, which the caller's thread may not be or may not want.
    std::thread::Builder::new()
        .name(name.into())
        .spawn(show)
        .map_err(|error| format!("the file dialog could not be started: {error}"))?
        .join()
        .map_err(|_| "the file dialog ended unexpectedly".to_owned())?
}

fn show_dialog(patterns: &str) -> Result<Option<PathBuf>, String> {
    // The same text in both halves: a filter's name and its pattern, so the
    // dialog reads alike in every language.
    let filter: Vec<u16> = format!("{patterns}\0{patterns}\0\0").encode_utf16().collect();
    let mut path = vec![0u16; PATH_CAPACITY];
    // SAFETY: all-zero is a valid OPENFILENAMEW (null pointers, no flags).
    let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    dialog.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    dialog.lpstrFilter = filter.as_ptr();
    dialog.nFilterIndex = 1;
    dialog.lpstrFile = path.as_mut_ptr();
    dialog.nMaxFile = PATH_CAPACITY as u32;
    // NOCHANGEDIR: the app's working folder stays where it is.
    dialog.Flags = OFN_EXPLORER
        | OFN_FILEMUSTEXIST
        | OFN_PATHMUSTEXIST
        | OFN_HIDEREADONLY
        | OFN_NOCHANGEDIR
        | OFN_DONTADDTORECENT;

    // SAFETY: plain Win32 calls; `dialog` and the buffers it points to
    // outlive the call.
    let (chosen, error) = unsafe {
        let com = CoInitializeEx(ptr::null(), (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32);
        let chosen = GetOpenFileNameW(&mut dialog) != 0;
        let error = if chosen { 0 } else { CommDlgExtendedError() };
        if com >= 0 {
            CoUninitialize();
        }
        (chosen, error)
    };
    match (chosen, error) {
        (true, _) => {
            let length = path.iter().position(|&unit| unit == 0).unwrap_or(path.len());
            Ok(Some(PathBuf::from(OsString::from_wide(&path[..length]))))
        }
        // No error: the user cancelled.
        (false, 0) => Ok(None),
        (false, code) => Err(format!("the file dialog failed (error {code:#x})")),
    }
}

fn show_save_dialog(suggested_name: &str) -> Result<Option<PathBuf>, String> {
    let filter: Vec<u16> = "*.json\0*.json\0\0".encode_utf16().collect();
    let extension: Vec<u16> = "json\0".encode_utf16().collect();
    let mut path = vec![0u16; PATH_CAPACITY];
    // The suggested name, cut to fit, with room for the closing zero.
    for (slot, unit) in path.iter_mut().zip(suggested_name.encode_utf16().take(PATH_CAPACITY - 1)) {
        *slot = unit;
    }
    // SAFETY: all-zero is a valid OPENFILENAMEW (null pointers, no flags).
    let mut dialog: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    dialog.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    dialog.lpstrFilter = filter.as_ptr();
    dialog.nFilterIndex = 1;
    dialog.lpstrFile = path.as_mut_ptr();
    dialog.nMaxFile = PATH_CAPACITY as u32;
    dialog.lpstrDefExt = extension.as_ptr();
    // NOCHANGEDIR: the app's working folder stays where it is.
    dialog.Flags = OFN_EXPLORER
        | OFN_OVERWRITEPROMPT
        | OFN_PATHMUSTEXIST
        | OFN_HIDEREADONLY
        | OFN_NOCHANGEDIR
        | OFN_DONTADDTORECENT;

    // SAFETY: plain Win32 calls; `dialog` and the buffers it points to
    // outlive the call.
    let (chosen, error) = unsafe {
        let com = CoInitializeEx(ptr::null(), (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32);
        let chosen = GetSaveFileNameW(&mut dialog) != 0;
        let error = if chosen { 0 } else { CommDlgExtendedError() };
        if com >= 0 {
            CoUninitialize();
        }
        (chosen, error)
    };
    match (chosen, error) {
        (true, _) => {
            let length = path.iter().position(|&unit| unit == 0).unwrap_or(path.len());
            Ok(Some(PathBuf::from(OsString::from_wide(&path[..length]))))
        }
        // No error: the user cancelled.
        (false, 0) => Ok(None),
        (false, code) => Err(format!("the file dialog failed (error {code:#x})")),
    }
}
