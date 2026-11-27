//! Lists the running processes with their executables, through a Tool Help
//! snapshot of the process table and each process's full image name.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use nicelibrary_core::ProcessList;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};

/// The processes of this Windows session.
pub struct WindowsProcessList;

impl ProcessList for WindowsProcessList {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        // Long paths can exceed MAX_PATH; 32 767 characters is Windows' limit.
        let mut buffer = vec![0u16; 32_768];
        Ok(process_ids()?.into_iter().filter_map(|id| executable_of(id, &mut buffer)).collect())
    }
}

/// The ids of every process running now.
fn process_ids() -> Result<Vec<u32>, String> {
    // SAFETY: plain Win32 call; the handle is closed by `Handle`.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        // SAFETY: plain Win32 call.
        return Err(format!("no snapshot of the processes (error {})", unsafe { GetLastError() }));
    }
    let snapshot = Handle(snapshot);
    // SAFETY: all-zero is a valid PROCESSENTRY32W.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut ids = Vec::new();
    // SAFETY: `entry` is a PROCESSENTRY32W with its size set, alive for the calls.
    let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) } != 0;
    while more {
        ids.push(entry.th32ProcessID);
        // SAFETY: as above.
        more = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
    }
    Ok(ids)
}

/// The full path of the executable process `id` runs; `None` when it is not
/// to be read, as for the system's own processes or one that just ended.
fn executable_of(id: u32, buffer: &mut [u16]) -> Option<PathBuf> {
    // SAFETY: plain Win32 call; a null handle means no access.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, id) };
    if process.is_null() {
        return None;
    }
    let process = Handle(process);
    let mut length = buffer.len() as u32;
    // SAFETY: `buffer` holds `length` characters and outlives the call.
    let read = unsafe { QueryFullProcessImageNameW(process.0, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut length) };
    (read != 0).then(|| PathBuf::from(OsString::from_wide(&buffer[..length as usize])))
}

/// A handle closed when dropped.
struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle is open and owned by this value.
        unsafe { CloseHandle(self.0) };
    }
}
