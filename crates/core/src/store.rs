//! Store: the one SQLite database in the user profile.
//!
//! The schema grows by migrations only. Each entry of [`MIGRATIONS`] runs once,
//! in order, and the database records how many have run (`user_version`), so
//! a database from any earlier version is brought up to date on open. Every
//! Steam-derived row carries the Steam Account it belongs to.

mod hidden_games;
mod steam_app_list;

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, params};

use crate::collections::{self, CollectionNaming};
use crate::local_games::StoredLocalGame;
use crate::sessions::Played;
use crate::steam::InstalledApp;
use crate::sync::OwnedGame;
use crate::theme::Theme;
use crate::{Error, Game, LaunchOptions, Status};

mod achievements;

/// The schema changes, oldest first. Never edit or reorder a released entry;
/// append a new one instead.
const MIGRATIONS: &[&str] = &[
    // 1: Installed Steam Games and the settings.
    "CREATE TABLE settings (
         key   TEXT PRIMARY KEY,
         value TEXT NOT NULL
     );
     CREATE TABLE steam_games (
         steam_account TEXT    NOT NULL,
         app_id        INTEGER NOT NULL,
         title         TEXT    NOT NULL,
         installed     INTEGER NOT NULL,
         PRIMARY KEY (steam_account, app_id)
     );",
    // 2: Steam's figures from a Sync, and when the last one succeeded.
    "ALTER TABLE steam_games ADD COLUMN owned INTEGER NOT NULL DEFAULT 0;
     ALTER TABLE steam_games ADD COLUMN playtime_minutes INTEGER;
     ALTER TABLE steam_games ADD COLUMN last_played INTEGER;
     CREATE TABLE steam_syncs (
         steam_account TEXT    PRIMARY KEY,
         synced_at     INTEGER NOT NULL
     );",
    // 3: Local Games, each with the path of its executable as chosen.
    "CREATE TABLE local_games (
         id         INTEGER PRIMARY KEY AUTOINCREMENT,
         title      TEXT    NOT NULL,
         executable TEXT    NOT NULL UNIQUE
     );",
    // 4: Collections and which Games are in them; they belong to the person, not to a Steam Account.
    "CREATE TABLE collections (
         id   INTEGER PRIMARY KEY,
         name TEXT    NOT NULL
     );
     CREATE TABLE collection_games (
         collection_id INTEGER NOT NULL REFERENCES collections (id),
         game_id       TEXT    NOT NULL,
         PRIMARY KEY (collection_id, game_id)
     );",
    // 5: The Steam App List. Steam's, not a Steam Account's, so its rows carry none.
    "CREATE TABLE steam_app_list (
         app_id INTEGER PRIMARY KEY,
         name   TEXT    NOT NULL,
         search TEXT    NOT NULL
     );",
    // 6: The Status the person set on a Game, by the Game's id, which is unique across Sources.
    "CREATE TABLE game_statuses (
         game_id TEXT PRIMARY KEY,
         status  TEXT NOT NULL
     );",
    // 7: Sessions of Local Games, each from its first to its last look, in seconds since 1970.
    "CREATE TABLE local_game_sessions (
         id         INTEGER PRIMARY KEY AUTOINCREMENT,
         local_game INTEGER NOT NULL REFERENCES local_games (id) ON DELETE CASCADE,
         started_at INTEGER NOT NULL,
         ended_at   INTEGER NOT NULL
     );
     CREATE INDEX local_game_sessions_by_game ON local_game_sessions (local_game);",
    // 8: Achievements: their definitions per Steam Game, which belong to no
    // Steam Account, the account's state of each, when they were last
    // fetched, and how many Web API calls were made each day.
    "CREATE TABLE steam_achievement_definitions (
         app_id      INTEGER NOT NULL,
         api_name    TEXT    NOT NULL,
         position    INTEGER NOT NULL,
         name        TEXT    NOT NULL,
         description TEXT,
         secret      INTEGER NOT NULL,
         icon        TEXT,
         icon_locked TEXT,
         PRIMARY KEY (app_id, api_name)
     );
     CREATE TABLE steam_achievements (
         steam_account TEXT    NOT NULL,
         app_id        INTEGER NOT NULL,
         api_name      TEXT    NOT NULL,
         unlocked      INTEGER NOT NULL,
         unlocked_at   INTEGER,
         PRIMARY KEY (steam_account, app_id, api_name)
     );
     CREATE TABLE steam_achievement_fetches (
         steam_account    TEXT    NOT NULL,
         app_id           INTEGER NOT NULL,
         fetched_at       INTEGER NOT NULL,
         playtime_minutes INTEGER,
         last_played      INTEGER,
         PRIMARY KEY (steam_account, app_id)
     );
     CREATE TABLE steam_web_api_calls (
         day   INTEGER PRIMARY KEY,
         calls INTEGER NOT NULL
     );",
    // 9: How each Local Game is launched: its Launch Arguments as typed, and
    // its Working Directory, NULL for the executable's folder.
    "ALTER TABLE local_games ADD COLUMN launch_arguments TEXT NOT NULL DEFAULT '';
     ALTER TABLE local_games ADD COLUMN working_directory TEXT;",
    // 10: Hidden Games; hiding belongs to the person, not to a Steam Account.
    "CREATE TABLE hidden_games (
         game_id TEXT PRIMARY KEY
     );",
];

/// The setting that names the Steam Account whose Games were read last, from
/// Steam's folders or by a Sync. The Library shows it while the user has
/// chosen no Steam Account.
const LAST_READ_STEAM_ACCOUNT: &str = "steam_account";

/// The setting that names the Steam Account the user chose.
const CHOSEN_STEAM_ACCOUNT: &str = "chosen_steam_account";

/// The setting that names the order the user chose for the Library.
const LIBRARY_SORT: &str = "library_sort";

/// The setting that holds the theme the user chose.
const THEME: &str = "theme";

/// The local store.
pub struct Store {
    connection: Mutex<Connection>,
}

impl Store {
    /// Opens the database at `path`, creating it and its folder if needed,
    /// and brings its schema up to date.
    ///
    /// A database written by a newer version of the app is refused rather
    /// than changed.
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder).map_err(|e| Error::new(format!("cannot create {}: {e}", folder.display())))?;
        }
        let mut connection = Connection::open(path)?;
        migrate(&mut connection)?;
        Ok(Self { connection: Mutex::new(connection) })
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        // A panic while holding the lock leaves SQLite itself consistent.
        self.connection.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The value of the setting `key`, once one is stored.
    pub(crate) fn setting(&self, key: &str) -> Result<Option<String>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = statement.query_map([key], |row| row.get(0))?;
        Ok(rows.next().transpose()?)
    }

    /// Stores `value` as the setting `key`, in place of any earlier one.
    pub(crate) fn set_setting(&self, key: &str, value: &str) -> Result<(), Error> {
        self.connection().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// The Steam Account whose Games were read last, from Steam's folders or
    /// by a Sync, once one is known.
    pub(crate) fn last_read_steam_account(&self) -> Result<Option<String>, Error> {
        self.setting(LAST_READ_STEAM_ACCOUNT)
    }

    /// The Steam Account the user chose, if they chose one.
    pub(crate) fn chosen_steam_account(&self) -> Result<Option<String>, Error> {
        self.setting(CHOSEN_STEAM_ACCOUNT)
    }

    /// Remembers `steam_id` as the Steam Account the user chose.
    pub(crate) fn choose_steam_account(&self, steam_id: &str) -> Result<(), Error> {
        self.connection().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![CHOSEN_STEAM_ACCOUNT, steam_id],
        )?;
        Ok(())
    }

    /// The name of the order the user chose for the Library, if they chose one.
    pub(crate) fn library_sort(&self) -> Result<Option<String>, Error> {
        self.setting(LIBRARY_SORT)
    }

    /// Remembers `name` as the order the user chose for the Library.
    pub(crate) fn choose_library_sort(&self, name: &str) -> Result<(), Error> {
        self.connection().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![LIBRARY_SORT, name],
        )?;
        Ok(())
    }

    /// The theme the user chose; [`Theme::System`] until they choose.
    pub(crate) fn theme(&self) -> Result<Theme, Error> {
        Ok(self.setting(THEME)?.map_or(Theme::System, |setting| Theme::from_setting(&setting)))
    }

    /// Remembers `theme` as the theme the user chose.
    pub(crate) fn choose_theme(&self, theme: Theme) -> Result<(), Error> {
        self.connection().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![THEME, theme.as_setting()],
        )?;
        Ok(())
    }

    /// Records which Steam apps are Installed for `account`, all at once:
    /// the given apps are Installed with their titles, every other Steam Game
    /// of the account is not. `account` becomes the account read last.
    pub(crate) fn record_installed_steam_games(&self, account: &str, apps: &[InstalledApp]) -> Result<(), Error> {
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![LAST_READ_STEAM_ACCOUNT, account],
        )?;
        transaction.execute("UPDATE steam_games SET installed = 0 WHERE steam_account = ?1", [account])?;
        for app in apps {
            transaction.execute(
                "INSERT INTO steam_games (steam_account, app_id, title, installed) VALUES (?1, ?2, ?3, 1)
                 ON CONFLICT (steam_account, app_id) DO UPDATE SET title = excluded.title, installed = 1",
                params![account, app.app_id, app.title],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Records the result of a Sync for `account`, all at once: the given
    /// games are owned, with Steam's titles, Playtime and Last Played; every
    /// other Steam Game of the account is not, and is gone unless it is
    /// Installed. A game Steam sends without a name keeps its stored title,
    /// or is titled by its app id, which reads the same in every language.
    /// `account` becomes the account read last, and `synced_at` the time of
    /// its last successful Sync.
    pub(crate) fn record_steam_sync(&self, account: &str, games: &[OwnedGame], synced_at: u64) -> Result<(), Error> {
        let synced_at = seconds_column(synced_at)?;
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![LAST_READ_STEAM_ACCOUNT, account],
        )?;
        transaction.execute("UPDATE steam_games SET owned = 0 WHERE steam_account = ?1", [account])?;
        for game in games {
            let last_played = game.last_played.map(seconds_column).transpose()?;
            transaction.execute(
                "INSERT INTO steam_games (steam_account, app_id, title, installed, owned, playtime_minutes, last_played)
                 VALUES (?1, ?2, COALESCE(?3, CAST(?2 AS TEXT)), 0, 1, ?4, ?5)
                 ON CONFLICT (steam_account, app_id) DO UPDATE SET
                     title = COALESCE(?3, title), owned = 1,
                     playtime_minutes = excluded.playtime_minutes, last_played = excluded.last_played",
                params![account, game.app_id, game.title, game.playtime_minutes, last_played],
            )?;
        }
        transaction.execute(
            "DELETE FROM steam_games WHERE steam_account = ?1 AND owned = 0 AND installed = 0",
            [account],
        )?;
        transaction.execute(
            "INSERT INTO steam_syncs (steam_account, synced_at) VALUES (?1, ?2)
             ON CONFLICT (steam_account) DO UPDATE SET synced_at = excluded.synced_at",
            params![account, synced_at],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// When the last successful Sync of `account` happened, in seconds since
    /// the Unix epoch.
    pub(crate) fn last_steam_sync(&self, account: &str) -> Result<Option<u64>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT synced_at FROM steam_syncs WHERE steam_account = ?1")?;
        let mut rows = statement.query_map([account], |row| row.get::<_, i64>(0))?;
        Ok(rows.next().transpose()?.and_then(|seconds| u64::try_from(seconds).ok()))
    }

    /// The Steam Games of `account`.
    pub(crate) fn steam_games(&self, account: &str) -> Result<Vec<Game>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare(
            "SELECT app_id, title, installed, playtime_minutes, last_played FROM steam_games WHERE steam_account = ?1",
        )?;
        let games = statement
            .query_map([account], |row| {
                let last_played: Option<i64> = row.get(4)?;
                Ok(Game {
                    playtime_minutes: row.get(3)?,
                    last_played: last_played.and_then(|seconds| u64::try_from(seconds).ok()),
                    ..Game::steam(row.get(0)?, row.get(1)?, row.get(2)?)
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(games)
    }

    /// The Status the person set on each Game, by the Game's id. A Game
    /// without one is Unplayed. No Sync touches them.
    pub(crate) fn statuses(&self) -> Result<HashMap<String, Status>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT game_id, status FROM game_statuses")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        let mut statuses = HashMap::new();
        for row in rows {
            let (game_id, status) = row?;
            // A Status this version does not know reads as Unplayed rather than failing the Library.
            if let Some(status) = Status::from_column(&status) {
                statuses.insert(game_id, status);
            }
        }
        Ok(statuses)
    }

    /// Remembers `status` as the Status of the Game `game_id`.
    pub(crate) fn set_status(&self, game_id: &str, status: Status) -> Result<(), Error> {
        self.connection().execute(
            "INSERT INTO game_statuses (game_id, status) VALUES (?1, ?2)
             ON CONFLICT (game_id) DO UPDATE SET status = excluded.status",
            params![game_id, status.as_column()],
        )?;
        Ok(())
    }
}

impl Store {
    /// Every Local Game, in the order they were added.
    pub(crate) fn local_games(&self) -> Result<Vec<StoredLocalGame>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare(&format!("{SELECT_LOCAL_GAMES} ORDER BY id"))?;
        let games = statement.query_map([], stored_local_game)?.collect::<Result<_, _>>()?;
        Ok(games)
    }

    /// The Local Game stored under `id`, if there is one.
    pub(crate) fn local_game(&self, id: i64) -> Result<Option<StoredLocalGame>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare(&format!("{SELECT_LOCAL_GAMES} WHERE id = ?1"))?;
        Ok(statement.query_map([id], stored_local_game)?.next().transpose()?)
    }

    /// Keeps `options` for the Local Game `id`; `false` when there is no
    /// such Game.
    pub(crate) fn set_launch_options(&self, id: i64, options: &LaunchOptions) -> Result<bool, Error> {
        let changed = self.connection().execute(
            "UPDATE local_games SET launch_arguments = ?2, working_directory = ?3 WHERE id = ?1",
            params![id, options.arguments, options.working_directory],
        )?;
        Ok(changed > 0)
    }

    /// Adds the Local Game `title` with its `executable` and answers its id.
    pub(crate) fn add_local_game(&self, title: &str, executable: &str) -> Result<i64, Error> {
        let connection = self.connection();
        connection.execute("INSERT INTO local_games (title, executable) VALUES (?1, ?2)", params![title, executable])?;
        Ok(connection.last_insert_rowid())
    }

    /// Every Collection, as its id and name.
    pub(crate) fn collections(&self) -> Result<Vec<(i64, String)>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT id, name FROM collections")?;
        let collections = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<Result<_, _>>()?;
        Ok(collections)
    }

    /// Which Games are in which Collection, as pairs of Collection id and Game id.
    pub(crate) fn collection_games(&self) -> Result<Vec<(i64, String)>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT collection_id, game_id FROM collection_games")?;
        let games = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<Result<_, _>>()?;
        Ok(games)
    }

    /// Gives the Collection `id` the name `name`, already trimmed and not
    /// empty, or creates a Collection with it when `id` is `None`. Refused
    /// when another Collection has the name in any case. Fails for an `id`
    /// that names no Collection.
    pub(crate) fn name_collection(&self, id: Option<i64>, name: &str) -> Result<CollectionNaming, Error> {
        let mut connection = self.connection();
        // Checked and written at once, so two namings cannot both take one name.
        let transaction = connection.transaction()?;
        let others: Vec<(i64, String)> = transaction
            .prepare("SELECT id, name FROM collections")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        if others.iter().any(|(other, other_name)| Some(*other) != id && collections::same_name(other_name, name)) {
            return Ok(CollectionNaming::NameTaken);
        }
        let id = match id {
            Some(id) => {
                if transaction.execute("UPDATE collections SET name = ?1 WHERE id = ?2", params![name, id])? == 0 {
                    return Err(Error::new(format!("no such collection: {id}")));
                }
                id
            }
            None => {
                transaction.execute("INSERT INTO collections (name) VALUES (?1)", [name])?;
                transaction.last_insert_rowid()
            }
        };
        transaction.commit()?;
        Ok(CollectionNaming::Named { id })
    }

    /// Deletes the Collection `id` and its record of which Games are in it,
    /// never a Game; nothing when there is no such Collection.
    pub(crate) fn delete_collection(&self, id: i64) -> Result<(), Error> {
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM collection_games WHERE collection_id = ?1", [id])?;
        transaction.execute("DELETE FROM collections WHERE id = ?1", [id])?;
        transaction.commit()?;
        Ok(())
    }
}

impl Store {
    /// Stores a Session of the Local Game `local_game` that began `at`, and
    /// ends there until [`Self::extend_session`] moves its end, and answers
    /// its id.
    pub(crate) fn begin_session(&self, local_game: i64, at: u64) -> Result<i64, Error> {
        let at = seconds_column(at)?;
        let connection = self.connection();
        connection.execute(
            "INSERT INTO local_game_sessions (local_game, started_at, ended_at) VALUES (?1, ?2, ?2)",
            params![local_game, at],
        )?;
        Ok(connection.last_insert_rowid())
    }

    /// Moves the end of the Session `session` to `at`, never before where it is.
    pub(crate) fn extend_session(&self, session: i64, at: u64) -> Result<(), Error> {
        self.connection().execute(
            "UPDATE local_game_sessions SET ended_at = MAX(ended_at, ?2) WHERE id = ?1",
            params![session, seconds_column(at)?],
        )?;
        Ok(())
    }

    /// What the Sessions of each Local Game that has any add up to, by its id.
    pub(crate) fn local_games_played(&self) -> Result<BTreeMap<i64, Played>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare(
            "SELECT local_game, SUM(ended_at - started_at), MAX(ended_at) FROM local_game_sessions GROUP BY local_game",
        )?;
        let played = statement
            .query_map([], |row| {
                let seconds: i64 = row.get(1)?;
                let last_ended: i64 = row.get(2)?;
                Ok((
                    row.get(0)?,
                    Played {
                        seconds: u64::try_from(seconds).unwrap_or(0),
                        last_ended: u64::try_from(last_ended).unwrap_or(0),
                    },
                ))
            })?
            .collect::<Result<_, _>>()?;
        Ok(played)
    }
}

/// The columns [`stored_local_game`] reads, from every Local Game.
const SELECT_LOCAL_GAMES: &str =
    "SELECT id, title, executable, launch_arguments, working_directory FROM local_games";

fn stored_local_game(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredLocalGame> {
    Ok(StoredLocalGame {
        id: row.get(0)?,
        title: row.get(1)?,
        executable: row.get(2)?,
        launch_options: LaunchOptions { arguments: row.get(3)?, working_directory: row.get(4)? },
    })
}

/// A time in seconds since the Unix epoch, as SQLite stores it.
fn seconds_column(seconds: u64) -> Result<i64, Error> {
    i64::try_from(seconds).map_err(|_| Error::new(format!("a time out of range: {seconds} seconds since 1970")))
}

fn migrate(connection: &mut Connection) -> Result<(), Error> {
    // How many migrations have already run on this database.
    let applied: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let applied = usize::try_from(applied).unwrap_or(usize::MAX);
    if applied > MIGRATIONS.len() {
        return Err(Error::new(format!(
            "the database was written by a newer version of NiceLibrary (schema {applied}, this version knows {})",
            MIGRATIONS.len()
        )));
    }
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(applied) {
        let transaction = connection.transaction()?;
        transaction.execute_batch(migration)?;
        transaction.pragma_update(None, "user_version", i64::try_from(index + 1).expect("few migrations"))?;
        transaction.commit()?;
    }
    Ok(())
}
