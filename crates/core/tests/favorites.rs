//! Favorites: Games the user marks to find them quickly. Any Game of the
//! Library can be marked and unmarked; the Library answers which Games are
//! Favorites, narrows itself to them and counts them. The mark survives
//! Syncs and restarts.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nicelibrary_core::{
    AddLocalGame, Core, CoverCache, Downloader, Game, LibraryFilter, OpenError, Opener, Ports, SecretStore, Source,
    SteamLocator, Store, SyncOutcome, WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// The Steam Account logged in on the test computer.
const PLAYER: &str = "76561197960287930";

/// A fake of the Steam Web API: the Steam Account owns the games the test
/// says, as `(app id, name, Playtime in minutes)`, which may change between
/// Syncs.
#[derive(Clone, Default)]
struct SteamOwning(Arc<Mutex<Vec<(u32, &'static str, u32)>>>);

impl SteamOwning {
    fn games(&self, games: &[(u32, &'static str, u32)]) {
        *self.0.lock().unwrap() = games.to_vec();
    }
}

impl WebApi for SteamOwning {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        if !url.starts_with("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?") {
            return Ok(WebAnswer { status: 404, body: b"<html><body>Not Found</body></html>".to_vec() });
        }
        let games: Vec<_> = self
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|(app_id, name, minutes)| {
                serde_json::json!({
                    "appid": app_id,
                    "name": name,
                    "playtime_forever": minutes,
                    "rtime_last_played": 0
                })
            })
            .collect();
        let body = serde_json::json!({ "response": { "game_count": games.len(), "games": games } });
        Ok(WebAnswer { status: 200, body: body.to_string().into_bytes() })
    }
}

/// A secret store that holds the Steam Web API key.
struct HoldingTheKey;

impl SecretStore for HoldingTheKey {
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

/// A computer that opens no URL; these tests launch nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// A substitute for the registry lookup: Steam is where the test says.
struct SteamAt(PathBuf);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        Some(self.0.clone())
    }
}

/// A computer without a network connection for Covers.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &nicelibrary_core::Wakeup) {}
}

/// A computer whose user chooses no executable; Local Games are added by path here.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
    }
}

/// The user profile of one test, on a computer where Steam has Portal 2
/// Installed and the Steam Account owns what `steam` says.
struct Profile {
    folder: TempDir,
    steam_folder: TempDir,
    steam: SteamOwning,
}

impl Profile {
    fn new() -> Self {
        let profile =
            Self { folder: tempfile::tempdir().unwrap(), steam_folder: steam_with_portal_2(), steam: SteamOwning::default() };
        profile.steam.games(&[(620, "Portal 2", 1_260), (220, "Half-Life 2", 90)]);
        profile
    }

    /// Starts the app and reads which Steam Games are Installed, as the interface does.
    fn start(&self) -> Core {
        let store = Store::open(&self.folder.path().join("nicelibrary.db")).unwrap();
        let core = Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(self.steam_folder.path().to_path_buf())),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(HoldingTheKey),
                web_api: Box::new(self.steam.clone()),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
            },
            store,
            CoverCache::new(self.folder.path().join("covers")),
        );
        core.read_installed_games().unwrap();
        core
    }

    /// Starts the app and Syncs, so the Library holds Portal 2 and Half-Life 2.
    fn start_synced(&self) -> Core {
        let core = self.start();
        assert_eq!(core.sync(), Ok(SyncOutcome::Synced { at: 0 }));
        core
    }

    /// An executable on disk for a Local Game.
    fn executable(&self, name: &str) -> String {
        let path = self.folder.path().join(name);
        fs::write(&path, b"MZ").unwrap();
        path.display().to_string()
    }
}

/// A Steam installation with PLAYER logged in and one Installed Steam Game, Portal 2.
fn steam_with_portal_2() -> TempDir {
    let steam = tempfile::tempdir().unwrap();
    fs::create_dir_all(steam.path().join("config")).unwrap();
    fs::write(
        steam.path().join("config").join("loginusers.vdf"),
        format!(r#""users" {{ "{PLAYER}" {{ "PersonaName" "Player" "MostRecent" "1" }} }}"#),
    )
    .unwrap();
    let path = steam.path().display().to_string().replace('\\', "\\\\");
    fs::create_dir_all(steam.path().join("steamapps")).unwrap();
    fs::write(
        steam.path().join("steamapps").join("libraryfolders.vdf"),
        format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{path}\"\n\t}}\n}}\n"),
    )
    .unwrap();
    fs::write(
        steam.path().join("steamapps").join("appmanifest_620.acf"),
        "\"AppState\"\n{\n\t\"appid\"\t\t\"620\"\n\t\"name\"\t\t\"Portal 2\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"Portal 2\"\n}\n",
    )
    .unwrap();
    steam
}

/// The ids of the Library's Favorites, in the Library's order.
fn favorites(core: &Core) -> Vec<String> {
    core.library().unwrap().into_iter().filter(|game| game.favorite).map(|game| game.id).collect()
}

fn favorites_only() -> LibraryFilter {
    LibraryFilter { favorites_only: true, ..LibraryFilter::default() }
}

fn ids(games: Vec<Game>) -> Vec<String> {
    games.into_iter().map(|game| game.id).collect()
}

#[test]
fn no_game_is_a_favorite_until_the_user_marks_one() {
    let core = Profile::new().start_synced();

    assert_eq!(core.library().unwrap().len(), 2);
    assert!(favorites(&core).is_empty());
    assert_eq!(core.favorite_count(), Ok(0));
}

#[test]
fn a_marked_game_is_a_favorite_in_the_library_and_on_its_detail_page() {
    let core = Profile::new().start_synced();

    core.mark_favorite("steam:620", true).unwrap();

    assert_eq!(favorites(&core), ["steam:620"]);
    assert!(core.game("steam:620").unwrap().unwrap().favorite);
    assert!(!core.game("steam:220").unwrap().unwrap().favorite);
    assert_eq!(core.favorite_count(), Ok(1));
}

#[test]
fn an_unmarked_game_is_no_favorite_any_more() {
    let core = Profile::new().start_synced();
    core.mark_favorite("steam:620", true).unwrap();
    core.mark_favorite("steam:220", true).unwrap();

    core.mark_favorite("steam:620", false).unwrap();

    assert_eq!(favorites(&core), ["steam:220"]);
    assert_eq!(core.favorite_count(), Ok(1));
}

#[test]
fn marking_a_favorite_again_or_unmarking_a_game_that_is_none_changes_nothing() {
    let core = Profile::new().start_synced();
    core.mark_favorite("steam:620", true).unwrap();

    core.mark_favorite("steam:620", true).unwrap();
    core.mark_favorite("steam:220", false).unwrap();

    assert_eq!(favorites(&core), ["steam:620"]);
    assert_eq!(core.favorite_count(), Ok(1));
}

#[test]
fn a_game_that_is_not_installed_and_a_local_game_can_be_favorites_too() {
    let profile = Profile::new();
    let core = profile.start_synced();
    let AddLocalGame::Added { game: tetris } = core.add_local_game(&profile.executable("tetris.exe"), "Tetris").unwrap()
    else {
        panic!("the Local Game should be added");
    };
    assert_eq!((tetris.source, tetris.favorite), (Source::Local, false));

    core.mark_favorite("steam:220", true).unwrap();
    core.mark_favorite(&tetris.id, true).unwrap();

    assert_eq!(favorites(&core), ["steam:220".to_owned(), tetris.id]);
    assert_eq!(core.favorite_count(), Ok(2));
}

#[test]
fn marking_a_game_that_is_not_in_the_library_fails_and_changes_nothing() {
    let core = Profile::new().start_synced();

    assert!(core.mark_favorite("steam:400", true).is_err());
    assert!(core.mark_favorite("local:7", true).is_err());

    assert!(favorites(&core).is_empty());
    assert_eq!(core.favorite_count(), Ok(0));
}

#[test]
fn the_library_narrows_to_the_favorites_together_with_the_other_filters() {
    let profile = Profile::new();
    profile.steam.games(&[(620, "Portal 2", 1_260), (400, "Portal", 300), (220, "Half-Life 2", 90)]);
    let core = profile.start_synced();
    core.mark_favorite("steam:620", true).unwrap();
    core.mark_favorite("steam:220", true).unwrap();

    assert_eq!(ids(core.filter_library(&favorites_only()).unwrap()), ["steam:220", "steam:620"]);
    assert_eq!(
        ids(core.filter_library(&LibraryFilter { search: "portal".into(), ..favorites_only() }).unwrap()),
        ["steam:620"]
    );
    assert_eq!(
        ids(core.filter_library(&LibraryFilter { installed_only: true, ..favorites_only() }).unwrap()),
        ["steam:620"]
    );
    assert_eq!(core.filter_library(&LibraryFilter::default()).unwrap().len(), 3);
}

#[test]
fn favorites_survive_a_restart() {
    let profile = Profile::new();
    {
        let core = profile.start_synced();
        core.mark_favorite("steam:620", true).unwrap();
        core.mark_favorite("steam:220", true).unwrap();
        core.mark_favorite("steam:220", false).unwrap();
    }

    let core = profile.start();

    assert_eq!(favorites(&core), ["steam:620"]);
    assert_eq!(core.favorite_count(), Ok(1));
}

#[test]
fn favorites_survive_a_sync_that_changes_their_playtime() {
    let profile = Profile::new();
    let core = profile.start_synced();
    core.mark_favorite("steam:220", true).unwrap();
    profile.steam.games(&[(620, "Portal 2", 1_260), (220, "Half-Life 2", 150)]);

    assert_eq!(core.sync(), Ok(SyncOutcome::Synced { at: 0 }));

    let half_life = core.game("steam:220").unwrap().unwrap();
    assert_eq!((half_life.favorite, half_life.playtime_minutes), (true, Some(150)));
}

#[test]
fn a_favorite_that_leaves_the_library_is_not_counted_and_is_a_favorite_again_when_it_returns() {
    let profile = Profile::new();
    let core = profile.start_synced();
    core.mark_favorite("steam:220", true).unwrap();

    profile.steam.games(&[(620, "Portal 2", 1_260)]);
    assert_eq!(core.sync(), Ok(SyncOutcome::Synced { at: 0 }));
    assert!(core.game("steam:220").unwrap().is_none());
    assert_eq!(core.favorite_count(), Ok(0));
    assert!(core.filter_library(&favorites_only()).unwrap().is_empty());

    profile.steam.games(&[(620, "Portal 2", 1_260), (220, "Half-Life 2", 90)]);
    assert_eq!(core.sync(), Ok(SyncOutcome::Synced { at: 0 }));
    assert_eq!(favorites(&core), ["steam:220"]);
}

#[test]
fn the_favorite_mark_reaches_the_interface_in_the_shape_the_interface_expects() {
    let core = Profile::new().start_synced();
    core.mark_favorite("steam:620", true).unwrap();

    let portal = serde_json::to_value(core.game("steam:620").unwrap().unwrap()).unwrap();
    assert_eq!(portal["favorite"], serde_json::json!(true));

    let filter: LibraryFilter = serde_json::from_value(serde_json::json!({ "favoritesOnly": true })).unwrap();
    assert_eq!(filter, favorites_only());
}
