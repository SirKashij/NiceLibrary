//! Which Games the user hid. Hiding belongs to the person, not to a Steam
//! Account, and is kept apart from the Games themselves, so a Sync that
//! rewrites a Game never brings it back.

use std::collections::HashSet;

use super::Store;
use crate::Error;

impl Store {
    /// The ids of every Hidden Game, also of one no longer in the Library.
    pub(crate) fn hidden_game_ids(&self) -> Result<HashSet<String>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT game_id FROM hidden_games")?;
        let ids = statement.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?;
        Ok(ids)
    }

    /// Hides the Game `game_id`; nothing when it is hidden already.
    pub(crate) fn hide_game(&self, game_id: &str) -> Result<(), Error> {
        self.connection().execute("INSERT OR IGNORE INTO hidden_games (game_id) VALUES (?1)", [game_id])?;
        Ok(())
    }

    /// Unhides the Game `game_id`; nothing when it is not hidden.
    pub(crate) fn unhide_game(&self, game_id: &str) -> Result<(), Error> {
        self.connection().execute("DELETE FROM hidden_games WHERE game_id = ?1", [game_id])?;
        Ok(())
    }
}
