//! Steam Source, local files: the Steam installation, its Steam Accounts and
//! the apps installed in its library folders.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::keyvalues;

/// A Steam app that is fully installed, as its app manifest describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstalledApp {
    pub app_id: u32,
    pub title: String,
}

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

/// How choosing the Steam Account went, when nothing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum SteamAccountChoice {
    /// The app works with the chosen Steam Account from now on.
    Chosen,
    /// What was entered is not the SteamID64 of a personal Steam Account;
    /// nothing changed.
    InvalidSteamId,
}

/// The SteamID64 in `text` if it is one of a personal Steam Account, without
/// surrounding spaces: 17 digits that encode the public universe, an
/// individual account, the desktop instance and an account number above 0.
pub(crate) fn personal_steam_id(text: &str) -> Option<&str> {
    /// Universe 1 (public), account type 1 (individual), instance 1.
    const PERSONAL: u64 = 0x0110_0001;
    let text = text.trim();
    // Exactly as Steam writes it, so it matches the accounts in loginusers.vdf.
    if text.len() != 17 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let id = text.parse::<u64>().ok()?;
    (id >> 32 == PERSONAL && id & 0xFFFF_FFFF != 0).then_some(text)
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

/// The Steam Account the app works with until the user chooses one: the one
/// that logged in most recently, else the first one listed.
pub(crate) fn current_account(folder: &Path) -> Option<SteamAccount> {
    let accounts = read_accounts(folder);
    let index = accounts.iter().position(|a| a.most_recent).unwrap_or(0);
    accounts.into_iter().nth(index)
}

/// The apps that are fully installed in any of Steam's library folders, each
/// once, without Steam's own redistributables and runtimes.
///
/// `None` when the list of library folders exists but cannot be read: then
/// it is unknown which apps are Installed, and nothing may be concluded.
pub(crate) fn installed_apps(steam_folder: &Path) -> Option<Vec<InstalledApp>> {
    let mut apps = BTreeMap::new();
    for folder in library_folders(steam_folder)? {
        let Ok(entries) = fs::read_dir(folder.join("steamapps")) else {
            continue; // a drive that is not connected right now
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            if let Some(app) = read_manifest(&entry.path()).filter(|app| !is_steam_tool(app)) {
                apps.entry(app.app_id).or_insert(app);
            }
        }
    }
    Some(apps.into_values().collect())
}

/// The Steam folder itself and every folder in `steamapps/libraryfolders.vdf`,
/// each once.
fn library_folders(steam_folder: &Path) -> Option<Vec<PathBuf>> {
    let mut folders = vec![steam_folder.to_path_buf()];
    // A missing file means Steam has only its own folder.
    if let Ok(bytes) = fs::read(steam_folder.join("steamapps").join("libraryfolders.vdf")) {
        let doc = keyvalues::parse(&String::from_utf8_lossy(&bytes)).ok()?;
        for (key, value) in doc.block("libraryfolders")?.entries() {
            // Numbered entries are folders; current Steam writes a block with
            // "path", older versions wrote the path directly.
            if key.parse::<u32>().is_err() {
                continue;
            }
            let path = match value {
                keyvalues::Value::Block(folder) => folder.string("path"),
                keyvalues::Value::String(path) => Some(path.as_str()),
            };
            if let Some(path) = path.filter(|p| !p.trim().is_empty()) {
                folders.push(PathBuf::from(path));
            }
        }
    }
    let mut seen = HashSet::new();
    folders.retain(|folder| seen.insert(fs::canonicalize(folder).unwrap_or_else(|_| folder.clone())));
    Some(folders)
}

/// Steam's flag for an app whose files are complete. It stays set while an
/// update waits or downloads.
const FULLY_INSTALLED: u32 = 4;

/// The app in one `appmanifest_<id>.acf`, if it is fully installed. A
/// manifest that cannot be read is skipped.
fn read_manifest(path: &Path) -> Option<InstalledApp> {
    let bytes = fs::read(path).ok()?;
    let doc = keyvalues::parse(&String::from_utf8_lossy(&bytes)).ok()?;
    let state = doc.block("AppState")?;
    let app_id = state.string("appid")?.trim().parse::<u32>().ok()?;
    let flags = state.string("StateFlags")?.trim().parse::<u32>().ok()?;
    if flags & FULLY_INSTALLED == 0 {
        return None;
    }
    let title = [state.string("name"), state.string("installdir")]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|title| !title.is_empty())?
        .to_owned();
    Some(InstalledApp { app_id, title })
}

/// Steam's own redistributables and runtimes, which Steam installs next to
/// Games but which are not Games.
const STEAM_TOOL_IDS: &[u32] = &[
    228980,  // Steamworks Common Redistributables
    1070560, // Steam Linux Runtime 1.0 (scout)
    1391110, // Steam Linux Runtime 2.0 (soldier)
    1628350, // Steam Linux Runtime 3.0 (sniper)
    1493710, // Proton Experimental
    2180100, // Proton Hotfix
    1826330, // Proton EasyAntiCheat Runtime
    1161040, // Proton BattlEye Runtime
];

/// Whether an app is one of Steam's tools rather than a Game. Versioned
/// entries ("Proton 9.0", "Steam Linux Runtime 4.0") are recognised by name,
/// because each new version gets a new app id.
fn is_steam_tool(app: &InstalledApp) -> bool {
    let title = app.title.as_str();
    let proton_version =
        title.strip_prefix("Proton ").is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()));
    STEAM_TOOL_IDS.contains(&app.app_id)
        || title.starts_with("Steamworks Common Redistributables")
        || title.starts_with("Steam Linux Runtime")
        || proton_version
}
