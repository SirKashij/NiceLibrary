//! Status: the person's own judgement of where they stand with a Game. It is
//! set by hand only, is independent of Progress, and no Sync changes it.

use serde::{Deserialize, Serialize};

/// Where the person stands with a Game, in their own judgement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// Every Game starts here.
    #[default]
    Unplayed,
    Playing,
    Completed,
    Dropped,
}

impl Status {
    /// The Status as the store keeps it.
    pub(crate) fn as_column(self) -> &'static str {
        match self {
            Self::Unplayed => "unplayed",
            Self::Playing => "playing",
            Self::Completed => "completed",
            Self::Dropped => "dropped",
        }
    }

    /// The Status the store kept as `column`; `None` for anything else.
    pub(crate) fn from_column(column: &str) -> Option<Self> {
        [Self::Unplayed, Self::Playing, Self::Completed, Self::Dropped]
            .into_iter()
            .find(|status| status.as_column() == column)
    }
}
