//! Tracker: looks at the running processes again and again while the app
//! runs, matches them to the Local Games and records their Sessions.
//!
//! A Session begins at the first look that finds its Game's executable
//! running and is stored at once; every later look that still finds it moves
//! its end there. So a Session is always kept up to the last moment the
//! process was seen, even when the app is closed or crashes while the Game
//! runs. Focus plays no part: a Session lasts as long as the process runs.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::Error;
use crate::local_games::StoredLocalGame;

/// Port: the processes running on this computer.
///
/// Contract: answers the executable of every process running now that the
/// user may see, as an absolute path, the same path the process was started
/// from, once per process (several processes of one executable each appear).
/// A process whose executable cannot be read, such as a protected system
/// process, is left out. `Err` only when the list itself cannot be read,
/// described for a log or a developer.
pub trait ProcessList: Send + Sync {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String>;
}

/// What the tracking notices, as it happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Tracking {
    /// A Session of the Game `game_id` began: its process runs.
    Started {
        #[serde(rename = "gameId")]
        game_id: String,
    },
    /// The Session of the Game `game_id` ended at the look before: its
    /// process no longer runs.
    Ended {
        #[serde(rename = "gameId")]
        game_id: String,
    },
    /// This look failed and changed nothing; the Sessions that run go on.
    Failed { reason: Error },
}

/// One Session of a Game, as the detail page lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// When it began, in seconds since the Unix epoch.
    pub started_at: u64,
    /// How long it lasted, or has lasted so far while it runs, in seconds.
    pub seconds: u64,
    /// Whether its process still runs: the last look found it and tracking goes on.
    pub running: bool,
}

/// The id of the Local Game whose Game id is `game_id`, if it is one.
pub(crate) fn local_game_id(game_id: &str) -> Option<i64> {
    game_id.strip_prefix("local:")?.parse().ok()
}

/// How long after one look at the running processes the next one happens.
/// A Session may begin and end up to this much later than its process.
pub(crate) const LOOK_EVERY: Duration = Duration::from_secs(5);

/// The Sessions that run, as the tracking knows them: the stored Session of
/// each Local Game whose process ran at the last look, by the Game's id.
pub(crate) type OpenSessions = BTreeMap<i64, i64>;

/// The Game id of the Local Game stored under `id` (see [`crate::Game::id`]).
pub(crate) fn game_id(id: i64) -> String {
    format!("local:{id}")
}

/// The ids of the Local Games in `games` whose executable is one of
/// `running`, by whatever spelling of its path.
pub(crate) fn running_games(games: &[StoredLocalGame], running: &[PathBuf]) -> BTreeSet<i64> {
    let games: Vec<(&StoredLocalGame, Option<String>, Option<PathBuf>)> = games
        .iter()
        .map(|game| (game, file_name(Path::new(&game.executable)), fs::canonicalize(&game.executable).ok()))
        .collect();
    let mut found = BTreeSet::new();
    for process in running {
        // Only a process whose file is named like a Game's executable is looked at closer.
        let name = file_name(process);
        let mut canonical = None;
        for (game, game_name, game_canonical) in &games {
            if found.contains(&game.id) || name.is_none() || *game_name != name {
                continue;
            }
            let same = process.as_os_str() == Path::new(&game.executable).as_os_str()
                || game_canonical.as_ref().is_some_and(|game_canonical| {
                    canonical.get_or_insert_with(|| fs::canonicalize(process).ok()).as_ref() == Some(game_canonical)
                });
            if same {
                found.insert(game.id);
            }
        }
    }
    found
}

/// What the stored Sessions of one Local Game add up to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Played {
    /// The length of all its Sessions together, in seconds.
    pub seconds: u64,
    /// When its latest Session ended, in seconds since the Unix epoch.
    pub last_ended: u64,
}

impl Played {
    /// `game` with the Playtime and Last Played its Sessions give it: the
    /// Sessions' seconds added up, in whole minutes, and the end of the
    /// latest.
    pub(crate) fn into_game(self, game: crate::Game) -> crate::Game {
        crate::Game {
            playtime_minutes: Some(u32::try_from(self.seconds / 60).unwrap_or(u32::MAX)),
            last_played: Some(self.last_ended),
            ..game
        }
    }
}

/// The file name of `path` in lower case, to compare names whatever their case.
fn file_name(path: &Path) -> Option<String> {
    path.file_name().map(|name| name.to_string_lossy().to_lowercase())
}
