//! Opens URLs with the program Windows has registered for their scheme, the
//! way a click on a link does. Steam registers itself for `steam://`. Shows
//! folders in File Explorer.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use nicelibrary_core::{OpenError, Opener};
use windows_sys::Win32::Foundation::{GetLastError, S_FALSE, S_OK};
use windows_sys::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows_sys::Win32::UI::Shell::{
    ASSOCF_INIT_IGNOREUNKNOWN, ASSOCSTR_COMMAND, AssocQueryStringW, SEE_MASK_CLASSNAME, SEE_MASK_FLAG_NO_UI,
    SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Hands URLs and folders to the Windows shell.
pub struct WindowsOpener;

impl Opener for WindowsOpener {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        let Some(scheme) = openable_scheme(url) else {
            return Err(OpenError::Failed(format!("not a URL that may be opened: {url}")));
        };
        // Without a registered program, Windows would offer to find an app
        // for the link instead of failing, even when told to show no dialog.
        if !is_registered(scheme) {
            return Err(OpenError::NoProgram);
        }
        shell_execute(OsStr::new(url), None)
            .map_err(|code| OpenError::Failed(format!("Windows could not open {url} (error {code})")))
    }

    fn open_folder(&self, folder: &Path) -> Result<(), OpenError> {
        // Only an existing folder: a file or a program is never opened.
        if !folder.is_dir() {
            return Err(OpenError::Failed(format!("not a folder: {}", folder.display())));
        }
        // Opened as a folder, whatever its name looks like.
        shell_execute(folder.as_os_str(), Some("folder")).map_err(|code| {
            OpenError::Failed(format!("Windows could not show the folder {} (error {code})", folder.display()))
        })
    }
}

/// Opens `file` with the shell's `open` verb, as the file type `class` when
/// one is given. Answers Windows' error code when it fails.
fn shell_execute(file: &OsStr, class: Option<&str>) -> Result<(), u32> {
    let verb = wide(OsStr::new("open"));
    let file = wide(file);
    let class = class.map(|class| wide(OsStr::new(class)));
    // SAFETY: all-zero is a valid SHELLEXECUTEINFOW (null pointers, no flags).
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
    // No dialog of Windows' own when nothing is registered; return only
    // once the URL or folder has been handed over.
    info.fMask = SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.nShow = SW_SHOWNORMAL;
    if let Some(class) = &class {
        info.fMask |= SEE_MASK_CLASSNAME;
        info.lpClass = class.as_ptr();
    }

    // The shell may use COM to reach the registered program; Microsoft
    // asks callers to initialise it on the calling thread first.
    // SAFETY: plain Win32 calls; `info` and the strings it points to
    // outlive the call.
    unsafe {
        let com = CoInitializeEx(ptr::null(), (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32);
        let opened = ShellExecuteExW(&mut info) != 0;
        let error = GetLastError();
        if com >= 0 {
            CoUninitialize();
        }
        opened.then_some(()).ok_or(error)
    }
}

/// The scheme of `text` if it is a URL of a scheme other than `file`: only
/// those are opened, so no path to a program can be started through the
/// opener.
fn openable_scheme(text: &str) -> Option<&str> {
    let (scheme, _) = text.split_once("://")?;
    // A single letter is a drive ("C://"), not a scheme.
    let url = scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        && !scheme.eq_ignore_ascii_case("file")
        && !text.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"');
    url.then_some(scheme)
}

/// Whether a program is registered to open URLs of `scheme`.
fn is_registered(scheme: &str) -> bool {
    let scheme = wide(OsStr::new(scheme));
    let verb = wide(OsStr::new("open"));
    let mut length = 0;
    // SAFETY: plain Win32 call; with no output buffer it only reports the
    // length of the registered command.
    let result = unsafe {
        AssocQueryStringW(
            ASSOCF_INIT_IGNOREUNKNOWN,
            ASSOCSTR_COMMAND,
            scheme.as_ptr(),
            verb.as_ptr(),
            ptr::null_mut(),
            &mut length,
        )
    };
    // S_OK, or S_FALSE for "here is the length you need".
    (result == S_OK || result == S_FALSE) && length > 1
}

/// `text` as a null-terminated UTF-16 string.
fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}
