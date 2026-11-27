//! Closing the window: by default the app keeps running in the Tray, so
//! tracking goes on, and the first time it says so.

use serde::Serialize;

use crate::store::Store;
use crate::Error;

/// The setting that turns close-to-tray off ("false"); on while it is missing.
pub(crate) const CLOSE_TO_TRAY: &str = "close_to_tray";

/// The setting that records that the Tray notice was shown ("true").
const TRAY_NOTICE_SHOWN: &str = "tray_notice_shown";

/// What closing the window does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum WindowClose {
    /// The first close with close-to-tray on: the window stays open and
    /// explains that the app keeps running in the Tray. Once the user has
    /// read it, closing again keeps the app running in the Tray.
    ShowTrayNotice,
    /// The window hides and the app keeps running in the Tray.
    KeepRunningInTray,
    /// Close-to-tray is off: the app quits.
    Quit,
}

/// Whether closing the window keeps the app running in the Tray.
pub(crate) fn close_to_tray(store: &Store) -> Result<bool, Error> {
    Ok(store.setting(CLOSE_TO_TRAY)?.as_deref() != Some("false"))
}

pub(crate) fn set_close_to_tray(store: &Store, on: bool) -> Result<(), Error> {
    store.set_setting(CLOSE_TO_TRAY, if on { "true" } else { "false" })
}

/// What closing the window does now; the first time it keeps the app
/// running, it also records that the Tray notice was shown.
pub(crate) fn close_window(store: &Store) -> Result<WindowClose, Error> {
    if !close_to_tray(store)? {
        return Ok(WindowClose::Quit);
    }
    if store.setting(TRAY_NOTICE_SHOWN)?.as_deref() == Some("true") {
        return Ok(WindowClose::KeepRunningInTray);
    }
    store.set_setting(TRAY_NOTICE_SHOWN, "true")?;
    Ok(WindowClose::ShowTrayNotice)
}
