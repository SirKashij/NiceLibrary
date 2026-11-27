//! Starts the app with Windows through the current user's `Run` key, the
//! entry the Task Manager lists under startup apps. Only
//! `HKEY_CURRENT_USER` is written, so other users are never touched.

use std::io::ErrorKind;
use std::path::PathBuf;

use nicelibrary_core::Autostart;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};

/// The programs Windows starts when the current user signs in.
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Where Windows records that the user turned an entry of `RUN` off or on in
/// the Task Manager or the Settings app; an entry without a mark here is on.
const STARTUP_APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// The name of the app's entry.
const NAME: &str = "NiceLibrary";

/// The current user's autostart entry `name` under `run_key` for `program`.
pub struct WindowsAutostart {
    run_key: String,
    approved_key: String,
    name: String,
    /// `None` when the running program's path is not known.
    program: Option<PathBuf>,
}

impl WindowsAutostart {
    /// The app's own entry, starting the program that runs now.
    pub fn new() -> Self {
        Self::under(RUN, STARTUP_APPROVED, NAME, std::env::current_exe().ok())
    }

    /// The entry `name` under the keys `run_key` and `approved_key` of
    /// `HKEY_CURRENT_USER`; for tests that must not touch Windows' own keys.
    pub fn under(run_key: &str, approved_key: &str, name: &str, program: Option<PathBuf>) -> Self {
        Self { run_key: run_key.into(), approved_key: approved_key.into(), name: name.into(), program }
    }

    /// The command line the entry runs: the quoted program and `arguments`.
    fn command(&self, arguments: &[&str]) -> Option<String> {
        let program = self.program.as_ref()?;
        let mut command = format!("\"{}\"", program.display());
        for argument in arguments {
            command.push(' ');
            command.push_str(argument);
        }
        Some(command)
    }

    /// Whether the user turned the entry off in the startup apps: Windows
    /// keeps 12 bytes whose first is odd for off.
    fn disabled_in_startup_apps(&self) -> Result<bool, String> {
        let Some(key) = open(&self.approved_key)? else {
            return Ok(false);
        };
        match key.get_raw_value(&self.name) {
            Ok(value) => Ok(value.bytes.first().is_some_and(|first| first & 1 == 1)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(format!("the startup apps could not be read: {error}")),
        }
    }
}

impl Default for WindowsAutostart {
    fn default() -> Self {
        Self::new()
    }
}

impl Autostart for WindowsAutostart {
    fn is_on(&self) -> Result<bool, String> {
        let Some(key) = open(&self.run_key)? else {
            return Ok(false);
        };
        let command: String = match key.get_value(&self.name) {
            Ok(command) => command,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(format!("the autostart entry could not be read: {error}")),
        };
        // An entry for this program with other arguments is still this app's.
        let Some(program) = self.command(&[]) else {
            return Ok(false);
        };
        let ours = command == program || command.starts_with(&format!("{program} "));
        Ok(ours && !self.disabled_in_startup_apps()?)
    }

    fn turn_on(&self, arguments: &[&str]) -> Result<(), String> {
        let command = self.command(arguments).ok_or("the app's program could not be found")?;
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(&self.run_key)
            .map_err(|error| format!("the startup programs could not be opened: {error}"))?;
        key.set_value(&self.name, &command).map_err(|error| format!("the autostart entry could not be written: {error}"))?;
        // Turned on in the app, it is on, even if the user had turned it off in the startup apps.
        delete_value(&self.approved_key, &self.name)
    }

    fn turn_off(&self) -> Result<(), String> {
        delete_value(&self.run_key, &self.name)?;
        delete_value(&self.approved_key, &self.name)
    }
}

/// The key `path` of the current user for reading; `None` when it does not exist.
fn open(path: &str) -> Result<Option<RegKey>, String> {
    match RegKey::predef(HKEY_CURRENT_USER).open_subkey(path) {
        Ok(key) => Ok(Some(key)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("the startup programs could not be opened: {error}")),
    }
}

/// Deletes the value `name` of the current user's key `path`; `Ok` when
/// there is none.
fn delete_value(path: &str, name: &str) -> Result<(), String> {
    let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(path, KEY_SET_VALUE) {
        Ok(key) => key,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("the startup programs could not be opened: {error}")),
    };
    match key.delete_value(name) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("the autostart entry could not be removed: {error}")),
    }
}
