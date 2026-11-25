//! The application core of NiceLibrary.
//!
//! Every operation the interface can call lives here. The core knows nothing
//! about Tauri; the desktop shell is a thin adapter over these operations, and
//! everything that varies by machine or OS reaches the core through ports.

pub mod keyvalues;
mod steam;

use serde::Serialize;

pub use steam::{SteamAccount, SteamInstallation, SteamLocator};

/// The origin a Game is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Steam,
    Local,
}

/// One playable title in the Library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub title: String,
    pub source: Source,
    pub installed: bool,
}

/// Everything that varies by machine or OS, handed to the core.
pub struct Ports {
    pub steam_locator: Box<dyn SteamLocator>,
}

/// The application core. Each public method is one operation.
pub struct Core {
    ports: Ports,
}

impl Core {
    pub fn new(ports: Ports) -> Self {
        Self { ports }
    }

    /// All Games in the Library.
    pub fn library(&self) -> Vec<Game> {
        Vec::new()
    }

    /// The Steam installation on this computer and its Steam Accounts.
    pub fn steam_installation(&self) -> SteamInstallation {
        steam::read_installation(self.ports.steam_locator.as_ref())
    }
}
