//! Asks the user for a Game's executable, an image for its Cover, or a
//! folder of Games, with Windows' own file dialogs.

use std::ffi::{OsString, c_void};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr;

use nicelibrary_core::{ExecutablePicker, ImagePicker};
use windows_sys::Win32::Foundation::{ERROR_CANCELLED, HWND};
use windows_sys::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize,
};
use windows_sys::Win32::UI::Shell::{
    FILEOPENDIALOGOPTIONS, FOS_DONTADDTORECENT, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR, FOS_PATHMUSTEXIST,
    FOS_PICKFOLDERS, FileOpenDialog, SIGDN, SIGDN_FILESYSPATH,
};
use windows_sys::core::{GUID, HRESULT};
use windows_sys::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, OFN_DONTADDTORECENT, OFN_EXPLORER, OFN_FILEMUSTEXIST,
    OFN_HIDEREADONLY, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
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

    /// Shows Windows' "Select Folder" dialog, in the language of Windows.
    fn choose_folder(&self) -> Result<Option<PathBuf>, String> {
        // A thread of its own, for the same reason as above.
        std::thread::Builder::new()
            .name("folder-picker".into())
            .spawn(show_folder_dialog)
            .map_err(|error| format!("the folder dialog could not be started: {error}"))?
            .join()
            .map_err(|_| "the folder dialog ended unexpectedly".to_owned())?
    }
}

/// `IID_IFileDialog`; windows-sys knows the dialog's class but not its interfaces.
const IID_IFILE_DIALOG: GUID = GUID::from_u128(0x42f85136_db7e_439c_85f1_e4075d135fc8);

/// What `IFileDialog::Show` answers when the user cancels:
/// `HRESULT_FROM_WIN32(ERROR_CANCELLED)`.
const CANCELLED: HRESULT = (0x8007_0000 | ERROR_CANCELLED) as HRESULT;

/// The start of `IFileDialog`'s table of methods, as far as it is used here
/// (shobjidl_core.h). The order is the interface's; unused entries are
/// placeholders of a pointer's size.
#[repr(C)]
struct FileDialogMethods {
    _query_interface: usize,
    _add_ref: usize,
    release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    show: unsafe extern "system" fn(this: *mut c_void, owner: HWND) -> HRESULT,
    _set_file_types: usize,
    _set_file_type_index: usize,
    _get_file_type_index: usize,
    _advise: usize,
    _unadvise: usize,
    set_options: unsafe extern "system" fn(this: *mut c_void, options: FILEOPENDIALOGOPTIONS) -> HRESULT,
    get_options: unsafe extern "system" fn(this: *mut c_void, options: *mut FILEOPENDIALOGOPTIONS) -> HRESULT,
    _set_default_folder: usize,
    _set_folder: usize,
    _get_folder: usize,
    _get_current_selection: usize,
    _set_file_name: usize,
    _get_file_name: usize,
    _set_title: usize,
    _set_ok_button_label: usize,
    _set_file_name_label: usize,
    get_result: unsafe extern "system" fn(this: *mut c_void, item: *mut *mut c_void) -> HRESULT,
}

/// The start of `IShellItem`'s table of methods, as far as it is used here.
#[repr(C)]
struct ShellItemMethods {
    _query_interface: usize,
    _add_ref: usize,
    release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    _bind_to_handler: usize,
    _get_parent: usize,
    get_display_name: unsafe extern "system" fn(this: *mut c_void, kind: SIGDN, name: *mut *mut u16) -> HRESULT,
}

/// The methods of the COM object at `object`.
///
/// # Safety
/// `object` points to a live COM object whose interface starts with `T`'s methods.
unsafe fn methods_of<'a, T>(object: *mut c_void) -> &'a T {
    // SAFETY: a COM object starts with a pointer to its table of methods.
    unsafe { &**(object as *mut *const T) }
}

fn show_folder_dialog() -> Result<Option<PathBuf>, String> {
    // SAFETY: plain COM calls; every object is released once, and the COM
    // apartment is left only after that.
    unsafe {
        let com = CoInitializeEx(ptr::null(), (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32);
        let answer = pick_folder();
        if com >= 0 {
            CoUninitialize();
        }
        answer
    }
}

/// # Safety
/// Runs on a thread in a single-threaded COM apartment.
unsafe fn pick_folder() -> Result<Option<PathBuf>, String> {
    let failed = |what: &str, code: HRESULT| format!("the folder dialog failed to {what} (error {code:#x})");
    let mut dialog: *mut c_void = ptr::null_mut();
    // SAFETY: the out pointer is valid; the answer is checked before use.
    let created = unsafe { CoCreateInstance(&FileOpenDialog, ptr::null_mut(), CLSCTX_INPROC_SERVER, &IID_IFILE_DIALOG, &mut dialog) };
    if created < 0 || dialog.is_null() {
        return Err(failed("open", created));
    }
    // SAFETY: `dialog` is a live IFileDialog until it is released at the end.
    let answer = unsafe {
        let methods = methods_of::<FileDialogMethods>(dialog);
        let mut options = 0;
        let set = match (methods.get_options)(dialog, &mut options) {
            code if code < 0 => code,
            // NOCHANGEDIR: the app's working folder stays where it is.
            _ => (methods.set_options)(
                dialog,
                options | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_NOCHANGEDIR | FOS_DONTADDTORECENT,
            ),
        };
        if set < 0 {
            Err(failed("start", set))
        } else {
            match (methods.show)(dialog, ptr::null_mut()) {
                CANCELLED => Ok(None),
                code if code < 0 => Err(failed("show", code)),
                _ => chosen_folder(dialog, methods).map(Some),
            }
        }
    };
    // SAFETY: the one reference this function holds.
    unsafe { (methods_of::<FileDialogMethods>(dialog).release)(dialog) };
    answer
}

/// The path of the folder the user chose in `dialog`.
///
/// # Safety
/// `dialog` is a live IFileDialog whose `Show` succeeded.
unsafe fn chosen_folder(dialog: *mut c_void, methods: &FileDialogMethods) -> Result<PathBuf, String> {
    let mut item: *mut c_void = ptr::null_mut();
    // SAFETY: the out pointer is valid; the answer is checked before use.
    let got = unsafe { (methods.get_result)(dialog, &mut item) };
    if got < 0 || item.is_null() {
        return Err(format!("the folder dialog gave no folder (error {got:#x})"));
    }
    // SAFETY: `item` is a live IShellItem until it is released below; the
    // name is freed with the allocator COM gave it with.
    unsafe {
        let item_methods = methods_of::<ShellItemMethods>(item);
        let mut name: *mut u16 = ptr::null_mut();
        let named = (item_methods.get_display_name)(item, SIGDN_FILESYSPATH, &mut name);
        let path = if named < 0 || name.is_null() {
            Err(format!("the chosen folder has no path (error {named:#x})"))
        } else {
            let length = (0..).take_while(|&at| *name.add(at) != 0).count();
            let path = PathBuf::from(OsString::from_wide(std::slice::from_raw_parts(name, length)));
            CoTaskMemFree(name as *const c_void);
            Ok(path)
        };
        (item_methods.release)(item);
        path
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

/// The dialog for files matching `patterns` (separated by `;`), on a thread
/// named `name`.
fn show_dialog_on_own_thread(name: &str, patterns: &'static str) -> Result<Option<PathBuf>, String> {
    // A thread of its own: the dialog needs a single-threaded COM
    // apartment, which the caller's thread may not be or may not want.
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || show_dialog(patterns))
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
