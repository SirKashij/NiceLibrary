//! Contract tests for the real ports.
//!
//! The core is tested against substitute ports. The tests in this group check
//! that the real ports (Steam installation, process list, secret storage, ...)
//! honour the same contract on a real machine. They touch the real operating
//! system, so they are not part of `cargo test` and only run on demand with
//! `cargo test-contract`.

#[test]
fn contract_tests_run_on_windows() {
    assert!(
        cfg!(windows),
        "version 1 ships real ports for Windows only (ADR-0001); run contract tests on Windows"
    );
}

/// `SteamLocator` (crates/core): `None` when Steam is not installed,
/// otherwise a folder that exists.
///
/// The tests write their own registry values under
/// `HKCU\Software\NiceLibraryContractTests\<unique>` and delete them again;
/// Steam's own keys are only read.
#[cfg(windows)]
mod steam_locator {
    use std::fs;
    use std::path::Path;

    use nicelibrary_core::SteamLocator;
    use nicelibrary_lib::ports::{Hive, RegistryValue, WindowsSteamLocator};
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    const PARENT: &str = r"Software\NiceLibraryContractTests";

    /// A registry key of its own for one test, removed when dropped.
    struct TestKey {
        path: String,
    }

    impl TestKey {
        fn new(name: &str) -> Self {
            let path = format!(r"{PARENT}\{name}-{}", std::process::id());
            RegKey::predef(HKEY_CURRENT_USER).create_subkey(&path).unwrap();
            Self { path }
        }

        fn set(&self, name: &str, value: &str) {
            let key = RegKey::predef(HKEY_CURRENT_USER).create_subkey(&self.path).unwrap().0;
            key.set_value(name, &value).unwrap();
        }

        fn value(&self, name: &str) -> RegistryValue {
            RegistryValue { hive: Hive::CurrentUser, key: self.path.clone(), name: name.into() }
        }
    }

    impl Drop for TestKey {
        fn drop(&mut self) {
            let user = RegKey::predef(HKEY_CURRENT_USER);
            let _ = user.delete_subkey_all(&self.path);
            // Fails while other tests still hold keys below it; the last one removes it.
            let _ = user.delete_subkey(PARENT);
        }
    }

    fn same_folder(a: &Path, b: &Path) -> bool {
        fs::canonicalize(a).unwrap() == fs::canonicalize(b).unwrap()
    }

    #[test]
    fn finds_the_folder_the_registry_names() {
        let steam = tempfile::tempdir().unwrap();
        let key = TestKey::new("finds");
        key.set("SteamPath", &steam.path().display().to_string());

        let found = WindowsSteamLocator::reading(vec![key.value("SteamPath")]).steam_folder();

        assert!(same_folder(&found.expect("Steam folder"), steam.path()));
    }

    #[test]
    fn reads_the_folder_in_steams_own_spelling_with_forward_slashes() {
        let steam = tempfile::tempdir().unwrap();
        let key = TestKey::new("slashes");
        key.set("SteamPath", &steam.path().display().to_string().replace('\\', "/").to_lowercase());

        let found = WindowsSteamLocator::reading(vec![key.value("SteamPath")]).steam_folder().expect("Steam folder");

        assert!(!found.display().to_string().contains('/'), "not normalised: {}", found.display());
        assert!(same_folder(&found, steam.path()));
    }

    #[test]
    fn a_folder_that_no_longer_exists_is_not_found() {
        let steam = tempfile::tempdir().unwrap();
        let gone = steam.path().join("uninstalled");
        let key = TestKey::new("gone");
        key.set("SteamPath", &gone.display().to_string());

        assert_eq!(WindowsSteamLocator::reading(vec![key.value("SteamPath")]).steam_folder(), None);
    }

    #[test]
    fn a_missing_key_or_value_is_not_found() {
        let key = TestKey::new("missing");
        let missing_key = RegistryValue {
            hive: Hive::CurrentUser,
            key: format!(r"{}\Absent", key.path),
            name: "SteamPath".into(),
        };

        assert_eq!(WindowsSteamLocator::reading(vec![missing_key, key.value("SteamPath")]).steam_folder(), None);
    }

    #[test]
    fn falls_back_to_the_next_registry_value() {
        let steam = tempfile::tempdir().unwrap();
        let key = TestKey::new("fallback");
        key.set("InstallPath", &steam.path().display().to_string());

        let found = WindowsSteamLocator::reading(vec![key.value("SteamPath"), key.value("InstallPath")])
            .steam_folder();

        assert!(same_folder(&found.expect("Steam folder"), steam.path()));
    }

    #[test]
    fn on_this_machine_finds_steam_or_says_it_is_not_installed() {
        if let Some(folder) = WindowsSteamLocator::new().steam_folder() {
            assert!(folder.is_dir(), "{} is not a folder", folder.display());
        }
    }
}

/// `Downloader` (crates/core): `Ok(Some(bytes))` when the server sends the
/// file, `Ok(None)` when it answers that the file does not exist (404, 410),
/// `Err` for every other failure.
///
/// The tests run a small web server of their own on this machine; only the
/// last one reaches Steam's artwork host.
mod downloader {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    use nicelibrary_core::Downloader;
    use nicelibrary_lib::ports::HttpDownloader;

    /// A web server on this machine that answers one request with `status`
    /// and `body`, and the address it serves.
    fn serve_once(status: &str, body: &[u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (status, body) = (status.to_owned(), body.to_vec());
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // Read the request up to the blank line that ends its head.
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 0 && line != "\r\n" {
                line.clear();
            }
            let head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
        });
        format!("http://{address}/cover.png")
    }

    /// A PNG image generated for the test.
    fn generated_png() -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 2, 3);
        encoder.set_color(png::ColorType::Rgb);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[90; 2 * 3 * 3]).unwrap();
        writer.finish().unwrap();
        bytes
    }

    #[test]
    fn a_file_the_server_sends_is_downloaded() {
        let image = generated_png();
        let url = serve_once("200 OK", &image);

        assert_eq!(HttpDownloader::new().download(&url), Ok(Some(image)));
    }

    #[test]
    fn a_file_the_server_does_not_have_is_none() {
        for status in ["404 Not Found", "410 Gone"] {
            let url = serve_once(status, b"<!doctype html><title>Not found</title>");

            assert_eq!(HttpDownloader::new().download(&url), Ok(None), "{status}");
        }
    }

    #[test]
    fn a_server_error_is_an_error() {
        let url = serve_once("503 Service Unavailable", b"busy");

        assert!(HttpDownloader::new().download(&url).is_err());
    }

    #[test]
    fn a_server_that_cannot_be_reached_is_an_error() {
        // A port that was free a moment ago and has no server now.
        let address = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();

        assert!(HttpDownloader::new().download(&format!("http://{address}/cover.png")).is_err());
    }

    #[test]
    fn downloads_a_cover_from_steams_artwork_host_over_https() {
        // Portal 2's upright library image. Only its format is checked, and it
        // is not kept anywhere.
        let url = "https://shared.steamstatic.com/store_item_assets/steam/apps/620/library_600x900.jpg";

        let bytes = HttpDownloader::new().download(url).expect("Steam reachable").expect("the image exists");

        assert!(bytes.starts_with(&[0xFF, 0xD8, 0xFF]), "not a JPEG");
    }
}
