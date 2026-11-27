//! Statistics: figures over the Library, for the Statistics page. They are
//! worked out from the Library as it shows, so a Hidden Game counts in none
//! of them. The Playtime per week comes from the recorded Sessions alone,
//! since Steam's figure says nothing about when a Game was played.

use std::collections::HashSet;

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
    /// The Playtime of each of the last [`WEEKS`] weeks, oldest first, from
    /// the recorded Sessions.
    pub weeks: Vec<PlayedWeek>,
    /// The [`MOST_PLAYED`] Games with the most Playtime, most played first.
    pub most_played: Vec<Game>,
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

/// How many weeks the Statistics show the Playtime of.
pub(crate) const WEEKS: u64 = 8;
/// How many Games the Statistics name as the most played.
pub(crate) const MOST_PLAYED: usize = 5;
/// A week, in seconds.
const WEEK: u64 = 7 * 24 * 60 * 60;

/// The Playtime of one week, from the Sessions recorded in it. A week is
/// the seven days up to a moment, so the latest one ends now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayedWeek {
    /// Where the week starts, in seconds since the Unix epoch.
    pub starts_at: u64,
    /// Where the week ends, in seconds since the Unix epoch; the next week
    /// starts there.
    pub ends_at: u64,
    /// How long Sessions ran within the week, in whole minutes rounded down.
    /// A Session across the end of a week counts in each with its share.
    pub minutes: u32,
}

/// One recorded Session of the Game `game_id`, from `started_at` to
/// `ended_at`, in seconds since the Unix epoch.
pub(crate) struct Recorded {
    pub game_id: String,
    pub started_at: u64,
    pub ended_at: u64,
}

/// The Statistics of `games`, the Library as it shows, `now`. `achievements`
/// answers how many of a Game's Achievements are unlocked and how many it
/// has, for a Game whose Achievements are known. `sessions` are the recorded
/// Sessions; only those of a Game in `games` count.
pub(crate) fn of(
    games: &[Game],
    achievements: impl Fn(&Game) -> Option<(u32, u32)>,
    sessions: &[Recorded],
    now: u64,
) -> Statistics {
    let mut statistics = Statistics {
        games: 0,
        playtime_minutes: 0,
        achievements_unlocked: 0,
        achievements_total: 0,
        achievement_share: None,
        complete_games: 0,
        games_by_status: GamesByStatus::default(),
        weeks: weeks(games, sessions, now),
        most_played: most_played(games),
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

/// Where the oldest of the weeks up to `now` starts; a Session that ended
/// before then counts in none of them.
pub(crate) fn first_week_start(now: u64) -> u64 {
    now.saturating_sub(WEEKS * WEEK)
}

/// The last [`WEEKS`] weeks up to `now`, oldest first, with how long the
/// `sessions` of `games` ran within each.
fn weeks(games: &[Game], sessions: &[Recorded], now: u64) -> Vec<PlayedWeek> {
    let shown: HashSet<&str> = games.iter().map(|game| game.id.as_str()).collect();
    let sessions: Vec<&Recorded> = sessions.iter().filter(|session| shown.contains(session.game_id.as_str())).collect();
    (1..=WEEKS)
        .rev()
        .map(|ago| {
            let starts_at = now.saturating_sub(ago * WEEK);
            let ends_at = now.saturating_sub((ago - 1) * WEEK);
            let seconds: u64 = sessions
                .iter()
                .map(|session| session.ended_at.min(ends_at).saturating_sub(session.started_at.max(starts_at)))
                .sum();
            PlayedWeek { starts_at, ends_at, minutes: u32::try_from(seconds / 60).unwrap_or(u32::MAX) }
        })
        .collect()
}

/// The [`MOST_PLAYED`] Games of `games` with the most Playtime, most played
/// first, those played as long as each other by title. A Game not played at
/// all is none of them.
fn most_played(games: &[Game]) -> Vec<Game> {
    let mut played: Vec<&Game> = games.iter().filter(|game| game.playtime_minutes.unwrap_or(0) > 0).collect();
    played.sort_by(|a, b| {
        b.playtime_minutes
            .cmp(&a.playtime_minutes)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
    played.into_iter().take(MOST_PLAYED).cloned().collect()
}

/// `part` of `whole` in whole percent, rounded down; `None` of nothing.
fn share(part: u32, whole: u32) -> Option<u8> {
    if whole == 0 {
        return None;
    }
    u8::try_from(u64::from(part.min(whole)) * u64::from(COMPLETE) / u64::from(whole)).ok()
}
