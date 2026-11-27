//! Starts a Local Game's executable as a process of its own, with
//! `CreateProcessW` through the standard library, never through a shell.

use std::path::Path;
use std::process::{Command, Stdio};

use nicelibrary_core::ProcessStarter;

/// Starts executables (`.exe`) directly. The standard library quotes each
/// argument so the program reads it back as one, spaces and quotes included.
pub struct WindowsProcessStarter;

impl ProcessStarter for WindowsProcessStarter {
    fn start(&self, executable: &Path, arguments: &[String], working_directory: &Path) -> Result<(), String> {
        // Windows would run a batch file through its command processor, a shell.
        if !executable.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("exe")) {
            return Err(format!("not an executable that may be started: {}", executable.display()));
        }
        // An absolute path, so Windows looks for the program nowhere else.
        if !executable.is_absolute() {
            return Err(format!("not an absolute path: {}", executable.display()));
        }
        Command::new(executable)
            .args(arguments)
            .current_dir(working_directory)
            // The app has no console; the game gets nothing of the app's.
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            // Dropping the child neither waits for it nor ends it.
            .map(drop)
            .map_err(|error| format!("{} could not be started: {error}", executable.display()))
    }
}
