//! The interface language: German, English, or whichever the operating system
//! is set to. The interface applies it; the core remembers the choice.

use serde::{Deserialize, Serialize};

/// The interface language the user chose in the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// The system language, English for anything but German; until the user chooses.
    #[serde(rename = "system")]
    System,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "en")]
    English,
}

impl Language {
    /// How the store writes the language.
    pub(crate) fn as_setting(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::German => "de",
            Self::English => "en",
        }
    }

    /// The language the store wrote as `setting`. Anything else, such as a
    /// language a newer version of the app knows, follows the system.
    pub(crate) fn from_setting(setting: &str) -> Self {
        match setting {
            "de" => Self::German,
            "en" => Self::English,
            _ => Self::System,
        }
    }
}
