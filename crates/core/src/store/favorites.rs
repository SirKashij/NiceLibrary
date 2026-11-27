//! The Favorites in the store: one row per Game id the user marked.

use std::collections::HashSet;

use super::Store;
use crate::Error;

impl Store {
    /// The ids of every Game marked as a Favorite, in the Library or not.
    pub(crate) fn favorites(&self) -> Result<HashSet<String>, Error> {
        let connection = self.connection();
        let mut statement = connection.prepare("SELECT game_id FROM favorites")?;
        let favorites = statement.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?;
        Ok(favorites)
    }

    /// Marks the Game `game_id` as a Favorite, or unmarks it; marking it
    /// again, or unmarking a Game that is none, changes nothing.
    pub(crate) fn mark_favorite(&self, game_id: &str, favorite: bool) -> Result<(), Error> {
        let sql = if favorite {
            "INSERT INTO favorites (game_id) VALUES (?1) ON CONFLICT (game_id) DO NOTHING"
        } else {
            "DELETE FROM favorites WHERE game_id = ?1"
        };
        self.connection().execute(sql, [game_id])?;
        Ok(())
    }
}
