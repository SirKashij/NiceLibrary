//! Steam Source, local files: the Steam installation and its Steam Accounts.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::keyvalues;

/// Port: where Steam is installed on this computer.
///
/// Contract: returns `None` when Steam is not installed, otherwise a folder
/// that exists. The folder need not contain any particular file; the core
/// copes with whatever is missing.
pub trait SteamLocator: Send + Sync {
    fn steam_folder(&self) -> Option<PathBuf>;
}

/// The Steam installation as the settings show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SteamInstallation {
    NotFound,
    #[serde(rename_all = "camelCase")]
    Found {
        path: String,
        /// The Steam Accounts that have logged in here, the most recently
        /// used one first.
        accounts: Vec<SteamAccount>,
    },
}

/// A Steam Account that has logged in on this computer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamAccount {
    /// The SteamID64, as text because it exceeds JavaScript's safe integers.
    pub steam_id: String,
    pub display_name: String,
    /// Whether this is the account that logged in most recently.
    pub most_recent: bool,
}

pub(crate) fn read_installation(locator: &dyn SteamLocator) -> SteamInstallation {
    match locator.steam_folder() {
        None => SteamInstallation::NotFound,
        Some(folder) => SteamInstallation::Found {
            path: folder.display().to_string(),
            accounts: read_accounts(&folder),
        },
    }
}

/// The accounts in `config/loginusers.vdf`; none if the file is missing or
/// cannot be read.
fn read_accounts(folder: &Path) -> Vec<SteamAccount> {
    let Ok(bytes) = fs::read(folder.join("config").join("loginusers.vdf")) else {
        return Vec::new();
    };
    let Ok(doc) = keyvalues::parse(&String::from_utf8_lossy(&bytes)) else {
        return Vec::new();
    };
    let Some(users) = doc.block("users") else {
        return Vec::new();
    };

    let logins: Vec<(&str, &keyvalues::Block)> = users
        .entries()
        .filter_map(|(steam_id, value)| match value {
            keyvalues::Value::Block(user) => Some((steam_id, user)),
            keyvalues::Value::String(_) => None,
        })
        .collect();

    // Steam flags the last login with "MostRecent"; after a logout no account
    // may carry the flag, and then the latest login time decides.
    let flagged = logins.iter().position(|(_, user)| user.string("MostRecent") == Some("1"));
    let latest = logins
        .iter()
        .enumerate()
        .filter_map(|(index, (_, user))| Some((user.string("Timestamp")?.parse::<u64>().ok()?, index)))
        .max()
        .map(|(_, index)| index);
    let most_recent = flagged.or(latest);

    let mut accounts: Vec<SteamAccount> = logins
        .iter()
        .enumerate()
        .map(|(index, (steam_id, user))| SteamAccount {
            steam_id: (*steam_id).to_owned(),
            display_name: [user.string("PersonaName"), user.string("AccountName")]
                .into_iter()
                .flatten()
                .find(|name| !name.trim().is_empty())
                .unwrap_or(steam_id)
                .to_owned(),
            most_recent: Some(index) == most_recent,
        })
        .collect();
    accounts.sort_by_key(|a| !a.most_recent);
    accounts
}
