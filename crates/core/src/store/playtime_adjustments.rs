//! The stored Playtime Adjustments of Local Games (see `crate::playtime_adjustment`).

use std::collections::BTreeMap;

use rusqlite::params;

use super::Store;
use crate::Error;

impl Store {
    /// The Playtime Adjustment of each Local Game that has one, in minutes,
    /// by the Local Game's id.
    pub(crate) fn playtime_adjustments(&self) -> Result<BTreeMap<i64, i32>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT local_game, minutes FROM local_game_playtime_adjustments")?;
        let adjustments = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(adjustments)
    }

    /// Makes `minutes` the Playtime Adjustment of the Local Game
    /// `local_game`, in place of any earlier one; `None` clears it.
    pub(crate) fn set_playtime_adjustment(&self, local_game: i64, minutes: Option<i32>) -> Result<(), Error> {
        let connection = self.connection();
        match minutes {
            Some(minutes) => connection.execute(
                "INSERT INTO local_game_playtime_adjustments (local_game, minutes) VALUES (?1, ?2)
                 ON CONFLICT (local_game) DO UPDATE SET minutes = excluded.minutes",
                params![local_game, minutes],
            )?,
            None => connection
                .execute("DELETE FROM local_game_playtime_adjustments WHERE local_game = ?1", [local_game])?,
        };
        Ok(())
    }
}
