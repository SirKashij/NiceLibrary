//! What the store reads only for an Export File (crate::export).

use super::{Error, Store};

impl Store {
    /// Every Session of a Local Game, as the number of its Game, its start
    /// and its end in seconds since the Unix epoch, oldest first.
    pub(crate) fn local_game_sessions(&self) -> Result<Vec<(i64, u64, u64)>, Error> {
        let connection = self.connection();
        let mut statement = connection
            .prepare("SELECT local_game, started_at, ended_at FROM local_game_sessions ORDER BY started_at, id")?;
        let sessions = statement
            .query_map([], |row| {
                let started_at: i64 = row.get(1)?;
                let ended_at: i64 = row.get(2)?;
                Ok((row.get(0)?, u64::try_from(started_at).unwrap_or(0), u64::try_from(ended_at).unwrap_or(0)))
            })?
            .collect::<Result<_, _>>()?;
        Ok(sessions)
    }
}
