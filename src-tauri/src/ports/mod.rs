//! The real ports: what the core needs from the operating system.
//!
//! Version 1 implements them for Windows (ADR-0001). On other systems the
//! app builds with stand-ins that report nothing found.

mod http;
#[cfg(windows)]
mod opener_windows;
#[cfg(windows)]
mod steam_windows;

pub use http::HttpDownloader;
#[cfg(windows)]
pub use opener_windows::WindowsOpener;
#[cfg(windows)]
pub use steam_windows::{Hive, RegistryValue, WindowsSteamLocator};

use nicelibrary_core::{Opener, Ports, SteamLocator};

/// The ports of this machine.
pub fn real_ports() -> Ports {
    Ports { steam_locator: steam_locator(), downloader: Box::new(HttpDownloader::new()), opener: opener() }
}

#[cfg(windows)]
fn steam_locator() -> Box<dyn SteamLocator> {
    Box::new(WindowsSteamLocator::new())
}

#[cfg(windows)]
fn opener() -> Box<dyn Opener> {
    Box::new(WindowsOpener)
}

#[cfg(not(windows))]
fn steam_locator() -> Box<dyn SteamLocator> {
    /// Locating Steam on Linux and macOS is not part of version 1.
    struct NotLocated;

    impl SteamLocator for NotLocated {
        fn steam_folder(&self) -> Option<std::path::PathBuf> {
            None
        }
    }

    Box::new(NotLocated)
}

#[cfg(not(windows))]
fn opener() -> Box<dyn Opener> {
    /// Launching on Linux and macOS is not part of version 1.
    struct NoOpener;

    impl Opener for NoOpener {
        fn open_url(&self, url: &str) -> Result<(), String> {
            Err(format!("opening {url} is not supported on this system"))
        }
    }

    Box::new(NoOpener)
}
