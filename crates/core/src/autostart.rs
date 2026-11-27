//! Starting with Windows: the app starts when the user signs in, hidden in
//! the Tray. The setting is the operating system's autostart entry itself,
//! never a copy in the store, so it always shows what will really happen.

use crate::Error;

/// The argument the autostart entry starts the app with. A start with it is
/// a start with Windows, which keeps the window hidden in the Tray.
pub const AUTOSTART_ARGUMENT: &str = "--autostart";

/// The current user's autostart entry for this app: whether the app starts
/// when that user signs in. Other users of the computer are never touched.
pub trait Autostart: Send + Sync {
    /// Whether the entry is there and the system will start the app with it,
    /// also after the user changed or removed it outside the app.
    fn is_on(&self) -> Result<bool, String>;
    /// Adds the entry, or replaces it, so the app starts with `arguments`.
    fn turn_on(&self, arguments: &[&str]) -> Result<(), String>;
    /// Removes the entry; `Ok` when there is none.
    fn turn_off(&self) -> Result<(), String>;
}

/// Whether the app was started by its autostart entry, judged by the
/// arguments it was started with (the program name may be among them).
pub fn started_with_windows<I>(arguments: I) -> bool
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    arguments.into_iter().any(|argument| argument.as_ref() == AUTOSTART_ARGUMENT)
}

pub(crate) fn starts_with_windows(autostart: &dyn Autostart) -> Result<bool, Error> {
    autostart.is_on().map_err(autostart_error)
}

pub(crate) fn set_start_with_windows(autostart: &dyn Autostart, on: bool) -> Result<(), Error> {
    let changed = if on { autostart.turn_on(&[AUTOSTART_ARGUMENT]) } else { autostart.turn_off() };
    changed.map_err(autostart_error)
}

fn autostart_error(reason: String) -> Error {
    Error::new(format!("autostart entry: {reason}"))
}
