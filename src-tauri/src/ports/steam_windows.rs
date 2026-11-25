//! Finds the Steam installation through the registry entries Steam writes.

use std::path::PathBuf;

use nicelibrary_core::SteamLocator;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

/// A registry hive the locator may read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

/// One registry value that may hold the Steam folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryValue {
    pub hive: Hive,
    pub key: String,
    pub name: String,
}

/// Reads the Steam folder from the first registry value that names an
/// existing folder.
pub struct WindowsSteamLocator {
    candidates: Vec<RegistryValue>,
}

impl WindowsSteamLocator {
    /// Reads where Steam records itself: the current user's `SteamPath`,
    /// then the machine-wide `InstallPath` the Steam installer writes.
    pub fn new() -> Self {
        Self::reading(vec![
            RegistryValue {
                hive: Hive::CurrentUser,
                key: r"Software\Valve\Steam".into(),
                name: "SteamPath".into(),
            },
            RegistryValue {
                hive: Hive::LocalMachine,
                key: r"SOFTWARE\WOW6432Node\Valve\Steam".into(),
                name: "InstallPath".into(),
            },
        ])
    }

    /// Reads the given registry values, in order.
    pub fn reading(candidates: Vec<RegistryValue>) -> Self {
        Self { candidates }
    }
}

impl Default for WindowsSteamLocator {
    fn default() -> Self {
        Self::new()
    }
}

impl SteamLocator for WindowsSteamLocator {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.candidates.iter().filter_map(read).find(|folder| folder.is_dir())
    }
}

fn read(value: &RegistryValue) -> Option<PathBuf> {
    let hive = match value.hive {
        Hive::CurrentUser => HKEY_CURRENT_USER,
        Hive::LocalMachine => HKEY_LOCAL_MACHINE,
    };
    let text: String = RegKey::predef(hive).open_subkey(&value.key).ok()?.get_value(&value.name).ok()?;
    let text = text.trim();
    // Steam writes `SteamPath` with forward slashes ("c:/program files (x86)/steam").
    (!text.is_empty()).then(|| PathBuf::from(text.replace('/', "\\")))
}
