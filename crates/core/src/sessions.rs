//! Tracker: looks at the running processes again and again while the app
//! runs, matches them to the Local Games and the Installed Steam Games and
//! records their Sessions. A Local Game runs when its executable does, a
//! Steam Game when any executable in its install folder does.
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

/// A Game whose Sessions are recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Tracked {
    /// The Local Game stored under this id.
    Local(i64),
    /// The Steam Game with this app id.
    Steam(u32),
}

impl Tracked {
    /// Its Game id (see [`crate::Game::id`]).
    pub(crate) fn game_id(self) -> String {
        match self {
            Self::Local(id) => format!("local:{id}"),
            Self::Steam(app_id) => format!("steam:{app_id}"),
        }
    }
}

/// The Sessions that run, as the tracking knows them: the stored Session of
/// each Game whose process ran at the last look, by the Game.
pub(crate) type OpenSessions = BTreeMap<Tracked, i64>;

/// The app ids of the Installed Steam Games of the Steam installation in
/// `steam_folder` that have an executable in their install folder among
/// `running`, by whatever spelling of its path.
///
/// Steam's library folders are only read when a process runs from some
/// `steamapps/common` folder, so most looks touch no Steam file.
pub(crate) fn running_steam_games(steam_folder: &Path, running: &[PathBuf]) -> BTreeSet<u32> {
    let candidates: Vec<&PathBuf> = running.iter().filter(|process| in_a_steam_install_folder(process)).collect();
    if candidates.is_empty() {
        return BTreeSet::new();
    }
    let Some(apps) = crate::steam::installed_apps(steam_folder) else {
        return BTreeSet::new();
    };
    let folders: Vec<(u32, PathBuf)> = apps
        .into_iter()
        .filter_map(|app| {
            let folder = app.install_folder?;
            Some((app.app_id, fs::canonicalize(&folder).unwrap_or(folder)))
        })
        .collect();
    let mut found = BTreeSet::new();
    for process in candidates {
        let process = fs::canonicalize(process).unwrap_or_else(|_| process.clone());
        for (app_id, folder) in &folders {
            if process.starts_with(folder) {
                found.insert(*app_id);
            }
        }
    }
    found
}

/// Whether `path` lies below a `steamapps/common` folder, whatever the case
/// of its names: where Steam installs Games.
fn in_a_steam_install_folder(path: &Path) -> bool {
    let names: Vec<String> =
        path.components().map(|part| part.as_os_str().to_string_lossy().to_lowercase()).collect();
    names.windows(3).any(|names| names[0] == "steamapps" && names[1] == "common")
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
