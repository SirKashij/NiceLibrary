//! Which Games of the Library the user wants to see, and in which order:
//! filters by Source, to Installed Games, by Status, to Favorites and to a
//! Collection, combined with a search in the title, and a sort order.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{Game, Source, Status};

/// What the Library is narrowed to, and how it is ordered. Every part that
/// is set must match; the default filters nothing and sorts by recently
/// played.
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
    /// The order the Games are answered in.
    pub sort: LibrarySort,
    /// Only Favorites.
    pub favorites_only: bool,
    /// Only Games in the Collection with this id, the one chosen in the
    /// sidebar; `None` for every Game. A Collection that is gone holds none.
    pub collection: Option<i64>,
}

impl LibraryFilter {
    pub(crate) fn matches(&self, game: &Game) -> bool {
        self.source.is_none_or(|source| game.source == source)
            && (!self.installed_only || game.installed)
            && self.status.is_none_or(|status| game.status == status)
            && (!self.favorites_only || game.favorite)
            && game.title.to_lowercase().contains(&self.search.to_lowercase())
    }
}

/// The Sort Order of the Library. Every order puts the Games without a value
/// last, and Games with the same value in the order of their titles, so the
/// same Library is always answered in the same order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibrarySort {
    /// Last Played, the most recent first; Games never played last.
    #[default]
    RecentlyPlayed,
    /// Title, ignoring case.
    Name,
    /// Playtime, the longest first; Games whose Playtime is not known last.
    Playtime,
    /// Progress, the most complete first; Games without Progress last.
    Progress,
}

impl LibrarySort {
    const ALL: [Self; 4] = [Self::RecentlyPlayed, Self::Name, Self::Playtime, Self::Progress];

    /// The name the store keeps this order under.
    pub(crate) fn stored_name(self) -> &'static str {
        match self {
            Self::RecentlyPlayed => "recentlyPlayed",
            Self::Name => "name",
            Self::Playtime => "playtime",
            Self::Progress => "progress",
        }
    }

    /// The order stored as `name`; `None` for a name no order has.
    pub(crate) fn from_stored_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|sort| sort.stored_name() == name)
    }

    /// Sorts `games` in this order.
    pub(crate) fn sort(self, games: &mut [Game]) {
        games.sort_by(|a, b| {
            let first = match self {
                Self::RecentlyPlayed => greatest_first(a.last_played, b.last_played),
                Self::Name => Ordering::Equal,
                Self::Playtime => greatest_first(a.playtime_minutes, b.playtime_minutes),
                Self::Progress => greatest_first(a.progress, b.progress),
            };
            first.then_with(|| by_title(a, b))
        });
    }
}

/// The greater value first, no value last.
fn greatest_first<T: Ord>(a: Option<T>, b: Option<T>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => b.cmp(&a),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// By title ignoring case, and by id between Games of the same title.
pub(crate) fn by_title(a: &Game, b: &Game) -> Ordering {
    a.title.to_lowercase().cmp(&b.title.to_lowercase()).then_with(|| a.id.cmp(&b.id))
}
