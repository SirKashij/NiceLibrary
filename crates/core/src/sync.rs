//! Sync: one round of fetching the Steam Account's Games, with Steam's
//! Playtime and Last Played, through the Steam Web API.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::steam_web_api::{WebApi, steam_get};

/// How a Sync went, when nothing failed in the app itself. Every outcome but
/// `Synced` leaves the stored data as it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum SyncOutcome {
    /// The Library now holds every Steam Game the account owns. `at` is
    /// when, in seconds since the Unix epoch.
    Synced { at: u64 },
    /// No Steam Web API key is stored.
    NoKey,
    /// No Steam Account is known: none chosen and none on this computer.
    NoSteamAccount,
    /// Steam refuses the stored key.
    KeyRejected,
    /// Steam does not reveal the account's games: its game details are private.
    GameDetailsPrivate,
    /// Steam could not be asked or gave no usable answer.
    SteamUnreachable,
}

/// How long after the app starts the first automatic Sync begins: long
/// enough that the window shows the stored Library first.
pub(crate) const FIRST_AUTOMATIC_SYNC_AFTER: Duration = Duration::from_secs(5);

/// How long after one automatic Sync ends the next begins.
pub(crate) const AUTOMATIC_SYNC_EVERY: Duration = Duration::from_secs(30 * 60);

/// What happens in a round of the automatic Syncs, as it happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum AutomaticSync {
    /// No Steam Web API key is stored, so no Sync begins this round.
    Skipped,
    /// A Sync begins.
    Started,
    /// The Sync ended with this outcome.
    Finished(SyncOutcome),
    /// The Sync failed in the app itself and changed nothing.
    Failed { reason: Error },
}

/// A Steam Game the account owns, as Steam reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnedGame {
    pub app_id: u32,
    /// `None` when Steam sends no name.
    pub title: Option<String>,
    /// Playtime in minutes.
    pub playtime_minutes: u32,
    /// Last Played in seconds since the Unix epoch; `None` for never.
    pub last_played: Option<u64>,
}

#[derive(Deserialize)]
struct OwnedGamesAnswer {
    response: OwnedGamesResponse,
}

#[derive(Deserialize)]
struct OwnedGamesResponse {
    /// Missing when Steam does not reveal the account's games.
    game_count: Option<u32>,
    #[serde(default)]
    games: Vec<OwnedGameEntry>,
}

#[derive(Deserialize)]
struct OwnedGameEntry {
    appid: u32,
    name: Option<String>,
    #[serde(default)]
    playtime_forever: u32,
    /// 0 when the game was never played.
    #[serde(default)]
    rtime_last_played: u32,
}

/// Every Steam Game the account `steam_id` owns, Installed or not, with
/// Playtime and Last Played, asked with `key`. Free games count once played.
/// `Err` is the outcome of a Sync that failed this way.
pub(crate) fn owned_games(api: &dyn WebApi, key: &str, steam_id: &str) -> Result<Vec<OwnedGame>, SyncOutcome> {
    let params = [("steamid", steam_id), ("include_appinfo", "1"), ("include_played_free_games", "1"), ("format", "json")];
    let answer = steam_get(api, "IPlayerService/GetOwnedGames/v1", Some(key), &params)
        .map_err(|_| SyncOutcome::SteamUnreachable)?;
    match answer.status {
        401 | 403 => return Err(SyncOutcome::KeyRejected),
        200 => {}
        _ => return Err(SyncOutcome::SteamUnreachable),
    }
    // Anything but Steam's JSON, such as a network's sign-in page, is no answer from Steam.
    let answer: OwnedGamesAnswer = serde_json::from_slice(&answer.body).map_err(|_| SyncOutcome::SteamUnreachable)?;
    // Steam answers an account whose game details are private with an
    // empty response, an account without games with a count of 0.
    if answer.response.game_count.is_none() {
        return Err(SyncOutcome::GameDetailsPrivate);
    }
    Ok(answer
        .response
        .games
        .into_iter()
        .map(|game| OwnedGame {
            app_id: game.appid,
            title: game.name.filter(|name| !name.trim().is_empty()),
            playtime_minutes: game.playtime_forever,
            last_played: (game.rtime_last_played > 0).then_some(u64::from(game.rtime_last_played)),
        })
        .collect())
}
