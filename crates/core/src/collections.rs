//! Collections: named groups of Games the user creates. A Game can be in
//! several Collections; deleting a Collection removes no Game.

use serde::Serialize;

/// A Collection as the sidebar lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: i64,
    pub name: String,
    /// How many Games of the Library are in it.
    pub game_count: u32,
}

/// How naming a Collection, on creating or renaming it, went when nothing
/// failed. Every outcome but `Named` changed nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum CollectionNaming {
    /// The Collection `id` has the name now.
    Named { id: i64 },
    /// The name is empty, or only spaces.
    EmptyName,
    /// Another Collection has the name already, maybe in another case.
    NameTaken,
}

/// The name as it is saved: without the spaces around it; `None` when
/// nothing is left.
pub(crate) fn name_in(name: &str) -> Option<&str> {
    Some(name.trim()).filter(|name| !name.is_empty())
}

/// Whether two names count as the same name: they differ in case at most.
pub(crate) fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}
