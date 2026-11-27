//! The stored Achievements of Steam Games, and the Web API calls counted per day.

use std::collections::HashSet;

use rusqlite::{OptionalExtension, params};

use super::{Store, seconds_column};
use crate::Error;
use crate::achievements::{Achievement, DueGame, Fetched};

impl Store {
    /// How many Web API calls Syncs made on `day` (days since the Unix epoch, UTC).
    pub(crate) fn web_api_calls(&self, day: u64) -> Result<u32, Error> {
        let calls: Option<u32> = self
            .connection()
            .query_row("SELECT calls FROM steam_web_api_calls WHERE day = ?1", [seconds_column(day)?], |row| row.get(0))
            .optional()?;
        Ok(calls.unwrap_or(0))
    }

    /// At most `limit` Steam Games `account` owns whose Achievements are due:
    /// first those played since they were last fetched, then those never
    /// fetched, each the most recently played first.
    pub(crate) fn games_with_achievements_due(&self, account: &str, limit: usize) -> Result<Vec<DueGame>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare(
            "SELECT g.app_id, g.playtime_minutes, g.last_played
             FROM steam_games g
             LEFT JOIN steam_achievement_fetches f ON f.steam_account = g.steam_account AND f.app_id = g.app_id
             WHERE g.steam_account = ?1 AND g.owned = 1
               AND (f.app_id IS NULL OR f.playtime_minutes IS NOT g.playtime_minutes OR f.last_played IS NOT g.last_played)
             ORDER BY f.app_id IS NULL, g.last_played IS NULL, g.last_played DESC, g.playtime_minutes DESC, g.app_id
             LIMIT ?2",
        )?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let games = statement
            .query_map(params![account, limit], |row| {
                let last_played: Option<i64> = row.get(2)?;
                Ok(DueGame {
                    app_id: row.get(0)?,
                    playtime_minutes: row.get(1)?,
                    last_played: last_played.and_then(|seconds| u64::try_from(seconds).ok()),
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(games)
    }

    /// Steam's names of the Achievements of the Steam Game `app_id` whose definitions are stored.
    pub(crate) fn achievement_definitions_known(&self, app_id: u32) -> Result<HashSet<String>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT api_name FROM steam_achievement_definitions WHERE app_id = ?1")?;
        let known = statement.query_map([app_id], |row| row.get(0))?.collect::<Result<_, _>>()?;
        Ok(known)
    }

    /// Records what one Sync fetched for `account`, all at once: each game's
    /// Achievements replace the stored ones, new definitions replace the
    /// game's stored definitions, and `calls` Web API calls count for `day`.
    pub(crate) fn record_achievements(
        &self,
        account: &str,
        fetched: &[Fetched],
        fetched_at: u64,
        day: u64,
        calls: u32,
    ) -> Result<(), Error> {
        let fetched_at = seconds_column(fetched_at)?;
        let day = seconds_column(day)?;
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        for game in fetched {
            let app_id = game.game.app_id;
            if let Some(definitions) = &game.definitions {
                transaction.execute("DELETE FROM steam_achievement_definitions WHERE app_id = ?1", [app_id])?;
                for (position, definition) in definitions.iter().enumerate() {
                    transaction.execute(
                        "INSERT OR REPLACE INTO steam_achievement_definitions
                             (app_id, api_name, position, name, description, secret, icon, icon_locked)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            app_id,
                            definition.api_name,
                            i64::try_from(position).unwrap_or(i64::MAX),
                            definition.name,
                            definition.description,
                            definition.secret,
                            definition.icon,
                            definition.icon_locked
                        ],
                    )?;
                }
            }
            transaction.execute(
                "DELETE FROM steam_achievements WHERE steam_account = ?1 AND app_id = ?2",
                params![account, app_id],
            )?;
            for achievement in &game.achievements {
                let unlocked_at = achievement.unlocked_at.map(seconds_column).transpose()?;
                transaction.execute(
                    "INSERT OR REPLACE INTO steam_achievements (steam_account, app_id, api_name, unlocked, unlocked_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![account, app_id, achievement.api_name, achievement.unlocked, unlocked_at],
                )?;
            }
            let last_played = game.game.last_played.map(seconds_column).transpose()?;
            transaction.execute(
                "INSERT INTO steam_achievement_fetches (steam_account, app_id, fetched_at, playtime_minutes, last_played)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (steam_account, app_id) DO UPDATE SET fetched_at = excluded.fetched_at,
                     playtime_minutes = excluded.playtime_minutes, last_played = excluded.last_played",
                params![account, app_id, fetched_at, game.game.playtime_minutes, last_played],
            )?;
        }
        transaction.execute(
            "INSERT INTO steam_web_api_calls (day, calls) VALUES (?1, ?2)
             ON CONFLICT (day) DO UPDATE SET calls = calls + excluded.calls",
            params![day, calls],
        )?;
        // Only today's count matters.
        transaction.execute("DELETE FROM steam_web_api_calls WHERE day < ?1", [day])?;
        transaction.commit()?;
        Ok(())
    }

    /// The Achievements of the Steam Game `app_id` for `account`, unlocked
    /// ones first, the most recently unlocked first, then the locked ones in
    /// Steam's order; `None` when they were never fetched.
    pub(crate) fn achievements(&self, account: &str, app_id: u32) -> Result<Option<Vec<Achievement>>, Error> {
        let connection = self.connection();
        let fetched: Option<i64> = connection
            .query_row(
                "SELECT fetched_at FROM steam_achievement_fetches WHERE steam_account = ?1 AND app_id = ?2",
                params![account, app_id],
                |row| row.get(0),
            )
            .optional()?;
        if fetched.is_none() {
            return Ok(None);
        }
        let mut statement = connection.prepare(
            "SELECT a.api_name, d.name, d.description, a.unlocked, a.unlocked_at, d.icon, d.icon_locked
             FROM steam_achievements a
             LEFT JOIN steam_achievement_definitions d ON d.app_id = a.app_id AND d.api_name = a.api_name
             WHERE a.steam_account = ?1 AND a.app_id = ?2
             ORDER BY a.unlocked DESC, a.unlocked_at IS NULL, a.unlocked_at DESC, d.position IS NULL, d.position, a.api_name",
        )?;
        let achievements = statement
            .query_map(params![account, app_id], |row| {
                let api_name: String = row.get(0)?;
                let name: Option<String> = row.get(1)?;
                let unlocked: bool = row.get(3)?;
                let unlocked_at: Option<i64> = row.get(4)?;
                let icon: Option<String> = row.get(if unlocked { 5 } else { 6 })?;
                Ok(Achievement {
                    // A definition Steam did not send yet: its name is all there is.
                    name: name.unwrap_or_else(|| api_name.clone()),
                    id: api_name,
                    description: row.get(2)?,
                    unlocked,
                    unlocked_at: unlocked_at.and_then(|seconds| u64::try_from(seconds).ok()),
                    icon,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(Some(achievements))
    }
}
