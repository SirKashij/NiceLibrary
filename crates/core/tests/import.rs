//! Importing the data: the person chooses an Export File, on the same or
//! another computer, and gets back everything that is their own. The import
//! replaces their Local Games with how they are launched and the Cover
//! images they chose, the Sessions, Playtime Adjustments, Manual Progress,
//! Statuses, Favorites, Hidden Games, Collections and the settings with what
//! the file holds; Steam Games come from Steam and stay, and the Steam Web
//! API key is never touched. Each Local Game gets a new number here, and
//! everything that names it follows. A file of a newer or unknown version,
//! or a damaged one, is refused and changes nothing (docs/export-format.md).

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nicelibrary_core::{
    AddLocalGame, Clock, Core, CoverCache, CoverImageChoice, DataExport, DataImport, Downloader, ExecutablePicker,
    ExportFilePicker, Language, LaunchOptions, LibraryLayout, LibrarySort, OpenError, Opener, Ports, ProcessList,
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
fn an_export_imported_on_an_empty_installation_reproduces_the_same_library() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    steam.install(400, "Portal");
    let before = App::start_with(Some(&steam));
    before.fill_with_everything();
    let file = before.export_to_disk();

    let after = App::start_with(Some(&steam));
    after.core.read_installed_games().unwrap();
    let imported = after.import(&file);

    assert_eq!(imported, DataImport::Imported { file: file.display().to_string(), local_games: 2 });
    assert_eq!(after.core.library().unwrap(), before.core.library().unwrap());
    assert_eq!(after.core.collections().unwrap(), before.core.collections().unwrap());
    assert_eq!(without_time(after.export()), without_time(before.export()));
}

#[test]
fn the_cover_image_the_person_chose_comes_back() {
    let before = App::start();
    let celeste = before.add("Games/Celeste/Celeste.exe", "Celeste");
    before.images.choose(&before.put("Pictures/celeste.png", PNG));
    assert_eq!(before.core.choose_cover_image(&celeste), Ok(CoverImageChoice::Chosen));
    let file = before.export_to_disk();

    let after = App::start();
    after.import(&file);

    let games = after.core.library().unwrap();
    let cover = after.core.cover(&games[0].id).unwrap().expect("the chosen Cover image");
    assert_eq!(cover.bytes, PNG);
}

#[test]
fn local_games_get_new_numbers_and_everything_that_names_them_follows() {
    let before = App::start();
    let celeste = before.add("Games/Celeste/Celeste.exe", "Celeste");
    before.core.set_status(&celeste, Status::Playing).unwrap();
    before.core.mark_favorite(&celeste, true).unwrap();
    let puzzles = collection_id(before.core.create_collection("Puzzles").unwrap());
    before.core.add_game_to_collection(puzzles, &celeste).unwrap();
    before.play(&celeste, at(10, 0), at(11, 0));
    let file = before.export_to_disk();
    // Here the first numbers are taken already.
    let after = App::start();
    let gone = after.add("Games/Old/Old.exe", "Old");
    after.core.remove_local_game(&gone).unwrap();

    after.import(&file);

    let games = after.core.library().unwrap();
    assert_eq!(games.len(), 1);
    let game = &games[0];
    assert_eq!(game.title, "Celeste");
    assert_ne!(game.id, celeste, "the number in the file is not kept");
    assert_eq!(game.status, Status::Playing);
    assert!(game.favorite);
    assert_eq!(game.playtime_minutes, Some(60));
    let collection = after.core.collections().unwrap()[0].id;
    assert_eq!(after.core.game_collections(&game.id).unwrap(), [collection]);
}

#[test]
fn the_import_replaces_the_data_that_was_here() {
    let before = App::start();
    before.add("Games/Celeste/Celeste.exe", "Celeste");
    let file = before.export_to_disk();
    let after = App::start();
    let hades = after.add("Games/Hades/Hades.exe", "Hades");
    after.core.set_status(&hades, Status::Completed).unwrap();
    after.core.create_collection("Roguelikes").unwrap();
    after.images.choose(&after.put("Pictures/hades.png", PNG));
    after.core.choose_cover_image(&hades).unwrap();
    after.core.choose_theme(Theme::Light).unwrap();

    after.import(&file);

    let titles: Vec<_> = after.core.library().unwrap().into_iter().map(|game| game.title).collect();
    assert_eq!(titles, ["Celeste"]);
    assert_eq!(after.core.collections().unwrap(), []);
    assert_eq!(after.core.theme(), Ok(Theme::System));
    assert_eq!(
        fs::read_dir(after.profile.path().join("covers")).map(|files| files.count()).unwrap_or(0),
        0,
        "the Cover image chosen for Hades is gone with it"
    );
}

#[test]
fn steam_games_stay_and_the_steam_web_api_key_is_never_touched() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let before = App::start();
    let file = before.export_to_disk();
    let after = App::start_with(Some(&steam));
    after.core.read_installed_games().unwrap();

    after.import(&file);

    let ids: Vec<_> = after.core.library().unwrap().into_iter().map(|game| game.id).collect();
    assert_eq!(ids, ["steam:620"]);
    assert_eq!(after.core.steam_web_api_key_stored(), Ok(true));
    assert_eq!(after.secrets.changes(), Vec::<String>::new());
}

#[test]
fn the_settings_are_imported_and_the_app_starts_with_windows_as_the_file_says() {
    let steam = SteamFixture::new();
    let before = App::start_with(Some(&steam));
    before.core.choose_steam_account("76561197960287930").unwrap();
    before.core.choose_theme(Theme::Light).unwrap();
    before.core.choose_language(Language::German).unwrap();
    before.core.choose_library_layout(LibraryLayout::List).unwrap();
    before.core.choose_library_sort(LibrarySort::Playtime).unwrap();
    before.core.set_close_to_tray(false).unwrap();
    before.core.set_start_with_windows(true).unwrap();
    let file = before.export_to_disk();
    let after = App::start();

    after.import(&file);

    assert_eq!(after.export()["settings"], before.export()["settings"]);
    assert_eq!(after.core.starts_with_windows(), Ok(true));
}

#[test]
fn steam_sessions_in_the_file_are_accepted() {
    let app = App::start();
    let file = app.write_file(json!({
        "sessions": [
            { "gameId": "steam:620", "startedAt": at(10, 0), "endedAt": at(11, 0), "steamAccount": "76561197960287930" },
            { "gameId": "steam:400", "startedAt": at(12, 0), "endedAt": at(13, 0), "steamAccount": null },
        ],
    }));

    assert!(matches!(app.import(&file), DataImport::Imported { local_games: 0, .. }));
}

#[test]
fn a_cover_match_in_the_file_comes_back_and_is_exported_again() {
    let app = App::start();
    let file = app.write_file(json!({
        "localGames": [local_game_entry("local:7", json!({ "coverMatch": { "appId": 367520, "name": "Hollow Knight" } }))],
    }));

    app.import(&file);

    let game = app.core.library().unwrap().remove(0);
    let matched = app.core.cover_match(&game.id).unwrap().expect("a Cover Match");
    assert_eq!((matched.app_id, matched.name.as_str()), (367520, "Hollow Knight"));
    assert_eq!(app.export()["localGames"][0]["coverMatch"], json!({ "appId": 367520, "name": "Hollow Knight" }));
}

#[test]
fn a_file_of_a_newer_version_is_refused_and_changes_nothing() {
    let app = App::start_with_some_data();
    let before = without_time(app.export());
    let mut newer = minimal_file();
    newer["version"] = json!(2);
    newer["somethingNew"] = json!({ "the app": "does not know this" });
    let file = app.put("newer.json", newer.to_string().as_bytes());

    assert_eq!(app.import(&file), DataImport::NewerVersion { version: 2 });
    assert_eq!(without_time(app.export()), before);
}

#[test]
fn a_damaged_file_or_one_that_is_no_export_file_is_refused_and_changes_nothing() {
    let app = App::start_with_some_data();
    let before = without_time(app.export());
    let whole = minimal_file().to_string();
    let damaged: Vec<(&str, Vec<u8>)> = vec![
        ("not JSON", b"this is no export".to_vec()),
        ("cut off", whole.as_bytes()[..whole.len() / 2].to_vec()),
        ("another format", with(json!({ "format": "some-other-app" })).to_string().into_bytes()),
        ("version 0", with(json!({ "version": 0 })).to_string().into_bytes()),
        ("no version", without_field("version").to_string().into_bytes()),
        ("no settings", without_field("settings").to_string().into_bytes()),
        ("an unknown Status", with(json!({ "statuses": [{ "gameId": "steam:620", "status": "abandoned" }] })).to_string().into_bytes()),
        ("an unknown Game", with(json!({ "favorites": ["local:9"] })).to_string().into_bytes()),
        ("an unknown Source", with(json!({ "hiddenGames": ["epic:1"] })).to_string().into_bytes()),
        (
            "a Session ending before it began",
            with(json!({ "sessions": [{ "gameId": "steam:620", "startedAt": 20, "endedAt": 10 }] })).to_string().into_bytes(),
        ),
        ("progress over 100", with(json!({ "localGames": [local_game_entry("local:1", json!({ "manualProgress": 101 }))] })).to_string().into_bytes()),
        (
            "a Cover image that is no image",
            with(json!({ "localGames": [local_game_entry("local:1", json!({ "coverImage": { "mediaType": "image/png", "data": "bm8gaW1hZ2U=" } }))] }))
                .to_string()
                .into_bytes(),
        ),
        (
            "a Cover image that is no Base64",
            with(json!({ "localGames": [local_game_entry("local:1", json!({ "coverImage": { "mediaType": "image/png", "data": "!!" } }))] }))
                .to_string()
                .into_bytes(),
        ),
        (
            "two Local Games with one number",
            with(json!({ "localGames": [local_game_entry("local:1", json!({})), local_game_entry("local:1", json!({ "executable": r"C:\Other.exe" }))] }))
                .to_string()
                .into_bytes(),
        ),
        (
            "two Local Games with one executable",
            with(json!({ "localGames": [local_game_entry("local:1", json!({})), local_game_entry("local:2", json!({}))] }))
                .to_string()
                .into_bytes(),
        ),
        (
            "two Collections with one name",
            with(json!({ "collections": [{ "name": "Puzzles", "gameIds": [] }, { "name": "puzzles", "gameIds": [] }] }))
                .to_string()
                .into_bytes(),
        ),
    ];

    for (what, bytes) in damaged {
        let file = app.put("damaged.json", &bytes);
        assert_eq!(app.import(&file), DataImport::Damaged, "{what}");
        assert_eq!(without_time(app.export()), before, "{what} changed something");
    }
}

#[test]
fn fields_the_app_does_not_know_are_ignored() {
    let app = App::start();
    let file = app.write_file(json!({
        "fromTheFuture": true,
        "localGames": [local_game_entry("local:1", json!({ "rating": 5 }))],
    }));

    assert!(matches!(app.import(&file), DataImport::Imported { local_games: 1, .. }));
}

#[test]
fn cancelling_imports_nothing() {
    let app = App::start_with_some_data();
    let before = without_time(app.export());

    assert_eq!(app.core.import_data(), Ok(DataImport::Cancelled));
    assert_eq!(without_time(app.export()), before);
}

#[test]
fn a_file_dialog_that_cannot_be_shown_is_an_error() {
    let app = App::start();
    app.user.sees_no_dialog();

    assert!(app.core.import_data().is_err());
}

#[test]
fn a_file_that_cannot_be_read_is_an_error() {
    let app = App::start();
    app.user.opens(&app.disk.path().join("gone.json"));

    assert!(app.core.import_data().is_err());
}

/// The smallest Export File there is: default settings, no data.
fn minimal_file() -> Value {
    json!({
        "format": "nicelibrary-export",
        "version": 1,
        "appVersion": "0.1.0",
        "exportedAt": 1_800_000_000,
        "settings": {
            "chosenSteamAccount": null,
            "theme": "system",
            "language": "system",
            "libraryLayout": "coverGrid",
            "librarySort": "recentlyPlayed",
            "closeToTray": true,
            "startWithWindows": false,
        },
        "localGames": [],
        "sessions": [],
        "statuses": [],
        "favorites": [],
        "hiddenGames": [],
        "collections": [],
    })
}

/// [`minimal_file`] with the top-level fields of `changes` in place of its own.
fn with(changes: Value) -> Value {
    let mut file = minimal_file();
    for (field, value) in changes.as_object().unwrap() {
        file[field] = value.clone();
    }
    file
}

fn without_field(field: &str) -> Value {
    let mut file = minimal_file();
    file.as_object_mut().unwrap().remove(field);
    file
}

/// An entry of `localGames` numbered `id`, with the fields of `changes` in place of its own.
fn local_game_entry(id: &str, changes: Value) -> Value {
    let mut entry = json!({
        "id": id,
        "title": "Celeste",
        "executable": r"C:\Games\Celeste\Celeste.exe",
        "launchArguments": "",
        "workingDirectory": null,
        "manualProgress": null,
        "playtimeAdjustmentMinutes": null,
        "coverImage": null,
    });
    for (field, value) in changes.as_object().unwrap() {
        entry[field] = value.clone();
    }
    entry
}

/// An export without the time it was made, which differs between two exports.
fn without_time(mut export: Value) -> Value {
    export.as_object_mut().unwrap().remove("exportedAt");
    export
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
    user: UserChoosesFiles,
    images: UserChoosesImage,
    secrets: KeyStored,
    profile: TempDir,
    disk: TempDir,
    /// The executable of each Local Game added, by its id.
    executables: RefCell<HashMap<String, PathBuf>>,
}

impl App {
    fn start() -> Self {
        Self::start_with(None)
    }

    /// An app with a Local Game that has a bit of everything, and settings of its own.
    fn start_with_some_data() -> Self {
        let app = Self::start();
        let celeste = app.add("Games/Celeste/Celeste.exe", "Celeste");
        app.core.set_status(&celeste, Status::Playing).unwrap();
        app.core.mark_favorite(&celeste, true).unwrap();
        app.core.set_manual_progress(&celeste, Some(30)).unwrap();
        app.core.create_collection("Puzzles").unwrap();
        app.core.choose_theme(Theme::Light).unwrap();
        app.images.choose(&app.put("Pictures/celeste.png", PNG));
        app.core.choose_cover_image(&celeste).unwrap();
        app.play(&celeste, at(10, 0), at(10, 30));
        app
    }

    fn start_with(steam: Option<&SteamFixture>) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let processes =
            ScriptedProcesses { looks: Arc::default(), clock: ScriptedClock(Arc::new(Mutex::new(UNIX_EPOCH))) };
        let user = UserChoosesFiles::default();
        let images = UserChoosesImage::default();
        let secrets = KeyStored::default();
        let steam = steam.map(|steam| steam.0.path().to_path_buf());
        let core = Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam)),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(secrets.clone()),
                web_api: Box::new(NoWebApi),
                clock: Box::new(processes.clock.clone()),
                executable_picker: Box::new(NoExecutableChosen),
                processes: Box::new(processes.clone()),
                autostart: Box::new(Autostart::default()),
                process_starter: Box::new(NothingStarted),
                icon_extractor: Box::new(NoIcons),
                image_picker: Box::new(images.clone()),
                export_file_picker: Box::new(user.clone()),
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
            secrets,
            profile,
            disk: tempfile::tempdir().unwrap(),
            executables: RefCell::default(),
        }
    }

    /// Local Games, Sessions, a Cover image, Statuses, Favorites, a Hidden
    /// Game, Collections and settings, as a person would have them.
    fn fill_with_everything(&self) {
        self.core.read_installed_games().unwrap();
        let celeste = self.add("Games/Celeste/Celeste.exe", "Celeste");
        let hades = self.add("Games/Hades/Hades.exe", "Hades");
        let options = LaunchOptions { arguments: "-windowed".into(), working_directory: Some(r"D:\Saves".into()) };
        self.core.set_launch_options(&celeste, &options).unwrap();
        self.core.set_manual_progress(&celeste, Some(40)).unwrap();
        self.core.set_playtime_adjustment(&hades, Some(-15)).unwrap();
        self.images.choose(&self.put("Pictures/celeste.png", PNG));
        self.core.choose_cover_image(&celeste).unwrap();
        self.play(&celeste, at(10, 0), at(10, 45));
        self.play(&hades, at(12, 0), at(13, 0));
        self.core.set_status("steam:620", Status::Completed).unwrap();
        self.core.set_status(&hades, Status::Dropped).unwrap();
        self.core.mark_favorite("steam:620", true).unwrap();
        self.core.mark_favorite(&celeste, true).unwrap();
        self.core.hide_game("steam:400").unwrap();
        let puzzles = collection_id(self.core.create_collection("Puzzles").unwrap());
        self.core.create_collection("Someday").unwrap();
        self.core.add_game_to_collection(puzzles, "steam:620").unwrap();
        self.core.add_game_to_collection(puzzles, &celeste).unwrap();
        self.core.choose_theme(Theme::Light).unwrap();
        self.core.choose_library_sort(LibrarySort::Name).unwrap();
    }

    /// Exports into a file of the app's profile and answers what it holds.
    fn export(&self) -> Value {
        let file = self.profile.path().join("export.json");
        self.user.saves_to(&file);
        assert!(matches!(self.core.export_data(), Ok(DataExport::Exported { .. })));
        read(&file)
    }

    /// Exports onto the disk, outside the app's profile, and answers the file.
    fn export_to_disk(&self) -> PathBuf {
        let file = self.disk.path().join("NiceLibrary.json");
        self.user.saves_to(&file);
        assert!(matches!(self.core.export_data(), Ok(DataExport::Exported { .. })));
        file
    }

    /// Writes [`minimal_file`] with the fields of `changes` to the disk and answers the file.
    fn write_file(&self, changes: Value) -> PathBuf {
        self.put("import.json", with(changes).to_string().as_bytes())
    }

    /// Imports `file` as the user would, choosing it in the dialog.
    fn import(&self, file: &Path) -> DataImport {
        self.user.opens(file);
        self.core.import_data().unwrap()
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

/// A substitute for the operating system's "Save as" and "Open" dialogs:
/// answers the file the test says the user chose, that they cancelled, or
/// that no dialog could be shown.
#[derive(Clone)]
struct UserChoosesFiles {
    saves: Arc<Mutex<Result<Option<PathBuf>, String>>>,
    opens: Arc<Mutex<Result<Option<PathBuf>, String>>>,
}

impl Default for UserChoosesFiles {
    /// Until the test says otherwise, the user cancels.
    fn default() -> Self {
        Self { saves: Arc::new(Mutex::new(Ok(None))), opens: Arc::new(Mutex::new(Ok(None))) }
    }
}

impl UserChoosesFiles {
    fn saves_to(&self, file: &Path) {
        *self.saves.lock().unwrap() = Ok(Some(file.to_path_buf()));
    }

    fn opens(&self, file: &Path) {
        *self.opens.lock().unwrap() = Ok(Some(file.to_path_buf()));
    }

    fn sees_no_dialog(&self) {
        *self.opens.lock().unwrap() = Err("no dialog today".into());
    }
}

impl ExportFilePicker for UserChoosesFiles {
    fn choose_export_file(&self, _suggested_name: &str) -> Result<Option<PathBuf>, String> {
        self.saves.lock().unwrap().clone()
    }

    fn choose_import_file(&self) -> Result<Option<PathBuf>, String> {
        self.opens.lock().unwrap().clone()
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

/// A secret store that holds a Steam Web API key and remembers every change
/// anyone makes to it.
#[derive(Clone, Default)]
struct KeyStored(Arc<Mutex<Vec<String>>>);

impl KeyStored {
    fn changes(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl nicelibrary_core::SecretStore for KeyStored {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(Some(KEY.into()))
    }

    fn write(&self, name: &str, _secret: &str) -> Result<(), String> {
        self.0.lock().unwrap().push(format!("write {name}"));
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.0.lock().unwrap().push(format!("delete {name}"));
        Ok(())
    }
}

/// A network that reaches no Web API.
struct NoWebApi;

impl nicelibrary_core::WebApi for NoWebApi {
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
