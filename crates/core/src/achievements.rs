//! Achievements of Steam Games: fetched from Steam's Web API with a Sync,
//! spread over Syncs so that a large Library stays well within Steam's
//! daily limit of Web API calls.

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::store::Store;
use crate::steam_web_api::{WebApi, steam_get};

/// The Achievements of one Game, for its detail page.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum GameAchievements {
    /// A Local Game; Local Games have no Achievements.
    LocalGame,
    /// No Sync has fetched the Steam Game's Achievements yet.
    NotFetchedYet,
    /// The Steam Game defines no Achievements.
    NoAchievements,
    /// Every Achievement of the Steam Game, unlocked ones first, the most
    /// recently unlocked first, then the locked ones in Steam's order.
    Listed { unlocked: u32, total: u32, achievements: Vec<Achievement> },
}

/// One Achievement of a Steam Game, with whether the Steam Account unlocked it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Achievement {
    /// Steam's name for it, unique within the Game.
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub unlocked: bool,
    /// When it was unlocked, in seconds since the Unix epoch; `None` while
    /// locked, or when Steam does not know when.
    pub unlocked_at: Option<u64>,
    /// The icon as the Achievement looks now, in colour when unlocked, grey
    /// when locked: Steam's name for the image (see
    /// [`crate::Core::achievement_icon`]); `None` when Steam has none.
    pub icon: Option<String>,
    /// A Secret Achievement: the interface conceals its name and description
    /// while it is locked, unless the person reveals them.
    pub secret: bool,
    /// Its Rarity: the percent of all players worldwide who have unlocked
    /// it, as the last Sync fetched it; `None` when Steam reports none.
    pub rarity: Option<f64>,
}

/// At most this many Steam Games have their Achievements fetched in one Sync;
/// the others follow in the next Syncs.
const GAMES_PER_SYNC: usize = 50;

/// At most this many Web API calls in one day (UTC), all Syncs together.
/// Steam allows 100,000 a day for a key; this leaves most of that unused.
pub(crate) const WEB_API_CALLS_PER_DAY: u32 = 10_000;

/// Seconds in a day: the days Web API calls are counted in.
const DAY: u64 = 86_400;

/// A Steam Game whose Achievements are due: never fetched, or played since.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DueGame {
    pub app_id: u32,
    pub playtime_minutes: Option<u32>,
    pub last_played: Option<u64>,
}

/// The state of one Achievement for the Steam Account, as Steam reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerAchievement {
    pub api_name: String,
    pub unlocked: bool,
    pub unlocked_at: Option<u64>,
}

/// The definition of one Achievement of a Steam Game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Definition {
    pub api_name: String,
    pub name: String,
    pub description: Option<String>,
    /// A Secret Achievement.
    pub secret: bool,
    pub icon: Option<String>,
    pub icon_locked: Option<String>,
}

/// What one Sync fetched for one Steam Game.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Fetched {
    pub game: DueGame,
    /// Empty for a game without Achievements.
    pub achievements: Vec<PlayerAchievement>,
    /// New definitions, when the stored ones were missing or incomplete.
    pub definitions: Option<Vec<Definition>>,
    /// The Rarity of each Achievement Steam reports one for, by Steam's name
    /// for it; these replace the game's stored ones.
    pub rarities: Vec<(String, f64)>,
}

/// Why fetching stops for this Sync; the games not fetched stay due.
struct Stop;

/// Fetches the Achievements of the Steam Games of `account` that are due,
/// most recently played first, within this Sync's and this day's share of
/// Web API calls, and stores them. `calls_before` is how many calls this
/// Sync has already made. Anything Steam does not answer leaves the rest
/// for the next Sync and changes no stored Achievements.
pub(crate) fn fetch(
    api: &dyn WebApi,
    store: &Store,
    key: &str,
    account: &str,
    now: u64,
    calls_before: u32,
) -> Result<(), Error> {
    let day = now / DAY;
    let mut calls_today = store.web_api_calls(day)? + calls_before;
    let mut calls = calls_before;
    let mut fetched = Vec::new();
    for game in store.games_with_achievements_due(account, GAMES_PER_SYNC)? {
        // A game can need three calls: the account's Achievements, their
        // definitions and their Rarity.
        if calls_today + 3 > WEB_API_CALLS_PER_DAY {
            break;
        }
        calls += 1;
        calls_today += 1;
        let Ok(achievements) = player_achievements(api, key, account, game.app_id) else {
            break;
        };
        let known = store.achievement_definitions_known(game.app_id)?;
        let definitions = if achievements.iter().all(|a| known.contains(&a.api_name)) {
            None
        } else {
            calls += 1;
            calls_today += 1;
            let Ok(definitions) = definitions(api, key, game.app_id) else {
                break;
            };
            Some(definitions)
        };
        let rarities = if achievements.is_empty() {
            Vec::new()
        } else {
            calls += 1;
            calls_today += 1;
            let Ok(rarities) = rarities(api, game.app_id) else {
                break;
            };
            rarities
        };
        fetched.push(Fetched { game, achievements, definitions, rarities });
    }
    store.record_achievements(account, &fetched, now, day, calls)
}

#[derive(Deserialize)]
struct PlayerAnswer {
    playerstats: PlayerStats,
}

#[derive(Deserialize)]
struct PlayerStats {
    #[serde(default)]
    achievements: Vec<PlayerEntry>,
}

#[derive(Deserialize)]
struct PlayerEntry {
    apiname: String,
    #[serde(default)]
    achieved: u8,
    #[serde(default)]
    unlocktime: u64,
}

/// The Achievements of the Steam Game `app_id` for the account `steam_id`;
/// empty for a game without them.
fn player_achievements(api: &dyn WebApi, key: &str, steam_id: &str, app_id: u32) -> Result<Vec<PlayerAchievement>, Stop> {
    let app_id = app_id.to_string();
    let params = [("steamid", steam_id), ("appid", app_id.as_str()), ("format", "json")];
    let answer = steam_get(api, "ISteamUserStats/GetPlayerAchievements/v1", Some(key), &params).map_err(|_| Stop)?;
    let answer: PlayerAnswer = match answer.status {
        // Steam answers a game without stats this way.
        200 | 400 => serde_json::from_slice(&answer.body).map_err(|_| Stop)?,
        _ => return Err(Stop),
    };
    Ok(answer
        .playerstats
        .achievements
        .into_iter()
        .map(|entry| PlayerAchievement {
            api_name: entry.apiname,
            unlocked: entry.achieved != 0,
            unlocked_at: (entry.achieved != 0 && entry.unlocktime > 0).then_some(entry.unlocktime),
        })
        .collect())
}

#[derive(Deserialize)]
struct SchemaAnswer {
    game: SchemaGame,
}

#[derive(Deserialize)]
struct SchemaGame {
    #[serde(rename = "availableGameStats")]
    available_game_stats: Option<SchemaStats>,
}

#[derive(Deserialize)]
struct SchemaStats {
    #[serde(default)]
    achievements: Vec<SchemaEntry>,
}

#[derive(Deserialize)]
struct SchemaEntry {
    name: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    hidden: u8,
    icon: Option<String>,
    icongray: Option<String>,
}

/// The definitions of the Achievements of the Steam Game `app_id`, in Steam's order.
fn definitions(api: &dyn WebApi, key: &str, app_id: u32) -> Result<Vec<Definition>, Stop> {
    let app_id = app_id.to_string();
    let params = [("appid", app_id.as_str()), ("format", "json")];
    let answer = steam_get(api, "ISteamUserStats/GetSchemaForGame/v2", Some(key), &params).map_err(|_| Stop)?;
    if answer.status != 200 {
        return Err(Stop);
    }
    let answer: SchemaAnswer = serde_json::from_slice(&answer.body).map_err(|_| Stop)?;
    Ok(answer
        .game
        .available_game_stats
        .map(|stats| stats.achievements)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| Definition {
            name: entry.display_name.filter(|name| !name.trim().is_empty()).unwrap_or_else(|| entry.name.clone()),
            api_name: entry.name,
            description: entry.description.filter(|description| !description.trim().is_empty()),
            secret: entry.hidden != 0,
            icon: entry.icon.as_deref().and_then(icon_name),
            icon_locked: entry.icongray.as_deref().and_then(icon_name),
        })
        .collect())
}

#[derive(Deserialize)]
struct RarityAnswer {
    achievementpercentages: RarityList,
}

#[derive(Deserialize)]
struct RarityList {
    #[serde(default)]
    achievements: Vec<RarityEntry>,
}

#[derive(Deserialize)]
struct RarityEntry {
    name: String,
    percent: serde_json::Value,
}

/// The Rarity of the Achievements of the Steam Game `app_id`, by Steam's
/// name for each. Steam tells it to anyone, so the key is not sent. When
/// Steam refuses to tell it, no Rarity is known; when it does not answer,
/// fetching stops for this Sync.
fn rarities(api: &dyn WebApi, app_id: u32) -> Result<Vec<(String, f64)>, Stop> {
    let app_id = app_id.to_string();
    let params = [("gameid", app_id.as_str()), ("format", "json")];
    let answer =
        steam_get(api, "ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2", None, &params).map_err(|_| Stop)?;
    match answer.status {
        200 => {}
        400..=499 => return Ok(Vec::new()),
        _ => return Err(Stop),
    }
    let answer: RarityAnswer = serde_json::from_slice(&answer.body).map_err(|_| Stop)?;
    Ok(answer
        .achievementpercentages
        .achievements
        .into_iter()
        .filter_map(|entry| Some((entry.name, percent(&entry.percent)?)))
        .collect())
}

/// A percent as Steam writes it, as text or as a number; `None` for
/// anything that is no percent.
fn percent(value: &serde_json::Value) -> Option<f64> {
    let percent = match value {
        serde_json::Value::String(text) => text.trim().parse().ok()?,
        value => value.as_f64()?,
    };
    (0.0..=100.0).contains(&percent).then_some(percent)
}

/// Steam's name for the image at `url`: the file name without `.jpg`, if it
/// is one Steam gives its Achievement icons.
fn icon_name(url: &str) -> Option<String> {
    let name = url.rsplit('/').next()?.strip_suffix(".jpg")?;
    is_icon_name(name).then(|| name.to_owned())
}

/// Whether `name` has the form of Steam's name for an Achievement icon: a
/// hash in hexadecimal digits. Nothing else may reach a URL or a file name.
pub(crate) fn is_icon_name(name: &str) -> bool {
    (1..=64).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Where Steam serves the Achievement icon `icon` of the Steam Game `app_id`.
pub(crate) fn icon_url(app_id: u32, icon: &str) -> String {
    format!("https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/{app_id}/{icon}.jpg")
}
