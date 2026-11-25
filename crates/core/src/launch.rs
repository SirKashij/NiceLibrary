//! Launch: starting a Game from the app. A Steam Game starts through Steam
//! itself, by opening its `steam://` URL, never through its executable
//! (ADR-0002).

use serde::Serialize;

/// Port: opens a URL with the program the operating system has registered
/// for its scheme, as a click on a link would.
///
/// Contract: `Ok` once the registered program has been handed the URL; it
/// does not wait for that program to finish. `Err` when no program is
/// registered for the scheme or it cannot be started. Never shows a dialog
/// of its own.
pub trait Opener: Send + Sync {
    fn open_url(&self, url: &str) -> Result<(), String>;
}

/// How a Launch went, when nothing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum Launch {
    /// The Game has been handed to whatever starts it.
    Started,
    /// The Game is a Steam Game and Steam is not installed on this computer.
    SteamNotFound,
}

/// The URL that tells Steam to start the app `app_id`.
pub(crate) fn steam_run_url(app_id: u32) -> String {
    format!("steam://rungameid/{app_id}")
}
