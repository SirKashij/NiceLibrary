//! The application core of NiceLibrary.
//!
//! Every operation the interface can call lives here. The core knows nothing
//! about Tauri; the desktop shell is a thin adapter over these operations, and
//! everything that varies by machine or OS will reach the core through ports.

use serde::Serialize;

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

/// The application core. Each public method is one operation.
#[derive(Debug, Default)]
pub struct Core;

impl Core {
    pub fn new() -> Self {
        Self
    }

    /// All Games in the Library.
    pub fn library(&self) -> Vec<Game> {
        Vec::new()
    }
}
