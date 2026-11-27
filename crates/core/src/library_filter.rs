//! Which Games of the Library the user wants to see: filters by Source, to
//! Installed Games and by Status, combined with a search in the title.

use serde::Deserialize;

use crate::{Game, Source, Status};

/// What the Library is narrowed to. Every part that is set must match; the
/// default filters nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LibraryFilter {
    /// Text the title must contain, ignoring case; empty matches every title.
    pub search: String,
    /// Only Games of this Source; `None` for both Sources.
    pub source: Option<Source>,
    /// Only Installed Games.
    pub installed_only: bool,
    /// Only Games with this Status; `None` for every Status.
    pub status: Option<Status>,
}

impl LibraryFilter {
    pub(crate) fn matches(&self, game: &Game) -> bool {
        self.source.is_none_or(|source| game.source == source)
            && (!self.installed_only || game.installed)
            && self.status.is_none_or(|status| game.status == status)
            && game.title.to_lowercase().contains(&self.search.to_lowercase())
    }
}
