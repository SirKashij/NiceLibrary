//! Favorites: Games the user marks to find them quickly. The mark belongs to
//! the Game's id, so it outlasts a Sync that takes a Steam Game out of the
//! Library and back in.

use std::collections::HashSet;

use crate::Game;

/// Marks each Game of `games` whose id is in `favorites` as a Favorite.
pub(crate) fn mark(games: &mut [Game], favorites: &HashSet<String>) {
    for game in games {
        game.favorite = favorites.contains(&game.id);
    }
}
