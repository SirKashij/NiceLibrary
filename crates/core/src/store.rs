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
];

/// The setting that names the Steam Account whose Games the Library shows.
const STEAM_ACCOUNT: &str = "steam_account";

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

    /// The Steam Account whose Games the Library shows, once one is known.
    pub(crate) fn steam_account(&self) -> Result<Option<String>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = statement.query_map([STEAM_ACCOUNT], |row| row.get(0))?;
        Ok(rows.next().transpose()?)
    }

    /// Records which Steam apps are Installed for `account`, all at once:
    /// the given apps are Installed with their titles, every other Steam Game
    /// of the account is not. `account` becomes the account the Library shows.
    pub(crate) fn record_installed_steam_games(&self, account: &str, apps: &[InstalledApp]) -> Result<(), Error> {
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![STEAM_ACCOUNT, account],
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

    /// The Steam Games of `account`.
    pub(crate) fn steam_games(&self, account: &str) -> Result<Vec<Game>, Error> {
        let connection = self.connection();
        let mut statement =
            connection.prepare("SELECT app_id, title, installed FROM steam_games WHERE steam_account = ?1")?;
        let games = statement
            .query_map([account], |row| Ok(Game::steam(row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<_, _>>()?;
        Ok(games)
    }
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
