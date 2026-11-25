//! The real ports: what the core needs from the operating system.
//!
//! Version 1 implements them for Windows (ADR-0001). On other systems the
//! app builds with stand-ins that report nothing found.

#[cfg(windows)]
mod steam_windows;

#[cfg(windows)]
pub use steam_windows::{Hive, RegistryValue, WindowsSteamLocator};

use nicelibrary_core::Ports;

/// The ports of this machine.
pub fn real_ports() -> Ports {
    Ports { steam_locator: steam_locator() }
}

#[cfg(windows)]
fn steam_locator() -> Box<dyn nicelibrary_core::SteamLocator> {
    Box::new(WindowsSteamLocator::new())
}

#[cfg(not(windows))]
fn steam_locator() -> Box<dyn nicelibrary_core::SteamLocator> {
    /// Locating Steam on Linux and macOS is not part of version 1.
    struct NotLocated;

    impl nicelibrary_core::SteamLocator for NotLocated {
        fn steam_folder(&self) -> Option<std::path::PathBuf> {
            None
        }
    }

    Box::new(NotLocated)
}
