//! The theme of the interface: dark, light, or whichever the operating
//! system is set to. The interface applies it; the core remembers the choice.

use serde::{Deserialize, Serialize};

/// The theme the user chose in the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    /// Dark or light, as the operating system is set to; until the user chooses.
    System,
    Dark,
    Light,
}

impl Theme {
    /// How the store writes the theme.
    pub(crate) fn as_setting(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    /// The theme the store wrote as `setting`. Anything else, such as a theme
    /// a newer version of the app knows, follows the system.
    pub(crate) fn from_setting(setting: &str) -> Self {
        match setting {
            "dark" => Self::Dark,
            "light" => Self::Light,
            _ => Self::System,
        }
    }
}
