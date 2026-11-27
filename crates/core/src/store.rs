//! Store: the one SQLite database in the user profile.
//!
//! The schema grows by migrations only. Each entry of [`MIGRATIONS`] runs once,
//! in order, and the database records how many have run (`user_version`), so
//! a database from any earlier version is brought up to date on open. Every
//! Steam-derived row carries the Steam Account it belongs to.

use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, params};

use crate::steam::InstalledApp;
use crate::sync::OwnedGame;
use crate::{Error, Game};

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
];

/// The setting that names the Steam Account whose Games were read last, from
/// Steam's folders or by a Sync. The Library shows it while the user has
/// chosen no Steam Account.
const LAST_READ_STEAM_ACCOUNT: &str = "steam_account";

/// The setting that names the Steam Account the user chose.
const CHOSEN_STEAM_ACCOUNT: &str = "chosen_steam_account";

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
