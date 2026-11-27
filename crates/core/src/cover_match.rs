//! Cover Match: the Steam title whose Cover a Local Game borrows, step 1 of
//! the chain in ADR-0005. The app proposes one from the stored Steam App
//! List when a Local Game is added; the user confirms it, picks another
//! title or declines, and can change or remove it later.

use crate::steam_app_list::{SteamTitle, searchable};

/// The title among `found`, as a lookup for `search` answers them, that the
/// app proposes for a Local Game named `search`, written as [`searchable`]
/// writes it: the one of the same name, else the first whose name begins
/// with all of `search`'s words, such as "The Witcher 3: Wild Hunt" for "The
/// Witcher 3". A title that merely contains the name, or begins with half a
/// word of it, is no proposal: a wrong Cover is worse than the icon.
pub(crate) fn proposal(search: &str, found: Vec<SteamTitle>) -> Option<SteamTitle> {
    let words = format!("{search} ");
    found.into_iter().find(|title| {
        let name = searchable(&title.name);
        name == search || name.starts_with(&words)
    })
}
