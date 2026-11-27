//! The real ports: what the core needs from the operating system.
//!
//! Version 1 implements them for Windows (ADR-0001). On other systems the
//! app builds with stand-ins that report nothing found.

#[cfg(windows)]
mod autostart_windows;
mod clock;
#[cfg(windows)]
mod file_dialog_windows;
mod http;
#[cfg(windows)]
mod icon_windows;
#[cfg(windows)]
mod opener_windows;
#[cfg(windows)]
mod process_starter_windows;
#[cfg(windows)]
mod processes_windows;
#[cfg(windows)]
mod secrets_windows;
#[cfg(windows)]
mod steam_windows;

#[cfg(windows)]
pub use autostart_windows::WindowsAutostart;
pub use clock::SystemClock;
#[cfg(windows)]
pub use file_dialog_windows::{WindowsExecutablePicker, WindowsDataFilePicker, WindowsImagePicker};
pub use http::{HttpDownloader, HttpWebApi};
#[cfg(windows)]
pub use icon_windows::WindowsIconExtractor;
#[cfg(windows)]
pub use opener_windows::WindowsOpener;
#[cfg(windows)]
pub use process_starter_windows::WindowsProcessStarter;
#[cfg(windows)]
pub use processes_windows::WindowsProcessList;
#[cfg(windows)]
pub use secrets_windows::WindowsSecretStore;
#[cfg(windows)]
pub use steam_windows::{Hive, RegistryValue, WindowsSteamLocator};

use nicelibrary_core::{
    Autostart, ExecutablePicker, DataFilePicker, IconExtractor, ImagePicker, Opener, Ports, ProcessList, ProcessStarter, SecretStore,
    SteamLocator,
};

/// The ports of this machine.
pub fn real_ports() -> Ports {
    Ports {
        steam_locator: steam_locator(),
        downloader: Box::new(HttpDownloader::new()),
        opener: opener(),
        secrets: secrets(),
        web_api: Box::new(HttpWebApi::new()),
        clock: Box::new(SystemClock),
        autostart: autostart(),
        executable_picker: executable_picker(),
        processes: processes(),
        process_starter: process_starter(),
        icon_extractor: icon_extractor(),
        image_picker: image_picker(),
        data_file_picker: data_file_picker(),
        update_downloader: Box::new(HttpDownloader::for_update_packages()),
    }
}

#[cfg(windows)]
fn process_starter() -> Box<dyn ProcessStarter> {
    Box::new(WindowsProcessStarter)
}

#[cfg(not(windows))]
fn process_starter() -> Box<dyn ProcessStarter> {
    /// Launching Local Games on Linux and macOS is not part of version 1.
    struct NoProcessStarter;

    impl ProcessStarter for NoProcessStarter {
        fn start(&self, executable: &std::path::Path, _: &[String], _: &std::path::Path) -> Result<(), String> {
            Err(format!("starting {} is not supported on this system", executable.display()))
        }
    }

    Box::new(NoProcessStarter)
}

#[cfg(windows)]
fn executable_picker() -> Box<dyn ExecutablePicker> {
    Box::new(WindowsExecutablePicker)
}

#[cfg(not(windows))]
fn executable_picker() -> Box<dyn ExecutablePicker> {
    /// Adding Local Games on Linux and macOS is not part of version 1.
    struct NoFileDialog;

    impl ExecutablePicker for NoFileDialog {
        fn choose_executable(&self) -> Result<Option<std::path::PathBuf>, String> {
            Err("choosing an executable is not supported on this system".into())
        }
    }

    Box::new(NoFileDialog)
}

#[cfg(windows)]
fn icon_extractor() -> Box<dyn IconExtractor> {
    Box::new(WindowsIconExtractor)
}

#[cfg(not(windows))]
fn icon_extractor() -> Box<dyn IconExtractor> {
    /// Local Games on Linux and macOS are not part of version 1.
    struct NoIcons;

    impl IconExtractor for NoIcons {
        fn largest_icon(&self, _executable: &std::path::Path) -> Result<Option<nicelibrary_core::Icon>, String> {
            Ok(None)
        }
    }

    Box::new(NoIcons)
}

#[cfg(windows)]
fn image_picker() -> Box<dyn ImagePicker> {
    Box::new(WindowsImagePicker)
}

#[cfg(not(windows))]
fn image_picker() -> Box<dyn ImagePicker> {
    /// Choosing a Cover image on Linux and macOS is not part of version 1.
    struct NoFileDialog;

    impl ImagePicker for NoFileDialog {
        fn choose_image(&self) -> Result<Option<std::path::PathBuf>, String> {
            Err("choosing an image is not supported on this system".into())
        }
    }

    Box::new(NoFileDialog)
}

#[cfg(windows)]
fn secrets() -> Box<dyn SecretStore> {
    Box::new(WindowsSecretStore::new())
}

#[cfg(not(windows))]
fn secrets() -> Box<dyn SecretStore> {
    /// A secret store on Linux and macOS is not part of version 1; without
    /// one, no Steam Web API key can be kept.
    struct NoSecretStore;

    impl SecretStore for NoSecretStore {
        fn read(&self, _name: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn write(&self, _name: &str, _secret: &str) -> Result<(), String> {
            Err("keeping secrets is not supported on this system".into())
        }

        fn delete(&self, _name: &str) -> Result<(), String> {
            Ok(())
        }
    }

    Box::new(NoSecretStore)
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
        fn open_url(&self, url: &str) -> Result<(), nicelibrary_core::OpenError> {
            Err(nicelibrary_core::OpenError::Failed(format!("opening {url} is not supported on this system")))
        }
    }

    Box::new(NoOpener)
}

#[cfg(windows)]
fn autostart() -> Box<dyn Autostart> {
    Box::new(WindowsAutostart::new())
}

#[cfg(not(windows))]
fn autostart() -> Box<dyn Autostart> {
    /// Starting with the system on Linux and macOS is not part of version 1.
    struct NoAutostart;

    impl Autostart for NoAutostart {
        fn is_on(&self) -> Result<bool, String> {
            Ok(false)
        }

        fn turn_on(&self, _arguments: &[&str]) -> Result<(), String> {
            Err("starting with the system is not supported on this system".into())
        }

        fn turn_off(&self) -> Result<(), String> {
            Ok(())
        }
    }

    Box::new(NoAutostart)
}

#[cfg(windows)]
fn processes() -> Box<dyn ProcessList> {
    Box::new(WindowsProcessList)
}

#[cfg(not(windows))]
fn processes() -> Box<dyn ProcessList> {
    /// Recording Sessions on Linux and macOS is not part of version 1.
    struct NoProcessList;

    impl ProcessList for NoProcessList {
        fn running_executables(&self) -> Result<Vec<std::path::PathBuf>, String> {
            Err("listing processes is not supported on this system".into())
        }
    }

    Box::new(NoProcessList)
}

#[cfg(windows)]
fn data_file_picker() -> Box<dyn DataFilePicker> {
    Box::new(WindowsDataFilePicker)
}

#[cfg(not(windows))]
fn data_file_picker() -> Box<dyn DataFilePicker> {
    /// Exporting and importing on Linux and macOS is not part of version 1.
    struct NoFileDialog;

    impl DataFilePicker for NoFileDialog {
        fn choose_export_file(&self, _suggested_name: &str) -> Result<Option<std::path::PathBuf>, String> {
            Err("choosing where to save is not supported on this system".into())
        }

        fn choose_import_file(&self) -> Result<Option<std::path::PathBuf>, String> {
            Err("choosing a file to import is not supported on this system".into())
        }
    }

    Box::new(NoFileDialog)
}
