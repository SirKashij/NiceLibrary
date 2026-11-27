//! Statistics: figures over the Library, for the Statistics page. They are
//! worked out from the Library as it shows, so a Hidden Game counts in none
//! of them.

use serde::Serialize;

use crate::progress::COMPLETE;
use crate::{Game, Status};

/// The totals over the Games in the Library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    /// How many Games the Library holds.
    pub games: u32,
    /// The Playtime of every Game together, in minutes; a Game whose
    /// Playtime is not known yet adds nothing.
    pub playtime_minutes: u64,
    /// How many Achievements of all Steam Games are unlocked.
    pub achievements_unlocked: u32,
    /// How many Achievements the Steam Games have together.
    pub achievements_total: u32,
    /// The share of unlocked Achievements in whole percent, rounded down, so
    /// only all of them make 100. `None` while there are no Achievements.
    pub achievement_share: Option<u8>,
    /// How many Games are at 100 % Progress.
    pub complete_games: u32,
    /// How many Games have each Status.
    pub games_by_status: GamesByStatus,
}

/// How many Games have each Status.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GamesByStatus {
    pub unplayed: u32,
    pub playing: u32,
    pub completed: u32,
    pub dropped: u32,
}

/// The Statistics of `games`, the Library as it shows. `achievements`
/// answers how many of a Game's Achievements are unlocked and how many it
/// has, for a Game whose Achievements are known.
pub(crate) fn of(games: &[Game], achievements: impl Fn(&Game) -> Option<(u32, u32)>) -> Statistics {
    let mut statistics = Statistics {
        games: 0,
        playtime_minutes: 0,
        achievements_unlocked: 0,
        achievements_total: 0,
        achievement_share: None,
        complete_games: 0,
        games_by_status: GamesByStatus::default(),
    };
    for game in games {
        statistics.games += 1;
        statistics.playtime_minutes += u64::from(game.playtime_minutes.unwrap_or(0));
        if let Some((unlocked, total)) = achievements(game) {
            statistics.achievements_unlocked += unlocked.min(total);
            statistics.achievements_total += total;
        }
        if game.progress == Some(COMPLETE) {
            statistics.complete_games += 1;
        }
        let by_status = &mut statistics.games_by_status;
        *match game.status {
            Status::Unplayed => &mut by_status.unplayed,
            Status::Playing => &mut by_status.playing,
            Status::Completed => &mut by_status.completed,
            Status::Dropped => &mut by_status.dropped,
        } += 1;
    }
    statistics.achievement_share = share(statistics.achievements_unlocked, statistics.achievements_total);
    statistics
}

/// `part` of `whole` in whole percent, rounded down; `None` of nothing.
fn share(part: u32, whole: u32) -> Option<u8> {
    if whole == 0 {
        return None;
    }
    u8::try_from(u64::from(part.min(whole)) * u64::from(COMPLETE) / u64::from(whole)).ok()
}
