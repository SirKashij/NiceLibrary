//! Cover Match: a Local Game borrows the Cover of a Steam title. When a
//! Local Game is added, the app proposes the title of the Steam App List
//! that best matches its name; the user confirms it, picks another title or
//! declines, and can change or remove the match later. Without a stored
//! Steam App List or without a match, the Game keeps its executable's icon.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nicelibrary_core::{
    AddLocalGame, Core, CoverCache, Downloader, Icon, IconExtractor, OpenError, Opener, Ports, SecretStore,
    SteamAppListFetch, SteamLocator, SteamTitle, Store, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";

/// The titles the fake Steam App List holds.
const STEAM_APPS: &[(u32, &str)] = &[
    (220, "Half-Life 2"),
    (292030, "The Witcher® 3: Wild Hunt"),
    (367520, "Hollow Knight"),
    (504230, "Celeste"),
    (1145360, "Hades"),
    (1145350, "Hades II"),
    (2000000, "Celeste Classic Fan Remake"),
    (2000001, "Hadesborn"),
];

/// A fake `IStoreService/GetAppList` that answers the whole list on one page.
struct SteamWebApi;

impl WebApi for SteamWebApi {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        if !url.starts_with("https://api.steampowered.com/IStoreService/GetAppList/v1/?") || !url.contains(KEY) {
            return Ok(WebAnswer { status: 403, body: b"Forbidden".to_vec() });
        }
        let apps: Vec<_> =
            STEAM_APPS.iter().map(|(id, name)| serde_json::json!({ "appid": id, "name": name })).collect();
        let body = serde_json::json!({ "response": { "apps": apps } }).to_string();
        Ok(WebAnswer { status: 200, body: body.into_bytes() })
    }
}

/// A secret store holding the key, or nothing.
#[derive(Clone, Default)]
struct Secrets(Arc<Mutex<HashMap<String, String>>>);

impl SecretStore for Secrets {
    fn read(&self, name: &str) -> Result<Option<String>, String> {
        Ok(self.0.lock().unwrap().get(name).cloned())
    }

    fn write(&self, name: &str, secret: &str) -> Result<(), String> {
        self.0.lock().unwrap().insert(name.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.0.lock().unwrap().remove(name);
        Ok(())
    }
}

/// The upright library image Steam has for `app_id` in these tests.
fn steam_cover(app_id: u32) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
    bytes.extend(format!("library image of {app_id}").into_bytes());
    bytes
}

/// Steam's artwork servers: every app of the list has a library image,
/// unless the network is gone.
#[derive(Clone, Default)]
struct Artwork {
    offline: Arc<Mutex<bool>>,
}

impl Downloader for Artwork {
    fn download(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        if *self.offline.lock().unwrap() {
            return Err("no network connection".into());
        }
        let app_id = url
            .strip_prefix("https://shared.steamstatic.com/store_item_assets/steam/apps/")
            .and_then(|rest| rest.strip_suffix("/library_600x900.jpg"))
            .and_then(|id| id.parse::<u32>().ok());
        Ok(app_id.filter(|id| STEAM_APPS.iter().any(|(app, _)| app == id)).map(steam_cover))
    }
}

/// Every executable shows one red square as its icon.
struct RedIcons;

impl IconExtractor for RedIcons {
    fn largest_icon(&self, executable: &Path) -> Result<Option<Icon>, String> {
        if !executable.is_file() {
            return Err(format!("cannot read {}", executable.display()));
        }
        Ok(Some(Icon { width: 32, height: 32, rgba: [210, 30, 40, 255].repeat(32 * 32) }))
    }
}

struct App {
    core: Core,
    secrets: Secrets,
    artwork: Artwork,
    image: Arc<Mutex<Option<PathBuf>>>,
    profile: TempDir,
    disk: TempDir,
}

impl App {
    /// The app with the key stored and the Steam App List fetched.
    fn with_steam_app_list() -> Self {
        let app = Self::without_key();
        app.secrets.write("steam-web-api-key", KEY).unwrap();
        assert!(matches!(app.core.fetch_steam_app_list().unwrap(), SteamAppListFetch::Fetched { .. }));
        app
    }

    /// The app without a key, so no Steam App List was ever fetched.
    fn without_key() -> Self {
        let profile = tempfile::tempdir().unwrap();
        let (secrets, artwork, user) = (Secrets::default(), Artwork::default(), UserChooses::default());
        let core = Self::core_in(profile.path(), &secrets, &artwork, &user);
        Self { core, secrets, artwork, image: user.0, profile, disk: tempfile::tempdir().unwrap() }
    }

    fn core_in(profile: &Path, secrets: &Secrets, artwork: &Artwork, user: &UserChooses) -> Core {
        Core::new(
            Ports {
                steam_locator: Box::new(NoSteam),
                downloader: Box::new(artwork.clone()),
                opener: Box::new(NoUrls),
                secrets: Box::new(secrets.clone()),
                web_api: Box::new(SteamWebApi),
                clock: Box::new(StoppedClock),
                autostart: Box::new(NoAutostart),
                executable_picker: Box::new(NoExecutableChosen),
                icon_extractor: Box::new(RedIcons),
                image_picker: Box::new(user.clone()),
                processes: Box::new(NothingRuns),
                process_starter: Box::new(NothingStarted),
                data_file_picker: Box::new(NoExportFile),
            },
            Store::open(&profile.join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.join("covers")).with_chosen_images_in(profile.join("chosen")),
        )
    }

    fn restart(self) -> Self {
        drop(self.core);
        let core = Self::core_in(self.profile.path(), &self.secrets, &self.artwork, &UserChooses(Arc::clone(&self.image)));
        Self { core, ..self }
    }

    /// Adds an executable named `file` as the Local Game `title` and answers its id.
    fn add_game(&self, file: &str, title: &str) -> String {
        let executable = self.disk.path().join(file);
        std::fs::write(&executable, b"MZ").unwrap();
        match self.core.add_local_game(executable.to_str().unwrap(), title).unwrap() {
            AddLocalGame::Added { game } => game.id,
            other => panic!("expected the game to be added, got {other:?}"),
        }
    }

    fn cover_bytes(&self, game_id: &str) -> Vec<u8> {
        self.core.cover(game_id).unwrap().expect("a Cover").bytes
    }

    /// The Cover the Game shows without a Cover Match: its executable's icon.
    fn is_icon_cover(&self, game_id: &str) -> bool {
        self.core.cover(game_id).unwrap().is_some_and(|cover| cover.media_type == "image/png")
    }
}

fn title(app_id: u32, name: &str) -> SteamTitle {
    SteamTitle { app_id, name: name.to_owned() }
}

#[test]
fn the_title_of_the_same_name_is_proposed() {
    let app = App::with_steam_app_list();

    assert_eq!(app.core.propose_cover_match("Hollow Knight").unwrap(), Some(title(367520, "Hollow Knight")));
}

#[test]
fn the_proposal_ignores_case_and_punctuation() {
    let app = App::with_steam_app_list();

    assert_eq!(app.core.propose_cover_match("half life 2").unwrap(), Some(title(220, "Half-Life 2")));
}

#[test]
fn the_same_name_is_proposed_before_a_longer_one() {
    let app = App::with_steam_app_list();

    assert_eq!(app.core.propose_cover_match("Celeste").unwrap(), Some(title(504230, "Celeste")));
    assert_eq!(app.core.propose_cover_match("Hades").unwrap(), Some(title(1145360, "Hades")));
}

#[test]
fn without_a_title_of_the_same_name_one_that_begins_with_the_name_is_proposed() {
    let app = App::with_steam_app_list();

    assert_eq!(
        app.core.propose_cover_match("The Witcher 3").unwrap(),
        Some(title(292030, "The Witcher® 3: Wild Hunt"))
    );
}

#[test]
fn a_title_that_only_contains_the_name_is_not_proposed() {
    let app = App::with_steam_app_list();

    assert_eq!(app.core.propose_cover_match("Knight").unwrap(), None);
    assert_eq!(app.core.propose_cover_match("Hadesb").unwrap(), None, "half a word is no match");
    assert_eq!(app.core.propose_cover_match("Totally Unknown Game").unwrap(), None);
    assert_eq!(app.core.propose_cover_match("   ").unwrap(), None);
}

#[test]
fn without_a_key_nothing_is_proposed() {
    let app = App::without_key();

    assert_eq!(app.core.propose_cover_match("Hollow Knight").unwrap(), None);
}

#[test]
fn another_title_can_be_searched_for_in_the_steam_app_list() {
    let app = App::with_steam_app_list();

    let found = app.core.find_steam_titles("hades").unwrap();

    assert_eq!(found[0], title(1145360, "Hades"));
    assert!(found.contains(&title(1145350, "Hades II")));
}

#[test]
fn a_local_game_without_a_cover_match_keeps_its_icon_cover() {
    let app = App::with_steam_app_list();
    let game = app.add_game("HollowKnight.exe", "Hollow Knight");

    assert_eq!(app.core.cover_match(&game).unwrap(), None);
    assert!(app.is_icon_cover(&game));
}

#[test]
fn a_confirmed_cover_match_gives_the_local_game_the_steam_cover_of_the_title() {
    let app = App::with_steam_app_list();
    let game = app.add_game("HollowKnight.exe", "Hollow Knight");

    app.core.set_cover_match(&game, Some(367520)).unwrap();

    assert_eq!(app.core.cover_match(&game).unwrap(), Some(title(367520, "Hollow Knight")));
    assert_eq!(app.cover_bytes(&game), steam_cover(367520));
}

#[test]
fn a_cover_match_can_be_changed() {
    let app = App::with_steam_app_list();
    let game = app.add_game("Hades.exe", "Hades");
    app.core.set_cover_match(&game, Some(1145360)).unwrap();

    app.core.set_cover_match(&game, Some(1145350)).unwrap();

    assert_eq!(app.core.cover_match(&game).unwrap(), Some(title(1145350, "Hades II")));
    assert_eq!(app.cover_bytes(&game), steam_cover(1145350));
}

#[test]
fn a_removed_cover_match_gives_the_game_its_icon_cover_back() {
    let app = App::with_steam_app_list();
    let game = app.add_game("HollowKnight.exe", "Hollow Knight");
    app.core.set_cover_match(&game, Some(367520)).unwrap();

    app.core.set_cover_match(&game, None).unwrap();

    assert_eq!(app.core.cover_match(&game).unwrap(), None);
    assert!(app.is_icon_cover(&game));
}

#[test]
fn a_cover_match_survives_a_restart() {
    let app = App::with_steam_app_list();
    let game = app.add_game("HollowKnight.exe", "Hollow Knight");
    app.core.set_cover_match(&game, Some(367520)).unwrap();

    let app = app.restart();

    assert_eq!(app.core.cover_match(&game).unwrap(), Some(title(367520, "Hollow Knight")));
    assert_eq!(app.cover_bytes(&game), steam_cover(367520));
}

#[test]
fn each_local_game_has_its_own_cover_match() {
    let app = App::with_steam_app_list();
    let hollow_knight = app.add_game("HollowKnight.exe", "Hollow Knight");
    let celeste = app.add_game("Celeste.exe", "Celeste");

    app.core.set_cover_match(&celeste, Some(504230)).unwrap();

    assert_eq!(app.core.cover_match(&hollow_knight).unwrap(), None);
    assert_eq!(app.cover_bytes(&celeste), steam_cover(504230));
}

#[test]
fn a_title_that_is_not_in_the_steam_app_list_cannot_be_matched() {
    let app = App::with_steam_app_list();
    let game = app.add_game("HollowKnight.exe", "Hollow Knight");

    assert!(app.core.set_cover_match(&game, Some(999_999)).is_err());

    assert_eq!(app.core.cover_match(&game).unwrap(), None);
}

#[test]
fn only_a_local_game_in_the_library_can_have_a_cover_match() {
    let app = App::with_steam_app_list();

    assert!(app.core.set_cover_match("steam:620", Some(367520)).is_err());
    assert!(app.core.set_cover_match("local:999", Some(367520)).is_err());
    assert_eq!(app.core.cover_match("steam:620").unwrap(), None);
    assert_eq!(app.core.cover_match("local:999").unwrap(), None);
}

#[test]
fn an_image_the_user_chose_comes_before_the_cover_match() {
    let app = App::with_steam_app_list();
    let game = app.add_game("HollowKnight.exe", "Hollow Knight");
    app.core.set_cover_match(&game, Some(367520)).unwrap();
    let image = app.disk.path().join("mine.jpg");
    let bytes = b"\xFF\xD8\xFF\xE0 my own image".to_vec();
    std::fs::write(&image, &bytes).unwrap();
    *app.image.lock().unwrap() = Some(image);

    app.core.choose_cover_image(&game).unwrap();
    assert_eq!(app.cover_bytes(&game), bytes);

    app.core.remove_cover_image(&game).unwrap();
    assert_eq!(app.cover_bytes(&game), steam_cover(367520), "back to the match, not the icon");
}

#[test]
fn without_a_connection_to_steam_a_matched_game_shows_its_icon_cover() {
    let app = App::with_steam_app_list();
    let game = app.add_game("HollowKnight.exe", "Hollow Knight");
    app.core.set_cover_match(&game, Some(367520)).unwrap();
    *app.artwork.offline.lock().unwrap() = true;

    assert!(app.is_icon_cover(&game));
}

#[test]
fn a_cover_match_stays_without_a_key() {
    let app = App::with_steam_app_list();
    let game = app.add_game("HollowKnight.exe", "Hollow Knight");
    app.core.set_cover_match(&game, Some(367520)).unwrap();

    app.core.remove_steam_web_api_key().unwrap();

    assert_eq!(app.cover_bytes(&game), steam_cover(367520));
}

#[test]
fn removing_a_local_game_forgets_its_cover_match() {
    let app = App::with_steam_app_list();
    let game = app.add_game("HollowKnight.exe", "Hollow Knight");
    app.core.set_cover_match(&game, Some(367520)).unwrap();

    app.core.remove_local_game(&game).unwrap();

    assert_eq!(app.core.cover_match(&game).unwrap(), None);
    // The same executable added again starts without one.
    let again = app.add_game("HollowKnight.exe", "Hollow Knight");
    assert_eq!(app.core.cover_match(&again).unwrap(), None);
    assert!(app.is_icon_cover(&again));
}

/// The image the user chooses in the file dialog; none until the test says.
#[derive(Clone, Default)]
struct UserChooses(Arc<Mutex<Option<PathBuf>>>);

impl nicelibrary_core::ImagePicker for UserChooses {
    fn choose_image(&self) -> Result<Option<PathBuf>, String> {
        Ok(self.0.lock().unwrap().clone())
    }
}

struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

struct NoSteam;

impl SteamLocator for NoSteam {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
    }
}

struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &nicelibrary_core::Wakeup) {}
}

struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

struct NoAutostart;

impl nicelibrary_core::Autostart for NoAutostart {
    fn is_on(&self) -> Result<bool, String> {
        Ok(false)
    }

    fn turn_on(&self, _arguments: &[&str]) -> Result<(), String> {
        Err("this test does not start the app with Windows".into())
    }

    fn turn_off(&self) -> Result<(), String> {
        Ok(())
    }
}

struct NothingStarted;

impl nicelibrary_core::ProcessStarter for NothingStarted {
    fn start(&self, executable: &Path, _arguments: &[String], _working_directory: &Path) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}

struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<PathBuf>, String> {
        Ok(Vec::new())
    }
}

/// No export or import is made here: the user cancels every file dialog.
struct NoExportFile;

impl nicelibrary_core::DataFilePicker for NoExportFile {
    fn choose_export_file(&self, _suggested_name: &str) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }

    fn choose_import_file(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}
