//! Playtime Adjustment: time the person adds to or removes from a Local
//! Game's Playtime by hand, for example for time played before the app was
//! installed. Steam Games have none; their Playtime is Steam's figure.

use crate::Game;

/// The number under which the Local Game `game_id` (`local:<number>`) is
/// stored; `None` for any other Game.
pub(crate) fn local_game_number(game_id: &str) -> Option<i64> {
    game_id.strip_prefix("local:")?.parse().ok()
}

/// The adjustment to store for `minutes` as entered: none for none or zero.
pub(crate) fn stored(minutes: Option<i32>) -> Option<i32> {
    minutes.filter(|&minutes| minutes != 0)
}

/// `game` with `minutes` added to its Playtime, which never drops below zero.
pub(crate) fn adjusted(game: Game, minutes: i32) -> Game {
    let played = i64::from(game.playtime_minutes.unwrap_or(0));
    let playtime = (played + i64::from(minutes)).clamp(0, i64::from(u32::MAX));
    Game { playtime_minutes: Some(u32::try_from(playtime).unwrap_or(u32::MAX)), ..game }
}
