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

/// `ExecutablePicker` and `ImagePicker` (crates/core): `Some` with the
/// absolute path of the file the user chose, `None` when they cancel;
/// blocks until they decide.
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

    use nicelibrary_core::{ExecutablePicker, ImagePicker};
    use nicelibrary_lib::ports::{WindowsExecutablePicker, WindowsImagePicker};
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

    #[test]
    fn answers_the_image_the_user_chose() {
        let _one = ONE_DIALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let folder = tempfile::tempdir().unwrap();
        let image = folder.path().join("Celeste cover.png");
        fs::write(&image, b"\x89PNG\r\n\x1a\n").unwrap();
        let answer = std::thread::spawn(|| WindowsImagePicker.choose_image());

        let dialog = the_dialog();
        let typed: Vec<u16> = image.display().to_string().encode_utf16().chain([0]).collect();
        // SAFETY: SendMessageW returns only once the dialog has copied the text.
        unsafe { SendMessageW(GetDlgItem(dialog, FILE_NAME_BOX), WM_SETTEXT, 0, typed.as_ptr() as LPARAM) };
        press(dialog, IDOK);

        let chosen = answer.join().unwrap().unwrap().expect("a chosen image");
        assert_eq!(fs::canonicalize(chosen).unwrap(), fs::canonicalize(&image).unwrap());
    }

    #[test]
    fn answers_no_image_when_the_user_cancels() {
        let _one = ONE_DIALOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let answer = std::thread::spawn(|| WindowsImagePicker.choose_image());

        press(the_dialog(), IDCANCEL);

        assert_eq!(answer.join().unwrap(), Ok(None));
    }
}

/// `IconExtractor` (crates/core): `Some` with the largest image of the
/// executable's first icon, at its own size, as straight RGBA; `None` for an
/// executable without an icon; `Err` for a file that is not an executable.
///
/// The tests copy Windows' `cmd.exe` into a temporary folder and replace all
/// resources of the copy with icons generated for the test; the original is
/// only read. One test reads the icon of Windows' own `explorer.exe`.
#[cfg(windows)]
mod icon_extractor {
    use std::fs;
    use std::path::{Path, PathBuf};

    use nicelibrary_core::{Icon, IconExtractor};
    use nicelibrary_lib::ports::WindowsIconExtractor;
    use tempfile::TempDir;
    use windows_sys::Win32::System::LibraryLoader::{BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{RT_GROUP_ICON, RT_ICON};
    use windows_sys::core::PCWSTR;

    const RED: [u8; 4] = [220, 30, 40, 255];
    const GREEN: [u8; 4] = [30, 180, 60, 255];
    const BLUE: [u8; 4] = [30, 60, 220, 255];

    /// One image of an icon as an executable keeps it.
    struct Image {
        size: u32,
        bit_count: u16,
        bytes: Vec<u8>,
    }

    /// A square PNG image in one colour; the left half transparent when `half_transparent`.
    fn png_image(size: u32, rgba: [u8; 4], half_transparent: bool) -> Image {
        let mut pixels = Vec::new();
        for _ in 0..size {
            for x in 0..size {
                pixels.extend(if half_transparent && x < size / 2 { [0, 0, 0, 0] } else { rgba });
            }
        }
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&pixels).unwrap();
        writer.finish().unwrap();
        Image { size, bit_count: 32, bytes }
    }

    /// A square bitmap image in one colour as icons keep them: the colours,
    /// bottom row first, then a mask whose set bits are transparent. With 32
    /// bits per pixel the alpha channel says what is transparent; with 24
    /// the left half is masked out.
    fn bitmap_image(size: u32, rgba: [u8; 4], bit_count: u16) -> Image {
        let mut bytes = Vec::new();
        bytes.extend(40u32.to_le_bytes());
        bytes.extend((size as i32).to_le_bytes());
        bytes.extend((size as i32 * 2).to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(bit_count.to_le_bytes());
        bytes.extend([0u8; 24]);
        let masked = |x: u32| bit_count == 24 && x < size / 2;
        for _ in 0..size {
            let mut row = Vec::new();
            for x in 0..size {
                let [r, g, b, a] = if masked(x) { [0; 4] } else { rgba };
                row.extend(if bit_count == 32 { vec![b, g, r, a] } else { vec![b, g, r] });
            }
            row.resize(row.len().div_ceil(4) * 4, 0);
            bytes.extend(row);
        }
        let mask_row = size.div_ceil(32) as usize * 4;
        for _ in 0..size {
            let mut row = vec![0u8; mask_row];
            for x in 0..size {
                if masked(x) {
                    row[x as usize / 8] |= 0x80 >> (x % 8);
                }
            }
            bytes.extend(row);
        }
        Image { size, bit_count, bytes }
    }

    /// A copy of Windows' `cmd.exe` whose resources are only `icons`, each a
    /// list of images; the icons are numbered from 1 in order, their images
    /// from 100 on.
    fn executable_with(folder: &TempDir, icons: &[Vec<Image>]) -> PathBuf {
        let windows = std::env::var_os("SystemRoot").expect("SystemRoot is set on Windows");
        let path = folder.path().join("game.exe");
        fs::copy(Path::new(&windows).join("System32").join("cmd.exe"), &path).unwrap();
        let wide: Vec<u16> = path.to_str().unwrap().encode_utf16().chain([0]).collect();
        let mut next_image = 100u16;
        // SAFETY: plain Win32 calls; every buffer outlives the call it is handed to.
        unsafe {
            // TRUE: the copy's own resources, its icon among them, are removed.
            let update = BeginUpdateResourceW(wide.as_ptr(), 1);
            assert!(!update.is_null(), "{}", std::io::Error::last_os_error());
            for (number, images) in icons.iter().enumerate() {
                let mut group = Vec::new();
                group.extend([0u16, 1, images.len() as u16].iter().flat_map(|value| value.to_le_bytes()));
                for image in images {
                    let id = next_image;
                    next_image += 1;
                    let added = UpdateResourceW(
                        update,
                        RT_ICON,
                        id as usize as PCWSTR,
                        0,
                        image.bytes.as_ptr().cast(),
                        image.bytes.len() as u32,
                    );
                    assert_ne!(added, 0, "{}", std::io::Error::last_os_error());
                    let width = if image.size >= 256 { 0 } else { image.size as u8 };
                    group.extend([width, width, 0, 0]);
                    group.extend(1u16.to_le_bytes());
                    group.extend(image.bit_count.to_le_bytes());
                    group.extend((image.bytes.len() as u32).to_le_bytes());
                    group.extend(id.to_le_bytes());
                }
                let name = (number + 1) as PCWSTR;
                let added = UpdateResourceW(update, RT_GROUP_ICON, name, 0, group.as_ptr().cast(), group.len() as u32);
                assert_ne!(added, 0, "{}", std::io::Error::last_os_error());
            }
            assert_ne!(EndUpdateResourceW(update, 0), 0, "{}", std::io::Error::last_os_error());
        }
        path
    }

    fn pixel(icon: &Icon, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * icon.width + x) * 4) as usize;
        icon.rgba[at..at + 4].try_into().unwrap()
    }

    fn largest_icon(path: &Path) -> Icon {
        WindowsIconExtractor.largest_icon(path).unwrap().expect("an icon")
    }

    #[test]
    fn answers_the_largest_image_of_the_icon_at_its_own_size() {
        let folder = tempfile::tempdir().unwrap();
        let game = executable_with(
            &folder,
            &[vec![bitmap_image(32, BLUE, 32), png_image(256, RED, false), bitmap_image(48, GREEN, 32)]],
        );

        let icon = largest_icon(&game);

        assert_eq!((icon.width, icon.height, icon.rgba.len()), (256, 256, 256 * 256 * 4));
        assert_eq!(pixel(&icon, 128, 128), RED);
    }

    #[test]
    fn an_icon_with_bitmaps_only_answers_its_largest_bitmap() {
        let folder = tempfile::tempdir().unwrap();
        let game = executable_with(&folder, &[vec![bitmap_image(48, GREEN, 32), bitmap_image(32, BLUE, 32)]]);

        let icon = largest_icon(&game);

        assert_eq!((icon.width, icon.height), (48, 48));
        assert_eq!(pixel(&icon, 0, 0), GREEN);
        assert_eq!(pixel(&icon, 47, 47), GREEN);
    }

    #[test]
    fn transparent_pixels_stay_transparent() {
        let folder = tempfile::tempdir().unwrap();
        let game = executable_with(&folder, &[vec![png_image(256, RED, true)]]);

        let icon = largest_icon(&game);

        assert_eq!(pixel(&icon, 10, 128)[3], 0, "the left half is transparent");
        assert_eq!(pixel(&icon, 200, 128), RED);
    }

    #[test]
    fn an_icon_without_alpha_is_transparent_where_its_mask_says() {
        let folder = tempfile::tempdir().unwrap();
        let game = executable_with(&folder, &[vec![bitmap_image(32, BLUE, 24)]]);

        let icon = largest_icon(&game);

        assert_eq!((icon.width, icon.height), (32, 32));
        assert_eq!(pixel(&icon, 3, 16)[3], 0, "the masked half is transparent");
        assert_eq!(pixel(&icon, 28, 16), BLUE);
    }

    #[test]
    fn the_first_icon_is_the_one_answered() {
        let folder = tempfile::tempdir().unwrap();
        let game = executable_with(&folder, &[vec![bitmap_image(32, BLUE, 32)], vec![png_image(256, RED, false)]]);

        let icon = largest_icon(&game);

        assert_eq!(pixel(&icon, 16, 16), BLUE);
    }

    #[test]
    fn an_executable_without_an_icon_has_none() {
        let folder = tempfile::tempdir().unwrap();
        let game = executable_with(&folder, &[]);

        assert_eq!(WindowsIconExtractor.largest_icon(&game), Ok(None));
    }

    #[test]
    fn a_file_that_is_not_an_executable_is_an_error() {
        let folder = tempfile::tempdir().unwrap();
        let notes = folder.path().join("notes.exe");
        fs::write(&notes, b"not an executable").unwrap();

        assert!(WindowsIconExtractor.largest_icon(&notes).is_err());
    }

    #[test]
    fn a_file_that_is_gone_is_an_error() {
        let folder = tempfile::tempdir().unwrap();

        assert!(WindowsIconExtractor.largest_icon(&folder.path().join("gone.exe")).is_err());
    }

    #[test]
    fn reads_the_icon_of_a_program_of_windows() {
        let windows = std::env::var_os("SystemRoot").expect("SystemRoot is set on Windows");

        let icon = largest_icon(&Path::new(&windows).join("explorer.exe"));

        assert_eq!(icon.width, icon.height);
        assert!(icon.width >= 48, "{} pixels is not the largest size", icon.width);
        assert!(icon.rgba.chunks(4).any(|pixel| pixel[3] == 255), "something is opaque");
    }
}
