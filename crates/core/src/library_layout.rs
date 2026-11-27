//! The Library Layout: whether the Library shows its Games as a cover grid or
//! as a list. The interface shows it; the core remembers the choice.

use serde::{Deserialize, Serialize};

/// How the Library shows its Games.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryLayout {
    /// One card with a large Cover per Game; until the user switches.
    CoverGrid,
    /// One row per Game with a small Cover.
    List,
}

impl LibraryLayout {
    /// How the store writes the Library Layout.
    pub(crate) fn as_setting(self) -> &'static str {
        match self {
            Self::CoverGrid => "cover_grid",
            Self::List => "list",
        }
    }

    /// The Library Layout the store wrote as `setting`. Anything else, such
    /// as a layout a newer version of the app knows, is the cover grid.
    pub(crate) fn from_setting(setting: &str) -> Self {
        match setting {
            "list" => Self::List,
            _ => Self::CoverGrid,
        }
    }
}
