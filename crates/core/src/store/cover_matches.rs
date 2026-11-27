//! The Cover Matches in the store: one row per Local Game that borrows the
//! Cover of a Steam title (see `crate::cover_match`).

use rusqlite::{OptionalExtension, params};

use super::Store;
use crate::Error;
use crate::steam_app_list::SteamTitle;

impl Store {
    /// The Steam title the Local Game numbered `local_game` is matched with, if any.
    pub(crate) fn cover_match(&self, local_game: i64) -> Result<Option<SteamTitle>, Error> {
        let title = self
            .connection()
            .query_row("SELECT app_id, name FROM cover_matches WHERE local_game = ?1", [local_game], |row| {
                Ok(SteamTitle { app_id: row.get(0)?, name: row.get(1)? })
            })
            .optional()?;
        Ok(title)
    }

    /// Matches the Local Game numbered `local_game` with `title`, in place of
    /// any match before; `None` removes its match.
    pub(crate) fn set_cover_match(&self, local_game: i64, title: Option<&SteamTitle>) -> Result<(), Error> {
        let connection = self.connection();
        match title {
            Some(title) => connection.execute(
                "INSERT INTO cover_matches (local_game, app_id, name) VALUES (?1, ?2, ?3)
                 ON CONFLICT (local_game) DO UPDATE SET app_id = excluded.app_id, name = excluded.name",
                params![local_game, title.app_id, title.name],
            )?,
            None => connection.execute("DELETE FROM cover_matches WHERE local_game = ?1", [local_game])?,
        };
        Ok(())
    }
}
