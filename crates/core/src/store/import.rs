//! What the store writes only for an import (crate::import).

use std::collections::HashMap;

use rusqlite::params;

use super::{CHOSEN_STEAM_ACCOUNT, Error, LANGUAGE, LIBRARY_LAYOUT, LIBRARY_SORT, Store, THEME, seconds_column};
use crate::export::ExportFile;
use crate::{playtime_adjustment, window};

impl Store {
    /// Replaces everything that is the person's own with what `file` holds,
    /// all at once: Local Games and all they keep, Sessions of every Game,
    /// Statuses, Favorites, Hidden Games, Collections and the settings an
    /// Export File has. Steam Games, the Steam App List, the last Syncs and
    /// app-internal settings stay.
    ///
    /// Each Local Game gets a new number, never one used before
    /// (`AUTOINCREMENT`), and every Game id `local:<n>` of the file is
    /// rewritten to it. `before_commit` hears the new number of each Local
    /// Game by its id in the file, once everything is written but not yet
    /// kept; when it fails, nothing changes. Answers the numbers the Local
    /// Games here had before. `file` must have passed `crate::import::check`.
    pub(crate) fn replace_own_data(
        &self,
        file: &ExportFile,
        before_commit: impl FnOnce(&HashMap<&str, i64>) -> Result<(), Error>,
    ) -> Result<Vec<i64>, Error> {
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        let removed: Vec<i64> =
            transaction.prepare("SELECT id FROM local_games")?.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?;
        transaction.execute_batch(
            "DELETE FROM local_game_sessions;
             DELETE FROM steam_game_sessions;
             DELETE FROM local_game_playtime_adjustments;
             DELETE FROM cover_matches;
             DELETE FROM local_games;
             DELETE FROM game_statuses;
             DELETE FROM favorites;
             DELETE FROM hidden_games;
             DELETE FROM collection_games;
             DELETE FROM collections;",
        )?;

        let mut numbers = HashMap::new();
        for game in &file.local_games {
            transaction.execute(
                "INSERT INTO local_games (title, executable, launch_arguments, working_directory, manual_progress)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![game.title, game.executable, game.launch_arguments, game.working_directory, game.manual_progress],
            )?;
            let number = transaction.last_insert_rowid();
            numbers.insert(game.id.as_str(), number);
            if let Some(minutes) = playtime_adjustment::stored(game.playtime_adjustment_minutes) {
                transaction.execute(
                    "INSERT INTO local_game_playtime_adjustments (local_game, minutes) VALUES (?1, ?2)",
                    params![number, minutes],
                )?;
            }
            if let Some(matched) = &game.cover_match {
                transaction.execute(
                    "INSERT INTO cover_matches (local_game, app_id, name) VALUES (?1, ?2, ?3)",
                    params![number, matched.app_id, matched.name],
                )?;
            }
        }
        // The Game id here of a Game id in the file.
        let here = |id: &str| match numbers.get(id) {
            Some(number) => format!("local:{number}"),
            None => id.to_owned(),
        };

        for session in &file.sessions {
            let (started_at, ended_at) = (seconds_column(session.started_at)?, seconds_column(session.ended_at)?);
            if let Some(number) = numbers.get(session.game_id.as_str()) {
                transaction.execute(
                    "INSERT INTO local_game_sessions (local_game, started_at, ended_at) VALUES (?1, ?2, ?3)",
                    params![number, started_at, ended_at],
                )?;
            } else if let Some(app_id) = session.game_id.strip_prefix("steam:").and_then(|app| app.parse::<u32>().ok()) {
                transaction.execute(
                    "INSERT INTO steam_game_sessions (app_id, steam_account, started_at, ended_at) VALUES (?1, ?2, ?3, ?4)",
                    params![app_id, session.steam_account, started_at, ended_at],
                )?;
            }
        }
        for status in &file.statuses {
            transaction.execute(
                "INSERT OR REPLACE INTO game_statuses (game_id, status) VALUES (?1, ?2)",
                params![here(&status.game_id), status.status.as_column()],
            )?;
        }
        for id in &file.favorites {
            transaction.execute("INSERT OR IGNORE INTO favorites (game_id) VALUES (?1)", [here(id)])?;
        }
        for id in &file.hidden_games {
            transaction.execute("INSERT OR IGNORE INTO hidden_games (game_id) VALUES (?1)", [here(id)])?;
        }
        for collection in &file.collections {
            transaction.execute("INSERT INTO collections (name) VALUES (?1)", [collection.name.trim()])?;
            let id = transaction.last_insert_rowid();
            for game in &collection.game_ids {
                transaction.execute(
                    "INSERT OR IGNORE INTO collection_games (collection_id, game_id) VALUES (?1, ?2)",
                    params![id, here(game)],
                )?;
            }
        }

        let settings = &file.settings;
        let set = |key: &str, value: &str| {
            transaction.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
        };
        match &settings.chosen_steam_account {
            Some(account) => set(CHOSEN_STEAM_ACCOUNT, account)?,
            None => transaction.execute("DELETE FROM settings WHERE key = ?1", [CHOSEN_STEAM_ACCOUNT])?,
        };
        set(THEME, settings.theme.as_setting())?;
        set(LANGUAGE, settings.language.as_setting())?;
        set(LIBRARY_LAYOUT, settings.library_layout.as_setting())?;
        set(LIBRARY_SORT, settings.library_sort.stored_name())?;
        set(window::CLOSE_TO_TRAY, if settings.close_to_tray { "true" } else { "false" })?;

        before_commit(&numbers)?;
        transaction.commit()?;
        Ok(removed)
    }
}
