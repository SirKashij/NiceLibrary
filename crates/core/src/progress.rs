//! Progress: how far a Game has been completed, in percent, worked out per
//! Source and carried on [`Game::progress`](crate::Game::progress). A Local
//! Game's Progress is its Manual Progress or nothing; a Steam Game's is the
//! share of its unlocked Achievements, or nothing while it has none.

use serde::Serialize;

use crate::Game;

/// The highest Progress, in percent.
pub(crate) const COMPLETE: u8 = 100;

/// How setting or clearing a Local Game's Manual Progress went, when nothing
/// failed. Every outcome but `Set` changed nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum ManualProgress {
    /// The Game's Manual Progress is set or cleared; `game` is the Game as
    /// the Library shows it now.
    Set { game: Game },
    /// The percentage is not between 0 and 100.
    OutOfRange,
    /// The Game is not a Local Game; only Local Games have Manual Progress.
    NotALocalGame,
}

/// `percent` as a Progress, when it is one: a whole number from 0 to 100.
pub(crate) fn percent_in(percent: i64) -> Option<u8> {
    u8::try_from(percent).ok().filter(|percent| *percent <= COMPLETE)
}

/// The Progress of a Local Game: its Manual Progress, or nothing.
pub(crate) fn of_local_game(manual_progress: Option<u8>) -> Option<u8> {
    manual_progress
}

/// The Progress of a Steam Game with `unlocked` of its `total` Achievements
/// unlocked: the share in whole percent, rounded down, so only a Game with
/// every Achievement unlocked is complete. A Game without Achievements has
/// no Progress, not 0 %.
pub(crate) fn of_steam_game(unlocked: u32, total: u32) -> Option<u8> {
    if total == 0 {
        return None;
    }
    let percent = u64::from(unlocked.min(total)) * u64::from(COMPLETE) / u64::from(total);
    u8::try_from(percent).ok()
}
