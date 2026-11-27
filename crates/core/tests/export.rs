//! Exporting the data: everything that is the person's own goes into one
//! Export File they choose, as a backup or to move to another computer. That
//! is their Local Games with how they are launched and the Cover images they
//! chose, the Sessions, Playtime Adjustments, Manual Progress, Statuses,
//! Favorites, Hidden Games, Collections and the settings. The file is
//! versioned (docs/export-format.md). The Steam Web API key never goes into
//! it, nor anything the app can fetch again: Steam's figures and cached
//! images.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use nicelibrary_core::{
    AddLocalGame, Clock, Core, CoverCache, CoverImageChoice, DataExport, Downloader, ExecutablePicker,
    DataFilePicker, Language, LaunchOptions, LibraryLayout, LibrarySort, OpenError, Opener, Ports, ProcessList,
    Status, SteamLocator, Store, Theme, Tracking, Wakeup,
};
use serde_json::{Value, json};
use tempfile::TempDir;

/// The Steam Web API key the secret store holds in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";

/// The time of day on the test's day, in seconds since the Unix epoch.
fn at(hours: u64, minutes: u64) -> u64 {
    // 2027-01-15 00:00:00 UTC.
    1_800_000_000 - 1_800_000_000 % 86_400 + hours * 3600 + minutes * 60
}

#[test]
fn the_export_file_is_written_where_the_person_chose_and_names_its_format_and_version() {
    let app = App::start();
    let file = app.disk.path().join("backup.json");
    app.user.saves_to(&file);

    let exported = app.core.export_data().unwrap();

    assert_eq!(exported, DataExport::Exported { file: file.display().to_string() });
    let export = read(&file);
    assert_eq!(export["format"], "nicelibrary-export");
    assert_eq!(export["version"], 1);
    assert_eq!(export["appVersion"], app.core.about().version);
}

#[test]
fn the_person_is_offered_a_file_name_with_todays_date() {
    let app = App::start();
    app.set_time(at(18, 30));
    app.user.saves_to(&app.disk.path().join("backup.json"));

    app.core.export_data().unwrap();

    assert_eq!(app.user.offered(), ["NiceLibrary-2027-01-15.json"]);
    assert_eq!(read(&app.disk.path().join("backup.json"))["exportedAt"], at(18, 30));
}

#[test]
fn cancelling_writes_no_file() {
    let app = App::start();

    assert_eq!(app.core.export_data(), Ok(DataExport::Cancelled));
    assert_eq!(fs::read_dir(app.disk.path()).unwrap().count(), 0);
}

#[test]
fn a_file_dialog_that_cannot_be_shown_is_an_error() {
    let app = App::start();
    app.user.sees_no_dialog();

    assert!(app.core.export_data().is_err());
}

#[test]
fn a_file_that_cannot_be_written_is_an_error_and_leaves_nothing_behind() {
    let app = App::start();
    let folder = app.disk.path().join("gone");
    app.user.saves_to(&folder.join("backup.json"));

    assert!(app.core.export_data().is_err());
    assert!(!folder.exists());
}

#[test]
fn an_existing_file_is_replaced_by_the_export() {
    let app = App::start();
    let file = app.disk.path().join("backup.json");
    fs::write(&file, "an older backup that is much longer than the new one ".repeat(1000)).unwrap();
    app.user.saves_to(&file);

    app.core.export_data().unwrap();

    assert_eq!(read(&file)["format"], "nicelibrary-export");
}

#[test]
fn local_games_are_exported_with_how_they_are_launched_their_manual_progress_and_playtime_adjustment() {
    let app = App::start();
    let celeste = app.add("Games/Celeste/Celeste.exe", "Celeste");
    let hades = app.add("Games/Hades/Hades.exe", "Hades");
    let options = LaunchOptions { arguments: r#"-windowed "--save slot""#.into(), working_directory: Some(r"D:\Saves".into()) };
    app.core.set_launch_options(&celeste, &options).unwrap();
    app.core.set_manual_progress(&celeste, Some(40)).unwrap();
    app.core.set_playtime_adjustment(&celeste, Some(-15)).unwrap();

    let export = app.export();

    assert_eq!(
        export["localGames"],
        json!([
            {
                "id": celeste,
                "title": "Celeste",
                "executable": app.executable(&celeste),
                "launchArguments": r#"-windowed "--save slot""#,
                "workingDirectory": r"D:\Saves",
                "manualProgress": 40,
                "playtimeAdjustmentMinutes": -15,
                "coverImage": null,
                "coverMatch": null,
            },
            {
                "id": hades,
                "title": "Hades",
                "executable": app.executable(&hades),
                "launchArguments": "",
                "workingDirectory": null,
                "manualProgress": null,
                "playtimeAdjustmentMinutes": null,
                "coverImage": null,
                "coverMatch": null,
            },
        ])
    );
}

#[test]
fn the_sessions_of_local_games_are_exported_one_by_one() {
    let app = App::start();
    let celeste = app.add("Games/Celeste/Celeste.exe", "Celeste");
    let hades = app.add("Games/Hades/Hades.exe", "Hades");
    app.play(&celeste, at(10, 0), at(10, 45));
    app.play(&hades, at(12, 0), at(13, 0));
    app.play(&celeste, at(20, 0), at(20, 30));

    let export = app.export();

    assert_eq!(
        export["sessions"],
        json!([
            { "gameId": celeste, "startedAt": at(10, 0), "endedAt": at(10, 45) },
            { "gameId": hades, "startedAt": at(12, 0), "endedAt": at(13, 0) },
            { "gameId": celeste, "startedAt": at(20, 0), "endedAt": at(20, 30) },
        ])
    );
}

#[test]
fn a_cover_image_the_person_chose_is_exported_but_no_cover_the_app_found_itself() {
    let app = App::start();
    let celeste = app.add("Games/Celeste/Celeste.exe", "Celeste");
    let hades = app.add("Games/Hades/Hades.exe", "Hades");
    let image = app.put("Pictures/celeste.png", PNG);
    app.images.choose(&image);
    assert_eq!(app.core.choose_cover_image(&celeste), Ok(CoverImageChoice::Chosen));
    // Fetching Covers fills the cache, which is not exported.
    let _ = app.core.cover(&hades);

    let export = app.export();

    let games = export["localGames"].as_array().unwrap();
    assert_eq!(games[0]["coverImage"]["mediaType"], "image/png");
    let data = base64::engine::general_purpose::STANDARD.decode(games[0]["coverImage"]["data"].as_str().unwrap());
    assert_eq!(data.unwrap(), PNG);
    assert_eq!(games[1]["coverImage"], Value::Null);
}

#[test]
fn statuses_favorites_hidden_games_and_collections_are_exported_for_steam_and_local_games() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    steam.install(400, "Portal");
    let app = App::start_with(Some(&steam));
    app.core.read_installed_games().unwrap();
    let celeste = app.add("Games/Celeste/Celeste.exe", "Celeste");
    app.core.set_status("steam:620", Status::Completed).unwrap();
    app.core.set_status(&celeste, Status::Playing).unwrap();
    app.core.mark_favorite("steam:620", true).unwrap();
    app.core.mark_favorite(&celeste, true).unwrap();
    app.core.hide_game("steam:400").unwrap();
    let puzzles = collection_id(app.core.create_collection("Puzzles").unwrap());
    let _empty = collection_id(app.core.create_collection("Someday").unwrap());
    app.core.add_game_to_collection(puzzles, "steam:620").unwrap();
    app.core.add_game_to_collection(puzzles, &celeste).unwrap();

    let export = app.export();

    assert_eq!(
        export["statuses"],
        json!([{ "gameId": celeste, "status": "playing" }, { "gameId": "steam:620", "status": "completed" }])
    );
    assert_eq!(export["favorites"], json!([celeste, "steam:620"]));
    assert_eq!(export["hiddenGames"], json!(["steam:400"]));
    assert_eq!(
        export["collections"],
        json!([{ "name": "Puzzles", "gameIds": [celeste, "steam:620"] }, { "name": "Someday", "gameIds": [] }])
    );
}

#[test]
fn the_settings_are_exported() {
    let steam = SteamFixture::new();
    let app = App::start_with(Some(&steam));
    app.core.choose_steam_account("76561197960287930").unwrap();
    app.core.choose_theme(Theme::Light).unwrap();
    app.core.choose_language(Language::German).unwrap();
    app.core.choose_library_layout(LibraryLayout::List).unwrap();
    app.core.choose_library_sort(LibrarySort::Playtime).unwrap();
    app.core.set_close_to_tray(false).unwrap();
    app.core.set_start_with_windows(true).unwrap();

    let export = app.export();

    assert_eq!(
        export["settings"],
        json!({
            "chosenSteamAccount": "76561197960287930",
            "theme": "light",
            "language": "de",
            "libraryLayout": "list",
            "librarySort": "playtime",
            "closeToTray": false,
            "startWithWindows": true,
        })
    );
}

#[test]
fn a_fresh_app_exports_its_default_settings_and_nothing_else() {
    let app = App::start();

    let export = app.export();

    assert_eq!(
        export["settings"],
        json!({
            "chosenSteamAccount": null,
            "theme": "system",
            "language": "system",
            "libraryLayout": "coverGrid",
            "librarySort": "recentlyPlayed",
            "closeToTray": true,
            "startWithWindows": false,
        })
    );
    for list in ["localGames", "sessions", "statuses", "favorites", "hiddenGames", "collections"] {
        assert_eq!(export[list], json!([]), "{list}");
    }
}

#[test]
fn the_steam_web_api_key_and_steams_figures_are_never_exported() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start_with(Some(&steam));
    app.core.read_installed_games().unwrap();
    assert_eq!(app.core.steam_web_api_key_stored(), Ok(true));
    app.user.saves_to(&app.disk.path().join("backup.json"));

    app.core.export_data().unwrap();

    let text = fs::read_to_string(app.disk.path().join("backup.json")).unwrap();
    assert!(!text.contains(KEY), "the key is in the export");
    assert!(!text.to_lowercase().contains(&KEY.to_lowercase()));
    assert!(!text.contains("Portal 2"), "Steam's own data comes back from Steam");
}

fn read(file: &Path) -> Value {
    serde_json::from_slice(&fs::read(file).unwrap()).expect("the export is JSON")
}

fn collection_id(naming: nicelibrary_core::CollectionNaming) -> i64 {
    match naming {
        nicelibrary_core::CollectionNaming::Named { id } => id,
        other => panic!("expected a named Collection, got {other:?}"),
    }
}

/// A small PNG image: 1 × 1 pixel.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49,
    0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF, 0x89, 0x99, 0x3D,
    0x1D, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// One running app with a disk of its own that games and files can be put on.
struct App {
    core: Core,
    processes: ScriptedProcesses,
    user: UserSaves,
    images: UserChoosesImage,
    profile: TempDir,
    disk: TempDir,
    /// The executable of each Local Game added, by its id.
    executables: RefCell<HashMap<String, PathBuf>>,
}

impl App {
    fn start() -> Self {
        Self::start_with(None)
    }

    fn start_with(steam: Option<&SteamFixture>) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let processes =
            ScriptedProcesses { looks: Arc::default(), clock: ScriptedClock(Arc::new(Mutex::new(UNIX_EPOCH))) };
        let user = UserSaves::default();
        let images = UserChoosesImage::default();
        let steam = steam.map(|steam| steam.0.path().to_path_buf());
        let core = Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam)),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(KeyStoredNoWebApi),
                web_api: Box::new(KeyStoredNoWebApi),
                clock: Box::new(processes.clock.clone()),
                executable_picker: Box::new(NoExecutableChosen),
                processes: Box::new(processes.clone()),
                autostart: Box::new(Autostart::default()),
                process_starter: Box::new(NothingStarted),
                icon_extractor: Box::new(NoIcons),
                image_picker: Box::new(images.clone()),
                data_file_picker: Box::new(user.clone()),
            },
            Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.path().join("cache").join("covers"))
                .with_chosen_images_in(profile.path().join("covers")),
        );
        Self {
            core,
            processes,
            user,
            images,
            profile,
            disk: tempfile::tempdir().unwrap(),
            executables: RefCell::default(),
        }
    }

    fn set_time(&self, seconds: u64) {
        *self.processes.clock.0.lock().unwrap() = UNIX_EPOCH + Duration::from_secs(seconds);
    }

    /// Exports into a file of the app's profile and answers what it holds.
    fn export(&self) -> Value {
        let file = self.profile.path().join("export.json");
        self.user.saves_to(&file);
        assert!(matches!(self.core.export_data(), Ok(DataExport::Exported { .. })));
        read(&file)
    }

    /// Puts `bytes` at `relative` (folders separated by `/`) on the disk.
    fn put(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = relative.split('/').fold(self.disk.path().to_path_buf(), |path, part| path.join(part));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }

    /// Puts an executable at `relative` on the disk, adds it as a Local
    /// Game titled `title` and answers its id.
    fn add(&self, relative: &str, title: &str) -> String {
        let path = self.put(relative, b"MZ");
        let AddLocalGame::Added { game } = self.core.add_local_game(path.to_str().unwrap(), title).unwrap() else {
            panic!("expected {title} to be added");
        };
        self.executables.borrow_mut().insert(game.id.clone(), path);
        game.id
    }

    fn executable(&self, id: &str) -> String {
        self.executables.borrow()[id].display().to_string()
    }

    /// Records one Session of the Local Game `id` from `from` to `to`.
    fn play(&self, id: &str, from: u64, to: u64) {
        let executable = self.executables.borrow()[id].clone();
        {
            let mut looks = self.processes.looks.lock().unwrap();
            looks.push_back((from, vec![executable.clone()]));
            looks.push_back((to, vec![executable]));
            looks.push_back((to + 1, Vec::new()));
        }
        self.core.track_sessions(|event| {
            if matches!(event, Tracking::Failed { .. }) {
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        });
    }
}

/// A substitute for the operating system's "Save as" dialog: answers the
/// file the test says the user chose, that they cancelled, or that no
/// dialog could be shown, and remembers the file names it offered.
#[derive(Clone)]
struct UserSaves {
    answer: Arc<Mutex<Result<Option<PathBuf>, String>>>,
    offered: Arc<Mutex<Vec<String>>>,
}

impl Default for UserSaves {
    /// Until the test says otherwise, the user cancels.
    fn default() -> Self {
        Self { answer: Arc::new(Mutex::new(Ok(None))), offered: Arc::default() }
    }
}

impl UserSaves {
    fn saves_to(&self, file: &Path) {
        *self.answer.lock().unwrap() = Ok(Some(file.to_path_buf()));
    }

    fn sees_no_dialog(&self) {
        *self.answer.lock().unwrap() = Err("no dialog today".into());
    }

    fn offered(&self) -> Vec<String> {
        self.offered.lock().unwrap().clone()
    }
}

impl DataFilePicker for UserSaves {
    fn choose_export_file(&self, suggested_name: &str) -> Result<Option<PathBuf>, String> {
        self.offered.lock().unwrap().push(suggested_name.to_owned());
        self.answer.lock().unwrap().clone()
    }

    fn choose_import_file(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// A substitute for the image dialog: answers the image the test names.
#[derive(Clone, Default)]
struct UserChoosesImage(Arc<Mutex<Option<PathBuf>>>);

impl UserChoosesImage {
    fn choose(&self, image: &Path) {
        *self.0.lock().unwrap() = Some(image.to_path_buf());
    }
}

impl nicelibrary_core::ImagePicker for UserChoosesImage {
    fn choose_image(&self) -> Result<Option<PathBuf>, String> {
        Ok(self.0.lock().unwrap().clone())
    }
}

/// The current user's autostart entry, off until the test turns it on.
#[derive(Default)]
struct Autostart(Mutex<bool>);

impl nicelibrary_core::Autostart for Autostart {
    fn is_on(&self) -> Result<bool, String> {
        Ok(*self.0.lock().unwrap())
    }

    fn turn_on(&self, _arguments: &[&str]) -> Result<(), String> {
        *self.0.lock().unwrap() = true;
        Ok(())
    }

    fn turn_off(&self) -> Result<(), String> {
        *self.0.lock().unwrap() = false;
        Ok(())
    }
}

/// A clock the process list sets: each look happens at the time the script says.
#[derive(Clone)]
struct ScriptedClock(Arc<Mutex<SystemTime>>);

impl Clock for ScriptedClock {
    fn now(&self) -> SystemTime {
        *self.0.lock().unwrap()
    }

    fn sleep(&self, _duration: Duration, _wakeup: &Wakeup) {}
}

/// A substitute for the operating system's process list: answers each look
/// as scripted, at the scripted time. Once the script is over, it fails.
#[derive(Clone)]
struct ScriptedProcesses {
    looks: Arc<Mutex<VecDeque<(u64, Vec<PathBuf>)>>>,
    clock: ScriptedClock,
}

impl ProcessList for ScriptedProcesses {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        let Some((at, running)) = self.looks.lock().unwrap().pop_front() else {
            return Err("the script is over".into());
        };
        *self.clock.0.lock().unwrap() = UNIX_EPOCH + Duration::from_secs(at);
        Ok(running)
    }
}

/// Steam on disk, as the test sets it up, at the path it names.
struct SteamAt(Option<PathBuf>);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.0.clone()
    }
}

/// Steam on disk with one Steam Account that has logged in.
struct SteamFixture(TempDir);

impl SteamFixture {
    fn new() -> Self {
        let steam = tempfile::tempdir().unwrap();
        fs::create_dir_all(steam.path().join("config")).unwrap();
        fs::write(
            steam.path().join("config").join("loginusers.vdf"),
            r#""users" { "76561197960287930" { "PersonaName" "Player" "MostRecent" "1" } }"#,
        )
        .unwrap();
        Self(steam)
    }

    /// Writes an app manifest as Steam does for a fully installed app.
    fn install(&self, app_id: u32, name: &str) {
        let steamapps = self.0.path().join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"{name}\"\n}}\n"
            ),
        )
        .unwrap();
    }
}

/// A computer without a network connection.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// A computer that opens no URL; these tests launch nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// A computer whose secret store holds a Steam Web API key and whose
/// network reaches no Web API.
struct KeyStoredNoWebApi;

impl nicelibrary_core::SecretStore for KeyStoredNoWebApi {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(Some(KEY.into()))
    }

    fn write(&self, _name: &str, _secret: &str) -> Result<(), String> {
        Err("this test stores no secret".into())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }
}

impl nicelibrary_core::WebApi for KeyStoredNoWebApi {
    fn get(&self, _url: &str) -> Result<nicelibrary_core::WebAnswer, String> {
        Err("this test asks no Web API".into())
    }
}

/// No file dialog is needed here; Games are added by path.
struct NoExecutableChosen;

impl ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// A computer that starts no process; these tests launch no Local Game.
struct NothingStarted;

impl nicelibrary_core::ProcessStarter for NothingStarted {
    fn start(&self, executable: &Path, _arguments: &[String], _working_directory: &Path) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}

/// A computer whose executables show no icon.
struct NoIcons;

impl nicelibrary_core::IconExtractor for NoIcons {
    fn largest_icon(&self, _executable: &Path) -> Result<Option<nicelibrary_core::Icon>, String> {
        Ok(None)
    }
}
