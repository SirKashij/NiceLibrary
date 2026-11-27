//! The detail page of a Game: one Game of the Library by its id, with its
//! Source, whether it is Installed, its Playtime and Last Played, exactly as
//! the Library shows it. A Game that is not in the Library has no detail page.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{
    Core, CoverCache, Downloader, Game, OpenError, Opener, Ports, SecretStore, Source, Status, SteamLocator, Store,
    SyncOutcome,
    WebAnswer, WebApi,
};
use tempfile::TempDir;

/// A key Steam accepts in these tests.
const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
/// 2026-11-20 20:15:00 UTC, in seconds since the Unix epoch.
const LAST_WEEK: u64 = 1_795_205_700;

/// A secret store that holds the Steam Web API key.
struct KeyStored;

impl SecretStore for KeyStored {
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

/// Steam's Web API, where the account owns Portal 2 with 21 hours played
/// last week and Half-Life 2, never played.
struct SteamOwnsPortal2AndHalfLife2;

impl WebApi for SteamOwnsPortal2AndHalfLife2 {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        if !url.starts_with("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?") {
            return Ok(WebAnswer { status: 404, body: b"<html><body>Not Found</body></html>".to_vec() });
        }
        let body = serde_json::json!({ "response": { "game_count": 2, "games": [
            { "appid": 620, "name": "Portal 2", "playtime_forever": 1_260, "rtime_last_played": LAST_WEEK },
            { "appid": 220, "name": "Half-Life 2", "playtime_forever": 0, "rtime_last_played": 0 }
        ] } });
        Ok(WebAnswer { status: 200, body: body.to_string().into_bytes() })
    }
}

/// A computer that opens no URL; these tests launch nothing.
struct NoUrls;

impl Opener for NoUrls {
    fn open_url(&self, url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed(format!("this test opens no URL, not even {url}")))
    }
}

/// A substitute for the registry lookup: Steam is in the fixture folder.
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

/// A file dialog the user always cancels; these tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<PathBuf>, String> {
        Ok(None)
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

    fn path(&self) -> &Path {
        self.0.path()
    }

    /// Writes an app manifest as Steam does for a fully installed app.
    fn install(&self, app_id: u32, name: &str) {
        let steamapps = self.path().join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"{name}\"\n}}\n"
            ),
        )
        .unwrap();
    }

    fn uninstall(&self, app_id: u32) {
        fs::remove_file(self.path().join("steamapps").join(format!("appmanifest_{app_id}.acf"))).unwrap();
    }
}

/// One running app over `steam`, its Installed Steam Games read.
struct App {
    core: Core,
    _profile: TempDir,
}

impl App {
    fn start(steam: &SteamFixture) -> Self {
        let profile = tempfile::tempdir().unwrap();
        let core = Core::new(
            Ports {
                steam_locator: Box::new(SteamAt(steam.path().to_path_buf())),
                downloader: Box::new(Offline),
                opener: Box::new(NoUrls),
                secrets: Box::new(KeyStored),
                web_api: Box::new(SteamOwnsPortal2AndHalfLife2),
                clock: Box::new(StoppedClock),
                executable_picker: Box::new(NoExecutableChosen),
                icon_extractor: Box::new(NoIconsNoImages),
                image_picker: Box::new(NoIconsNoImages),
                autostart: Box::new(NoAutostart),
            },
            Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.path().join("covers")),
        );
        core.read_installed_games().unwrap();
        Self { core, _profile: profile }
    }
}

#[test]
fn an_installed_steam_game_shows_its_source_and_that_it_is_installed() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);

    assert_eq!(
        app.core.game("steam:620"),
        Ok(Some(Game {
            id: "steam:620".into(),
            title: "Portal 2".into(),
            source: Source::Steam,
            installed: true,
            playtime_minutes: None,
            last_played: None,
            status: Status::Unplayed,
        }))
    );
}

#[test]
fn after_a_sync_a_game_shows_its_playtime_and_last_played() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);
    assert!(matches!(app.core.sync(), Ok(SyncOutcome::Synced { .. })));

    let portal = app.core.game("steam:620").unwrap().unwrap();
    assert_eq!((portal.playtime_minutes, portal.last_played), (Some(1_260), Some(LAST_WEEK)));
    let half_life = app.core.game("steam:220").unwrap().unwrap();
    assert!(!half_life.installed);
    assert_eq!((half_life.playtime_minutes, half_life.last_played), (Some(0), None));
}

#[test]
fn a_game_shows_what_the_library_shows_for_it() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    steam.install(400, "Portal");
    let app = App::start(&steam);
    app.core.sync().unwrap();

    for game in app.core.library().unwrap() {
        assert_eq!(app.core.game(&game.id), Ok(Some(game)));
    }
}

#[test]
fn a_game_that_was_uninstalled_shows_that_it_is_no_longer_installed() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);
    app.core.sync().unwrap();
    steam.uninstall(620);
    app.core.read_installed_games().unwrap();

    assert!(!app.core.game("steam:620").unwrap().unwrap().installed);
}

#[test]
fn a_game_that_is_not_in_the_library_has_no_detail_page() {
    let steam = SteamFixture::new();
    steam.install(620, "Portal 2");
    let app = App::start(&steam);

    assert_eq!(app.core.game("steam:400"), Ok(None));
    assert_eq!(app.core.game(r"C:\Windows\notepad.exe"), Ok(None));
}

/// A computer whose executables show no icon and whose user chooses no
/// image; these tests need neither.
struct NoIconsNoImages;

impl nicelibrary_core::IconExtractor for NoIconsNoImages {
    fn largest_icon(&self, _executable: &std::path::Path) -> Result<Option<nicelibrary_core::Icon>, String> {
        Ok(None)
    }
}

impl nicelibrary_core::ImagePicker for NoIconsNoImages {
    fn choose_image(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}

/// A computer where the app does not start with Windows; these tests do not turn it on.
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
