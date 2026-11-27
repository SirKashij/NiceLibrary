//! The stored Steam App List (see `crate::steam_app_list`).

use rusqlite::{OptionalExtension, Transaction, params};

use super::{Store, seconds_column};
use crate::Error;
use crate::steam_app_list::{SteamAppList, SteamTitle, searchable};

/// The setting that holds the app id the next page of an unfinished fetch
/// begins after. Absent while no fetch is unfinished.
const RESUME_AFTER: &str = "steam_app_list_resume_after";

/// The setting that holds when the last fetch that ran to its end ended.
const FETCHED_AT: &str = "steam_app_list_fetched_at";

impl Store {
    /// The app id an unfinished fetch resumes after; `None` when the next
    /// fetch starts from the beginning.
    pub(crate) fn steam_app_list_resume_after(&self) -> Result<Option<u32>, Error> {
        Ok(self.setting(RESUME_AFTER)?.and_then(|after| after.parse().ok()))
    }

    /// Stores one page of the Steam App List, all at once: its titles, in
    /// place of what was stored under their app ids, and where the next page
    /// begins, `None` after the last. `fetched_at` is when the fetch ended,
    /// once it has.
    pub(crate) fn record_steam_app_list_page(
        &self,
        titles: &[SteamTitle],
        resume_after: Option<u32>,
        fetched_at: Option<u64>,
    ) -> Result<(), Error> {
        let fetched_at = fetched_at.map(seconds_column).transpose()?;
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        {
            let mut insert = transaction.prepare(
                "INSERT INTO steam_app_list (app_id, name, search) VALUES (?1, ?2, ?3)
                 ON CONFLICT (app_id) DO UPDATE SET name = excluded.name, search = excluded.search",
            )?;
            for title in titles {
                insert.execute(params![title.app_id, title.name, searchable(&title.name)])?;
            }
        }
        match resume_after {
            Some(after) => set(&transaction, RESUME_AFTER, &after.to_string())?,
            None => {
                transaction.execute("DELETE FROM settings WHERE key = ?1", [RESUME_AFTER])?;
            }
        }
        if let Some(at) = fetched_at {
            set(&transaction, FETCHED_AT, &at.to_string())?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// What is stored of the Steam App List.
    pub(crate) fn steam_app_list(&self) -> Result<SteamAppList, Error> {
        let titles: u32 = self.connection().query_row("SELECT COUNT(*) FROM steam_app_list", [], |row| row.get(0))?;
        Ok(SteamAppList {
            titles,
            fetched_at: self.setting(FETCHED_AT)?.and_then(|at| at.parse().ok()),
            unfinished: self.setting(RESUME_AFTER)?.is_some(),
        })
    }

    /// Every stored title's app id with its name written as [`searchable`]
    /// writes it, in the order of the app ids.
    pub(crate) fn searchable_steam_titles(&self) -> Result<Vec<(u32, String)>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT app_id, search FROM steam_app_list ORDER BY app_id")?;
        let titles = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<Result<_, _>>()?;
        Ok(titles)
    }

    /// The stored titles with the app ids `app_ids`, in that order; an app id
    /// that is not stored is left out.
    pub(crate) fn steam_titles(&self, app_ids: &[u32]) -> Result<Vec<SteamTitle>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare_cached("SELECT name FROM steam_app_list WHERE app_id = ?1")?;
        let mut titles = Vec::with_capacity(app_ids.len());
        for &app_id in app_ids {
            if let Some(name) = statement.query_row([app_id], |row| row.get(0)).optional()? {
                titles.push(SteamTitle { app_id, name });
            }
        }
        Ok(titles)
    }
}

/// Sets the setting `key` to `value` within `transaction`.
fn set(transaction: &Transaction<'_>, key: &str, value: &str) -> Result<(), Error> {
    transaction.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
