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
    pub fn serve_once(status: &str, body: &[u8]) -> String {
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

/// `Opener` (crates/core): `Ok` once the program registered for the URL's
/// scheme has the URL, `Err(NoProgram)` when none is registered, never a
/// dialog. Anything that is not a URL, and every `file://` URL, is refused
/// with `Err(Failed)`, so a Game's executable can never be started through it.
///
/// The tests register URL schemes of their own under
/// `HKCU\Software\Classes\nicelibrary-contract-<name>-<process id>`, whose
/// program writes the URL it receives to a file, and delete them again. They
/// never open a `steam://` URL, so Steam does not start.
///
/// `open_folder`: `Ok` once File Explorer has the folder; anything that is
/// not an existing folder is refused with `Err(Failed)` and nothing starts.
/// The test that shows a folder leaves one Explorer window open on an empty
/// temporary folder.
#[cfg(windows)]
mod opener {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use nicelibrary_core::{OpenError, Opener};
    use nicelibrary_lib::ports::WindowsOpener;
    use tempfile::TempDir;
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    /// A URL scheme registered for one test, removed when dropped. Its
    /// program writes each URL it is handed to `received.txt`.
    struct TestScheme {
        scheme: String,
        folder: TempDir,
    }

    impl TestScheme {
        fn register(name: &str) -> Self {
            let scheme = format!("nicelibrary-contract-{name}-{}", std::process::id());
            let folder = tempfile::tempdir().unwrap();
            let classes = RegKey::predef(HKEY_CURRENT_USER);
            let (key, _) = classes.create_subkey(class_key(&scheme)).unwrap();
            key.set_value("", &"URL:NiceLibrary contract test").unwrap();
            key.set_value("URL Protocol", &"").unwrap();
            let (command, _) = key.create_subkey(r"shell\open\command").unwrap();
            let cmd = std::env::var("ComSpec").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".into());
            let received = folder.path().join("received.txt");
            command.set_value("", &format!(r#""{cmd}" /d /c echo %1> "{}""#, received.display())).unwrap();
            Self { scheme, folder }
        }

        /// The URL the scheme's program received, waiting for it up to ten seconds.
        fn received(&self) -> Option<String> {
            wait_for_file(&self.folder.path().join("received.txt"))
        }
    }

    impl Drop for TestScheme {
        fn drop(&mut self) {
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(class_key(&self.scheme));
        }
    }

    fn class_key(scheme: &str) -> String {
        format!(r"Software\Classes\{scheme}")
    }

    fn wait_for_file(path: &Path) -> Option<String> {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            // The program may still be writing; a complete line ends the wait.
            if let Ok(text) = fs::read_to_string(path)
                && text.ends_with('\n')
            {
                return Some(text.trim().to_owned());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        None
    }

    /// A program that leaves a marker file when it runs, for checking that
    /// something was not started.
    fn marker_program(folder: &Path) -> (PathBuf, PathBuf) {
        let program = folder.join("game.cmd");
        let marker = folder.join("started.txt");
        fs::write(&program, format!("@echo started> \"{}\"\r\n", marker.display())).unwrap();
        (program, marker)
    }

    #[test]
    fn opens_a_url_with_the_program_registered_for_its_scheme() {
        let scheme = TestScheme::register("opens");
        let url = format!("{}://rungameid/620", scheme.scheme);

        assert_eq!(WindowsOpener.open_url(&url), Ok(()));
        assert_eq!(scheme.received().as_deref(), Some(url.as_str()));
    }

    #[test]
    fn a_scheme_without_a_registered_program_is_an_error() {
        let scheme = format!("nicelibrary-contract-unregistered-{}", std::process::id());

        assert_eq!(WindowsOpener.open_url(&format!("{scheme}://rungameid/620")), Err(OpenError::NoProgram));
    }

    #[test]
    fn a_path_to_a_program_is_refused_and_not_started() {
        let folder = tempfile::tempdir().unwrap();
        let (program, marker) = marker_program(folder.path());
        let file_url = format!("file:///{}", program.display().to_string().replace('\\', "/"));

        for target in [program.display().to_string(), file_url] {
            let refused = WindowsOpener.open_url(&target);
            assert!(matches!(refused, Err(OpenError::Failed(_))), "{target} was not refused: {refused:?}");
        }
        std::thread::sleep(Duration::from_secs(1));
        assert!(!marker.exists(), "the program was started");
    }

    /// Opens one File Explorer window, on an empty temporary folder.
    #[test]
    fn shows_an_existing_folder_in_file_explorer() {
        let folder = tempfile::tempdir().unwrap();

        assert_eq!(WindowsOpener.open_folder(folder.path()), Ok(()));
    }

    #[test]
    fn a_folder_that_does_not_exist_is_refused() {
        let folder = tempfile::tempdir().unwrap();

        let refused = WindowsOpener.open_folder(&folder.path().join("missing"));

        assert!(matches!(refused, Err(OpenError::Failed(_))), "{refused:?}");
    }

    #[test]
    fn a_program_is_refused_as_a_folder_and_not_started() {
        let folder = tempfile::tempdir().unwrap();
        let (program, marker) = marker_program(folder.path());

        let refused = WindowsOpener.open_folder(&program);

        assert!(matches!(refused, Err(OpenError::Failed(_))), "{refused:?}");
        std::thread::sleep(Duration::from_secs(1));
        assert!(!marker.exists(), "the program was started");
    }
}

/// `WebApi` (crates/core): `Ok` with the status and body of every answer the
/// server gives, whatever its status; `Err` only when no answer arrives.
///
/// The tests use the web servers of the `downloader` tests; only the last one
/// reaches Steam's Web API, with a made-up key.
mod web_api {
    use std::net::TcpListener;

    use nicelibrary_core::{WebAnswer, WebApi};
    use nicelibrary_lib::ports::HttpWebApi;

    use super::downloader::serve_once;

    #[test]
    fn answers_with_the_status_and_body_the_server_sends() {
        for (status, code) in [("200 OK", 200), ("403 Forbidden", 403), ("503 Service Unavailable", 503)] {
            let body = format!(r#"{{"response":{{"status":{code}}}}}"#);
            let url = serve_once(status, body.as_bytes());

            assert_eq!(HttpWebApi::new().get(&url), Ok(WebAnswer { status: code, body: body.into_bytes() }), "{status}");
        }
    }

    #[test]
    fn a_server_that_cannot_be_reached_is_an_error() {
        // A port that was free a moment ago and has no server now.
        let address = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();

        assert!(HttpWebApi::new().get(&format!("http://{address}/IStoreService/GetAppList/v1/")).is_err());
    }

    #[test]
    fn steam_answers_a_made_up_key_with_forbidden_over_https() {
        // The core reads 401 and 403 as Steam's refusal of a key (crates/core/src/steam_web_api.rs).
        let url = "https://api.steampowered.com/IStoreService/GetAppList/v1/?key=0123456789ABCDEF0123456789ABCDEF&max_results=1";

        let answer = HttpWebApi::new().get(url).expect("Steam reachable");

        assert!(matches!(answer.status, 401 | 403), "Steam answered HTTP {}", answer.status);
    }

    #[test]
    fn steam_answers_a_sync_with_a_made_up_key_with_forbidden_over_https() {
        // The core reads 401 and 403 during a Sync as a rejected key (crates/core/src/sync.rs).
        let url = "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?key=0123456789ABCDEF0123456789ABCDEF&steamid=76561197960287930&include_appinfo=1&include_played_free_games=1&format=json";

        let answer = HttpWebApi::new().get(url).expect("Steam reachable");

        assert!(matches!(answer.status, 401 | 403), "Steam answered HTTP {}", answer.status);
    }
}

/// `Clock` (crates/core): `now` is the operating system's current time of
/// day, in UTC; `sleep` blocks the calling thread, and only that one, until
/// at least the duration has passed or the wakeup rings, which it uses up.
mod clock {
    use std::sync::Arc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use nicelibrary_core::{Clock, Wakeup};
    use nicelibrary_lib::ports::SystemClock;

    #[test]
    fn now_is_the_time_of_day_of_the_operating_system() {
        let before = SystemTime::now();

        let now = SystemClock.now();

        let after = SystemTime::now();
        assert!(before <= now && now <= after, "{now:?} is not between {before:?} and {after:?}");
    }

    #[test]
    fn now_is_after_this_clock_was_written_and_moves_on() {
        // 2026-11-25 00:00:00 UTC.
        let written = UNIX_EPOCH + Duration::from_secs(1_795_564_800);

        let first = SystemClock.now();
        std::thread::sleep(Duration::from_millis(20));
        let second = SystemClock.now();

        assert!(first > written, "{first:?}");
        assert!(second > first, "{second:?} is not after {first:?}");
    }

    #[test]
    fn sleep_returns_once_at_least_the_duration_has_passed() {
        let started = Instant::now();

        SystemClock.sleep(Duration::from_millis(200), &Wakeup::new());

        let slept = started.elapsed();
        assert!(slept >= Duration::from_millis(200), "returned after {slept:?}");
        assert!(slept < Duration::from_secs(5), "returned only after {slept:?}");
    }

    #[test]
    fn sleep_blocks_only_the_thread_that_sleeps() {
        let started = Instant::now();
        let sleeper = std::thread::spawn(|| SystemClock.sleep(Duration::from_secs(1), &Wakeup::new()));

        let meanwhile = started.elapsed();

        assert!(meanwhile < Duration::from_millis(500), "the calling thread waited {meanwhile:?}");
        sleeper.join().unwrap();
        assert!(started.elapsed() >= Duration::from_secs(1));
    }

    #[test]
    fn a_wakeup_rung_from_another_thread_ends_the_sleep_early() {
        let wakeup = Arc::new(Wakeup::new());
        let started = Instant::now();
        let ringer = {
            let wakeup = Arc::clone(&wakeup);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(200));
                wakeup.ring();
            })
        };

        SystemClock.sleep(Duration::from_secs(60), &wakeup);

        let slept = started.elapsed();
        ringer.join().unwrap();
        assert!(slept >= Duration::from_millis(200), "returned after {slept:?}, before the ring");
        assert!(slept < Duration::from_secs(5), "returned only after {slept:?}");
    }

    #[test]
    fn a_wakeup_rung_before_the_sleep_ends_it_at_once_and_only_that_one() {
        let wakeup = Wakeup::new();
        wakeup.ring();
        let started = Instant::now();

        SystemClock.sleep(Duration::from_secs(60), &wakeup);
        let first = started.elapsed();
        SystemClock.sleep(Duration::from_millis(200), &wakeup);

        assert!(first < Duration::from_secs(1), "the first sleep took {first:?}");
        assert!(started.elapsed() >= Duration::from_millis(200), "the ring ended the second sleep too");
    }
}

/// `SecretStore` (crates/core): `read` answers what the last `write` under
/// the same name stored, also after a restart; `write` replaces; `delete`
/// removes and is `Ok` when nothing is stored.
///
/// The tests keep their secrets in the Windows Credential Manager under
/// `NiceLibraryContractTests/<test>-<process id>/` and delete them again;
/// they never touch the app's own entries.
#[cfg(windows)]
mod secret_store {
    use nicelibrary_core::SecretStore;
    use nicelibrary_lib::ports::WindowsSecretStore;

    /// The secret store of one test, emptied when dropped.
    struct TestStore {
        prefix: String,
        names: Vec<&'static str>,
    }

    impl TestStore {
        fn new(test: &str, names: &[&'static str]) -> Self {
            Self { prefix: format!("NiceLibraryContractTests/{test}-{}/", std::process::id()), names: names.to_vec() }
        }

        /// The store as the app sees it after a start.
        fn open(&self) -> WindowsSecretStore {
            WindowsSecretStore::under(&self.prefix)
        }
    }

    impl Drop for TestStore {
        fn drop(&mut self) {
            for name in &self.names {
                let _ = self.open().delete(name);
            }
        }
    }

    #[test]
    fn a_secret_written_is_read_back_after_a_restart() {
        let test = TestStore::new("roundtrip", &["key"]);

        test.open().write("key", "0123456789ABCDEF0123456789ABCDEF").unwrap();

        assert_eq!(test.open().read("key"), Ok(Some("0123456789ABCDEF0123456789ABCDEF".to_owned())));
    }

    #[test]
    fn nothing_written_reads_as_none() {
        let test = TestStore::new("none", &[]);

        assert_eq!(test.open().read("key"), Ok(None));
    }

    #[test]
    fn a_secret_written_again_replaces_the_first() {
        let test = TestStore::new("replace", &["key"]);
        test.open().write("key", "first").unwrap();

        test.open().write("key", "second").unwrap();

        assert_eq!(test.open().read("key"), Ok(Some("second".to_owned())));
    }

    #[test]
    fn a_deleted_secret_is_gone_and_deleting_again_is_fine() {
        let test = TestStore::new("delete", &["key"]);
        test.open().write("key", "secret").unwrap();

        test.open().delete("key").unwrap();

        assert_eq!(test.open().read("key"), Ok(None));
        assert_eq!(test.open().delete("key"), Ok(()));
    }

    #[test]
    fn names_and_stores_keep_their_secrets_apart() {
        let one = TestStore::new("apart-one", &["key", "other"]);
        let two = TestStore::new("apart-two", &["key"]);

        one.open().write("key", "one").unwrap();
        one.open().write("other", "other").unwrap();
        two.open().write("key", "two").unwrap();

        assert_eq!(one.open().read("key"), Ok(Some("one".to_owned())));
        assert_eq!(one.open().read("other"), Ok(Some("other".to_owned())));
        assert_eq!(two.open().read("key"), Ok(Some("two".to_owned())));
    }

    #[test]
    fn any_text_is_kept_as_it_was_written() {
        let test = TestStore::new("text", &["key"]);
        let text = "Schlüssel 🔑 with spaces and \"quotes\"";

        test.open().write("key", text).unwrap();

        assert_eq!(test.open().read("key"), Ok(Some(text.to_owned())));
    }
}

/// `Autostart` (crates/core): off while there is no entry; `turn_on` makes
/// it on, `turn_off` off again and is `Ok` when there is nothing to remove;
/// an entry removed or disabled outside the app, or one for another program,
/// is off.
///
/// The tests write their entries under
/// `HKCU\Software\NiceLibraryContractTests\autostart-<test>-<process id>`
/// instead of Windows' own startup keys and delete them again; the app's
/// real entry is only read.
#[cfg(windows)]
mod autostart {
    use std::path::PathBuf;

    use nicelibrary_core::Autostart;
    use nicelibrary_lib::ports::WindowsAutostart;
    use winreg::enums::{HKEY_CURRENT_USER, REG_BINARY};
    use winreg::{RegKey, RegValue};

    const PARENT: &str = r"Software\NiceLibraryContractTests";
    const NAME: &str = "NiceLibrary";

    /// The startup keys of one test, removed when dropped.
    struct TestKeys {
        path: String,
    }

    impl TestKeys {
        fn new(test: &str) -> Self {
            let path = format!(r"{PARENT}\autostart-{test}-{}", std::process::id());
            // Created up front, so another test's clean-up no longer removes
            // the parent while it is empty; that may happen while this one
            // creates it, so it tries again.
            let user = RegKey::predef(HKEY_CURRENT_USER);
            let mut created = user.create_subkey(&path);
            for _ in 0..10 {
                if created.is_ok() {
                    break;
                }
                created = user.create_subkey(&path);
            }
            created.unwrap();
            Self { path }
        }

        fn run_key(&self) -> String {
            format!(r"{}\Run", self.path)
        }

        fn approved_key(&self) -> String {
            format!(r"{}\StartupApproved\Run", self.path)
        }

        /// The autostart of `program`, as the app would see it after a start.
        fn autostart_of(&self, program: &str) -> WindowsAutostart {
            WindowsAutostart::under(&self.run_key(), &self.approved_key(), NAME, Some(PathBuf::from(program)))
        }

        fn autostart(&self) -> WindowsAutostart {
            self.autostart_of(PROGRAM)
        }

        /// The command the entry runs, as Windows reads it.
        fn command(&self) -> Option<String> {
            RegKey::predef(HKEY_CURRENT_USER).open_subkey(self.run_key()).ok()?.get_value(NAME).ok()
        }

        /// The user removes the entry outside the app.
        fn remove_entry(&self) {
            let key = RegKey::predef(HKEY_CURRENT_USER).create_subkey(self.run_key()).unwrap().0;
            key.delete_value(NAME).unwrap();
        }

        /// The user turns the entry off or on in the Task Manager's startup
        /// apps, which records it as Windows does: the first of 12 bytes is
        /// odd for disabled.
        fn mark_in_startup_apps(&self, first_byte: u8) {
            let key = RegKey::predef(HKEY_CURRENT_USER).create_subkey(self.approved_key()).unwrap().0;
            let mut bytes = vec![0; 12];
            bytes[0] = first_byte;
            key.set_raw_value(NAME, &RegValue { bytes: bytes.into(), vtype: REG_BINARY }).unwrap();
        }

        fn has_startup_apps_mark(&self) -> bool {
            RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey(self.approved_key())
                .and_then(|key| key.get_raw_value(NAME))
                .is_ok()
        }
    }

    impl Drop for TestKeys {
        fn drop(&mut self) {
            let user = RegKey::predef(HKEY_CURRENT_USER);
            let _ = user.delete_subkey_all(&self.path);
            // Fails while other tests still hold keys below it; the last one removes it.
            let _ = user.delete_subkey(PARENT);
        }
    }

    /// A program path with a space, as under `C:\Program Files`. The tests
    /// never run it.
    const PROGRAM: &str = r"C:\Program Files\NiceLibrary Contract Tests\nicelibrary.exe";

    #[test]
    fn without_an_entry_it_is_off() {
        let keys = TestKeys::new("none");

        assert_eq!(keys.autostart().is_on(), Ok(false));
    }

    #[test]
    fn turning_it_on_adds_an_entry_that_starts_the_program_with_the_arguments() {
        let keys = TestKeys::new("on");

        keys.autostart().turn_on(&["--autostart"]).unwrap();

        assert_eq!(keys.autostart().is_on(), Ok(true));
        assert_eq!(keys.command().as_deref(), Some(format!("\"{PROGRAM}\" --autostart").as_str()));
    }

    #[test]
    fn turning_it_off_removes_the_entry_and_turning_it_off_again_is_fine() {
        let keys = TestKeys::new("off");
        keys.autostart().turn_on(&["--autostart"]).unwrap();

        keys.autostart().turn_off().unwrap();

        assert_eq!(keys.autostart().is_on(), Ok(false));
        assert_eq!(keys.command(), None);
        assert_eq!(keys.autostart().turn_off(), Ok(()));
    }

    #[test]
    fn an_entry_removed_outside_the_app_is_off() {
        let keys = TestKeys::new("removed");
        keys.autostart().turn_on(&["--autostart"]).unwrap();

        keys.remove_entry();

        assert_eq!(keys.autostart().is_on(), Ok(false));
    }

    #[test]
    fn an_entry_disabled_in_the_startup_apps_is_off_until_it_is_turned_on_again() {
        let keys = TestKeys::new("disabled");
        keys.autostart().turn_on(&["--autostart"]).unwrap();

        keys.mark_in_startup_apps(0x03);
        assert_eq!(keys.autostart().is_on(), Ok(false));

        keys.autostart().turn_on(&["--autostart"]).unwrap();
        assert_eq!(keys.autostart().is_on(), Ok(true));
    }

    #[test]
    fn an_entry_enabled_in_the_startup_apps_is_on() {
        let keys = TestKeys::new("enabled");
        keys.autostart().turn_on(&["--autostart"]).unwrap();

        keys.mark_in_startup_apps(0x02);

        assert_eq!(keys.autostart().is_on(), Ok(true));
    }

    #[test]
    fn turning_it_off_also_forgets_the_startup_apps_mark() {
        let keys = TestKeys::new("forget");
        keys.autostart().turn_on(&["--autostart"]).unwrap();
        keys.mark_in_startup_apps(0x03);

        keys.autostart().turn_off().unwrap();

        assert!(!keys.has_startup_apps_mark());
    }

    #[test]
    fn an_entry_for_another_program_is_off_and_turning_it_on_replaces_it() {
        let keys = TestKeys::new("moved");
        keys.autostart_of(r"C:\Old Place\nicelibrary.exe").turn_on(&["--autostart"]).unwrap();

        assert_eq!(keys.autostart().is_on(), Ok(false));

        keys.autostart().turn_on(&["--autostart"]).unwrap();
        assert_eq!(keys.autostart().is_on(), Ok(true));
        assert_eq!(keys.command().as_deref(), Some(format!("\"{PROGRAM}\" --autostart").as_str()));
    }

    #[test]
    fn the_apps_own_entry_can_be_read() {
        // Only reads Windows' startup keys for the running program.
        assert!(WindowsAutostart::new().is_on().is_ok());
    }
}

/// `ExecutablePicker` (crates/core): `Some` with the absolute path of the
/// file the user chose, `None` when they cancel; blocks until they decide.
///
/// The tests play the user: they find the dialog this process opens, type a
/// path into its file name box and confirm, or cancel it. A dialog shows on
/// the screen briefly; nothing on disk changes but a temporary folder.
#[cfg(windows)]
mod executable_picker {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use nicelibrary_core::ExecutablePicker;
    use nicelibrary_lib::ports::WindowsExecutablePicker;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetDlgItem, GetWindowThreadProcessId, IDCANCEL, IDOK, IsWindowVisible,
        PostMessageW, SendMessageW, WM_COMMAND, WM_SETTEXT,
    };

    /// The id of the file name box in Windows' file dialogs (`cmb13`).
    const FILE_NAME_BOX: i32 = 0x47c;

    /// One dialog at a time, so each test finds its own.
    static ONE_DIALOG: Mutex<()> = Mutex::new(());

    fn open_dialog() -> JoinHandle<Result<Option<PathBuf>, String>> {
        std::thread::spawn(|| WindowsExecutablePicker.choose_executable())
    }

    /// The visible file dialog of this process, waiting for it up to ten seconds.
    fn the_dialog() -> HWND {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let mut found: HWND = std::ptr::null_mut();
            // SAFETY: the callback only writes through the pointer it is given, which outlives the call.
            unsafe { EnumWindows(Some(remember_dialog), &mut found as *mut HWND as LPARAM) };
            // The file name box exists once the dialog is ready for input.
            if !found.is_null() && !unsafe { GetDlgItem(found, FILE_NAME_BOX) }.is_null() {
                return found;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("no file dialog appeared");
    }

    unsafe extern "system" fn remember_dialog(window: HWND, found: LPARAM) -> i32 {
        let mut process = 0;
        let mut class = [0u16; 16];
        // SAFETY: plain Win32 calls on a window handle Windows just gave us.
        let (ours, length) = unsafe {
            GetWindowThreadProcessId(window, &mut process);
            (process == std::process::id() && IsWindowVisible(window) != 0, GetClassNameW(window, class.as_mut_ptr(), 16))
        };
        // "#32770" is the window class of dialogs.
        if ours && String::from_utf16_lossy(&class[..length as usize]) == "#32770" {
            // SAFETY: `found` is the `&mut HWND` handed to EnumWindows.
            unsafe { *(found as *mut HWND) = window };
            return 0;
        }
        1
    }

    fn press(dialog: HWND, button: i32) {
        // SAFETY: plain Win32 call; the message carries no pointers.
        unsafe { PostMessageW(dialog, WM_COMMAND, button as WPARAM, 0) };
    }

    #[test]
    fn answers_the_file_the_user_chose() {
        let _one = ONE_DIALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let folder = tempfile::tempdir().unwrap();
        let game = folder.path().join("Celeste.exe");
        fs::write(&game, b"MZ").unwrap();
        let answer = open_dialog();

        let dialog = the_dialog();
        let typed: Vec<u16> = game.display().to_string().encode_utf16().chain([0]).collect();
        // SAFETY: SendMessageW returns only once the dialog has copied the text.
        unsafe { SendMessageW(GetDlgItem(dialog, FILE_NAME_BOX), WM_SETTEXT, 0, typed.as_ptr() as LPARAM) };
        press(dialog, IDOK);

        let chosen = answer.join().unwrap().unwrap().expect("a chosen file");
        assert_eq!(fs::canonicalize(chosen).unwrap(), fs::canonicalize(&game).unwrap());
    }

    #[test]
    fn answers_none_when_the_user_cancels() {
        let _one = ONE_DIALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let answer = open_dialog();

        press(the_dialog(), IDCANCEL);

        assert_eq!(answer.join().unwrap(), Ok(None));
    }

    #[test]
    fn leaves_the_working_folder_where_it_was() {
        let _one = ONE_DIALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = std::env::current_dir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let game = folder.path().join("game.exe");
        fs::write(&game, b"MZ").unwrap();
        let answer = open_dialog();

        let dialog = the_dialog();
        let typed: Vec<u16> = game.display().to_string().encode_utf16().chain([0]).collect();
        // SAFETY: SendMessageW returns only once the dialog has copied the text.
        unsafe { SendMessageW(GetDlgItem(dialog, FILE_NAME_BOX), WM_SETTEXT, 0, typed.as_ptr() as LPARAM) };
        press(dialog, IDOK);
        answer.join().unwrap().unwrap();

        assert_eq!(std::env::current_dir().unwrap(), before);
    }
}

/// `ProcessList` (crates/core): the absolute path of the executable of every
/// running process the user may see, once per process; a process that has
/// ended is no longer listed.
///
/// The tests copy this test program into a temporary folder and start the
/// copy as a stand-in Game that only waits, then end it again; they stop no
/// process but the one they started.
#[cfg(windows)]
mod process_list {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command};
    use std::time::{Duration, Instant};

    use nicelibrary_core::ProcessList;
    use nicelibrary_lib::ports::WindowsProcessList;

    /// Not a test of its own: the body of the stand-in Game, which runs
    /// until it is ended.
    #[test]
    #[ignore = "started by the process list tests as a stand-in Game"]
    fn stand_in_game() {
        std::thread::sleep(Duration::from_secs(120));
    }

    /// A running copy of this test program at its own path; ended when dropped.
    struct StandInGame {
        child: Child,
        executable: PathBuf,
        /// The folder of the copy, held by the stand-in that made it.
        _folder: Option<tempfile::TempDir>,
    }

    impl StandInGame {
        fn start() -> Self {
            let folder = tempfile::tempdir().unwrap();
            let executable = folder.path().join("StandInGame.exe");
            fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
            Self { child: run(&executable), executable, _folder: Some(folder) }
        }

        /// One more stand-in Game of the same executable.
        fn start_again(&self) -> Self {
            Self { child: run(&self.executable), executable: self.executable.clone(), _folder: None }
        }

        fn end(&mut self) {
            // Only the process this test started, through its own handle.
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    impl Drop for StandInGame {
        fn drop(&mut self) {
            self.end();
        }
    }

    fn run(executable: &Path) -> Child {
        Command::new(executable)
            .args(["--ignored", "--exact", "process_list::stand_in_game", "--quiet"])
            .spawn()
            .unwrap()
    }

    fn listed(executable: &Path) -> usize {
        let wanted = fs::canonicalize(executable).unwrap();
        WindowsProcessList
            .running_executables()
            .unwrap()
            .iter()
            .filter(|listed| fs::canonicalize(listed).ok().as_ref() == Some(&wanted))
            .count()
    }

    /// Waits up to ten seconds for `executable` to be listed `times` times.
    fn until_listed(executable: &Path, times: usize) -> usize {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let count = listed(executable);
            if count == times || Instant::now() > deadline {
                return count;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    #[test]
    fn lists_a_started_process_by_its_executable_until_it_ends() {
        let mut game = StandInGame::start();
        assert_eq!(until_listed(&game.executable, 1), 1, "the stand-in Game runs");

        game.end();

        assert_eq!(listed(&game.executable), 0, "the stand-in Game has ended");
    }

    #[test]
    fn lists_each_process_of_one_executable() {
        let first = StandInGame::start();
        let _second = first.start_again();

        assert_eq!(until_listed(&first.executable, 2), 2);
    }

    #[test]
    fn lists_absolute_paths_and_this_test_program_among_them() {
        let running = WindowsProcessList.running_executables().unwrap();

        assert!(running.iter().all(|path| path.is_absolute()), "{running:?}");
        let this = fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        assert!(running.iter().any(|path| fs::canonicalize(path).ok().as_ref() == Some(&this)), "{running:?}");
    }
}
