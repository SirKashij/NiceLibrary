//! Launch: starting a Game from the app. A Steam Game is launched through
//! Steam itself, by opening its `steam://` URL, never through its executable
//! (ADR-0002).

use std::fmt;
use std::path::Path;

use serde::Serialize;

/// Port: opens a URL with the program the operating system has registered
/// for its scheme, as a click on a link would.
///
/// Contract: `Ok` once the registered program has been handed the URL; it
/// does not wait for that program to finish. `Err(NoProgram)` when no
/// program is registered for the scheme, `Err(Failed)` when it cannot be
/// handed the URL. Anything that is not a URL, and every `file://` URL, is
/// refused with `Failed`, so no program can be started through the opener
/// by its path. Never shows a dialog of its own.
///
/// `open_folder` shows a folder in the operating system's file manager.
/// Contract: `Ok` once the file manager has been handed the folder;
/// `Err(Failed)` when it cannot be, and for anything that is not an
/// existing folder, so no file or program is ever opened through it. An
/// opener that cannot show folders keeps the refusing default.
pub trait Opener: Send + Sync {
    fn open_url(&self, url: &str) -> Result<(), OpenError>;

    fn open_folder(&self, folder: &Path) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("showing the folder {} is not supported here", folder.display())))
    }
}

/// Why the [`Opener`] could not open a URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    /// No program is registered for the URL's scheme.
    NoProgram,
    /// Anything else, described for a log or a developer.
    Failed(String),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoProgram => f.write_str("no program is registered for this kind of URL"),
            Self::Failed(reason) => f.write_str(reason),
        }
    }
}

/// How a Launch went, when nothing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum Launch {
    /// The Game has been handed to whatever launches it.
    Launched,
    /// The Game is a Steam Game and Steam is not available on this computer:
    /// not installed, or no longer registered for its `steam://` URLs.
    SteamNotFound,
}

/// The URL that tells Steam to launch the app `app_id`.
pub(crate) fn steam_launch_url(app_id: u32) -> String {
    format!("steam://rungameid/{app_id}")
}

/// How starting the installation of a Steam Game went, when nothing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum Install {
    /// Steam has been asked to show its install dialog for the Game; the
    /// person goes on there.
    Started,
    /// Steam is not available on this computer: not installed, or no longer
    /// registered for its `steam://` URLs.
    SteamNotFound,
}

/// The URL that opens Steam's install dialog for the app `app_id`.
pub(crate) fn steam_install_url(app_id: u32) -> String {
    format!("steam://install/{app_id}")
}
