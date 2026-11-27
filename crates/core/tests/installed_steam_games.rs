//! Without a key (Limited Mode), the Library lists the Steam Games that are
//! Installed on this computer, read from Steam's library folders and app
//! manifests and kept in the local store.

use std::fs;
use std::path::{Path, PathBuf};

use nicelibrary_core::{Core, CoverCache, Downloader, Game, OpenError, Opener, Ports, Source, SteamLocator, Store};
use tempfile::TempDir;

/// A computer whose secret store holds nothing and whose network reaches no
/// Web API; these tests need neither.
struct NoSecretsNoWebApi;

impl nicelibrary_core::SecretStore for NoSecretsNoWebApi {
    fn read(&self, _name: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn write(&self, _name: &str, _secret: &str) -> Result<(), String> {
        Err("this test stores no secret".into())
    }

    fn delete(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }
}

impl nicelibrary_core::WebApi for NoSecretsNoWebApi {
    fn get(&self, _url: &str) -> Result<nicelibrary_core::WebAnswer, String> {
        Err("this test asks no Web API".into())
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
struct SteamAt(Option<PathBuf>);

impl SteamLocator for SteamAt {
    fn steam_folder(&self) -> Option<PathBuf> {
        self.0.clone()
    }
}

/// A computer without a network connection.
struct Offline;

impl Downloader for Offline {
    fn download(&self, _url: &str) -> Result<Option<Vec<u8>>, String> {
        Err("no network connection".into())
    }
}

const PLAYER_ONE: &str = "76561197960287930";
const PLAYER_TWO: &str = "76561197960287931";

/// A Steam installation on disk, with library folders that may lie elsewhere
/// (on "other drives").
struct SteamFixture {
    steam: TempDir,
    other_drives: Vec<TempDir>,
}

impl SteamFixture {
    /// Steam with one account that has logged in.
    fn new() -> Self {
        let fixture = Self { steam: tempfile::tempdir().unwrap(), other_drives: Vec::new() };
        fixture.logged_in(PLAYER_ONE);
        fixture
    }

    fn path(&self) -> &Path {
        self.steam.path()
    }

    /// Makes `steam_id` the most recently used Steam Account.
    fn logged_in(&self, steam_id: &str) {
        fs::create_dir_all(self.path().join("config")).unwrap();
        fs::write(
            self.path().join("config").join("loginusers.vdf"),
            format!(r#""users" {{ "{steam_id}" {{ "PersonaName" "Player" "MostRecent" "1" }} }}"#),
        )
        .unwrap();
    }

    /// Adds a library folder outside the Steam folder and lists it in
    /// `libraryfolders.vdf`, next to the Steam folder itself.
    fn add_library_folder(&mut self) -> PathBuf {
        let drive = tempfile::tempdir().unwrap();
        let folder = drive.path().join("SteamLibrary");
        fs::create_dir_all(folder.join("steamapps")).unwrap();
        self.other_drives.push(drive);
        self.write_library_folders();
        folder
    }

    fn write_library_folders(&self) {
        let mut text = String::from("\"libraryfolders\"\n{\n");
        let folders = std::iter::once(self.path().to_path_buf())
            .chain(self.other_drives.iter().map(|d| d.path().join("SteamLibrary")));
        for (index, folder) in folders.enumerate() {
            let path = folder.display().to_string().replace('\\', "\\\\");
            text += &format!("\t\"{index}\"\n\t{{\n\t\t\"path\"\t\t\"{path}\"\n\t\t\"label\"\t\t\"\"\n\t}}\n");
        }
        text += "}\n";
        fs::create_dir_all(self.path().join("steamapps")).unwrap();
        fs::write(self.path().join("steamapps").join("libraryfolders.vdf"), text).unwrap();
    }

    /// Writes an app manifest as Steam does for a fully installed app.
    fn install(&self, library_folder: &Path, app_id: u32, name: &str) {
        self.manifest(library_folder, app_id, name, 4);
    }

    fn manifest(&self, library_folder: &Path, app_id: u32, name: &str, state_flags: u32) {
        let steamapps = library_folder.join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"universe\"\t\t\"1\"\n\t\"name\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"{state_flags}\"\n\t\"installdir\"\t\t\"{name}\"\n}}\n"
            ),
        )
        .unwrap();
    }

    fn uninstall(&self, library_folder: &Path, app_id: u32) {
        fs::remove_file(library_folder.join("steamapps").join(format!("appmanifest_{app_id}.acf"))).unwrap();
    }
}

/// The user profile of one test: where the database lives between starts.
struct Profile(TempDir);

impl Profile {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    fn database(&self) -> PathBuf {
        self.0.path().join("nicelibrary.db")
    }

    /// Starts the app: a core over this profile's database.
    fn start(&self, steam: Option<&Path>) -> Core {
        Core::new(
            Ports { steam_locator: Box::new(SteamAt(steam.map(Path::to_path_buf))), downloader: Box::new(Offline), opener: Box::new(NoUrls), secrets: Box::new(NoSecretsNoWebApi), web_api: Box::new(NoSecretsNoWebApi), clock: Box::new(StoppedClock), executable_picker: Box::new(NoExecutableChosen) },
            Store::open(&self.database()).expect("database opens"),
            CoverCache::new(self.0.path().join("covers")),
        )
    }
}

fn installed(app_id: u32, title: &str) -> Game {
    // Without a Sync, Steam's Playtime and Last Played are not known.
    Game {
        id: format!("steam:{app_id}"),
        title: title.into(),
        source: Source::Steam,
        installed: true,
        playtime_minutes: None,
        last_played: None,
        progress: None,
    }
}

fn titles(games: &[Game]) -> Vec<&str> {
    games.iter().map(|g| g.title.as_str()).collect()
}

#[test]
fn lists_the_installed_steam_games_by_title() {
    let steam = SteamFixture::new();
    steam.install(steam.path(), 620, "Portal 2");
    steam.install(steam.path(), 400, "Portal");

    let core = Profile::new().start(Some(steam.path()));

    assert_eq!(core.read_installed_games().unwrap(), vec![installed(400, "Portal"), installed(620, "Portal 2")]);
    assert_eq!(core.library().unwrap(), vec![installed(400, "Portal"), installed(620, "Portal 2")]);
}

#[test]
fn reads_every_library_folder_including_ones_on_other_drives() {
    let mut steam = SteamFixture::new();
    let drive_d = steam.add_library_folder();
    let drive_e = steam.add_library_folder();
    steam.install(steam.path(), 620, "Portal 2");
    steam.install(&drive_d, 70, "Half-Life");
    steam.install(&drive_e, 220, "Half-Life 2");

    let core = Profile::new().start(Some(steam.path()));

    assert_eq!(titles(&core.read_installed_games().unwrap()), ["Half-Life", "Half-Life 2", "Portal 2"]);
}

#[test]
fn reads_library_folders_in_the_format_of_older_steam_versions() {
    let steam = SteamFixture::new();
    let drive = tempfile::tempdir().unwrap();
    let folder = drive.path().join("SteamLibrary");
    steam.install(&folder, 70, "Half-Life");
    fs::create_dir_all(steam.path().join("steamapps")).unwrap();
    fs::write(
        steam.path().join("steamapps").join("libraryfolders.vdf"),
        format!(
            "\"LibraryFolders\"\n{{\n\t\"TimeNextStatsReport\"\t\t\"1700000000\"\n\t\"ContentStatsID\"\t\t\"-1\"\n\t\"1\"\t\t\"{}\"\n}}\n",
            folder.display().to_string().replace('\\', "\\\\")
        ),
    )
    .unwrap();

    let core = Profile::new().start(Some(steam.path()));

    assert_eq!(core.read_installed_games().unwrap(), vec![installed(70, "Half-Life")]);
}

#[test]
fn a_game_whose_manifest_lies_in_two_library_folders_appears_once() {
    let mut steam = SteamFixture::new();
    let drive = steam.add_library_folder();
    steam.install(steam.path(), 620, "Portal 2");
    steam.install(&drive, 620, "Portal 2");

    let core = Profile::new().start(Some(steam.path()));

    assert_eq!(core.read_installed_games().unwrap(), vec![installed(620, "Portal 2")]);
}

#[test]
fn steams_own_redistributables_and_runtimes_do_not_appear() {
    let steam = SteamFixture::new();
    steam.install(steam.path(), 620, "Portal 2");
    steam.install(steam.path(), 228980, "Steamworks Common Redistributables");
    steam.install(steam.path(), 1628350, "Steam Linux Runtime 3.0 (sniper)");
    steam.install(steam.path(), 4185400, "Steam Linux Runtime 4.0");
    steam.install(steam.path(), 1493710, "Proton Experimental");
    steam.install(steam.path(), 2805730, "Proton 9.0");

    let core = Profile::new().start(Some(steam.path()));

    assert_eq!(core.read_installed_games().unwrap(), vec![installed(620, "Portal 2")]);
}

#[test]
fn a_game_still_downloading_for_the_first_time_does_not_appear_but_one_waiting_for_an_update_does() {
    let steam = SteamFixture::new();
    steam.manifest(steam.path(), 620, "Portal 2", 1026); // update required, downloading; never complete
    steam.manifest(steam.path(), 400, "Portal", 6); // fully installed, update required

    let core = Profile::new().start(Some(steam.path()));

    assert_eq!(core.read_installed_games().unwrap(), vec![installed(400, "Portal")]);
}

#[test]
fn a_damaged_manifest_is_skipped() {
    let steam = SteamFixture::new();
    steam.install(steam.path(), 620, "Portal 2");
    fs::write(steam.path().join("steamapps").join("appmanifest_400.acf"), r#""AppState" { "appid" "400" "#).unwrap();

    let core = Profile::new().start(Some(steam.path()));

    assert_eq!(core.read_installed_games().unwrap(), vec![installed(620, "Portal 2")]);
}

#[test]
fn a_manifest_without_a_name_takes_its_title_from_the_install_folder_and_one_without_either_is_skipped() {
    let steam = SteamFixture::new();
    let steamapps = steam.path().join("steamapps");
    fs::create_dir_all(&steamapps).unwrap();
    fs::write(
        steamapps.join("appmanifest_620.acf"),
        r#""AppState" { "appid" "620" "name" "" "StateFlags" "4" "installdir" "Portal 2" }"#,
    )
    .unwrap();
    fs::write(steamapps.join("appmanifest_400.acf"), r#""AppState" { "appid" "400" "StateFlags" "4" }"#).unwrap();

    let core = Profile::new().start(Some(steam.path()));

    assert_eq!(core.read_installed_games().unwrap(), vec![installed(620, "Portal 2")]);
}

#[test]
fn a_second_start_shows_the_games_before_steam_is_read_again() {
    let steam = SteamFixture::new();
    steam.install(steam.path(), 620, "Portal 2");
    let profile = Profile::new();
    profile.start(Some(steam.path())).read_installed_games().unwrap();

    // Second start, even with Steam gone: the stored Library is there at once.
    let core = profile.start(None);

    assert_eq!(core.library().unwrap(), vec![installed(620, "Portal 2")]);
}

#[test]
fn reading_steam_again_keeps_the_library_when_steam_cannot_be_found() {
    let steam = SteamFixture::new();
    steam.install(steam.path(), 620, "Portal 2");
    let profile = Profile::new();
    profile.start(Some(steam.path())).read_installed_games().unwrap();

    assert_eq!(profile.start(None).read_installed_games().unwrap(), vec![installed(620, "Portal 2")]);
}

#[test]
fn a_damaged_list_of_library_folders_leaves_the_library_as_it_was() {
    let mut steam = SteamFixture::new();
    let drive = steam.add_library_folder();
    steam.install(&drive, 70, "Half-Life");
    let profile = Profile::new();
    profile.start(Some(steam.path())).read_installed_games().unwrap();

    fs::write(steam.path().join("steamapps").join("libraryfolders.vdf"), r#""libraryfolders" { "0" { "path" "#)
        .unwrap();

    assert_eq!(profile.start(Some(steam.path())).read_installed_games().unwrap(), vec![installed(70, "Half-Life")]);
}

#[test]
fn an_uninstalled_game_is_no_longer_installed() {
    let steam = SteamFixture::new();
    steam.install(steam.path(), 620, "Portal 2");
    steam.install(steam.path(), 400, "Portal");
    let core = Profile::new().start(Some(steam.path()));
    core.read_installed_games().unwrap();

    steam.uninstall(steam.path(), 400);

    assert_eq!(
        core.read_installed_games().unwrap(),
        vec![Game { installed: false, ..installed(400, "Portal") }, installed(620, "Portal 2")]
    );
}

#[test]
fn without_steam_the_library_stays_empty() {
    let core = Profile::new().start(None);

    assert!(core.read_installed_games().unwrap().is_empty());
}

#[test]
fn games_belong_to_the_steam_account_they_were_read_for() {
    let steam = SteamFixture::new();
    steam.install(steam.path(), 620, "Portal 2");
    let core = Profile::new().start(Some(steam.path()));
    core.read_installed_games().unwrap();

    // Another account logs in; until it has been read, its Library is empty,
    // and the first account's Games are not shown as its own.
    steam.logged_in(PLAYER_TWO);
    steam.uninstall(steam.path(), 620);
    steam.install(steam.path(), 400, "Portal");
    assert_eq!(core.read_installed_games().unwrap(), vec![installed(400, "Portal")]);

    // Back on the first account, its own rows are still there.
    steam.logged_in(PLAYER_ONE);
    assert_eq!(
        core.library().unwrap(),
        vec![installed(400, "Portal")],
        "the library shows the account last read until it is read again"
    );
    assert_eq!(
        core.read_installed_games().unwrap(),
        vec![installed(400, "Portal"), Game { installed: false, ..installed(620, "Portal 2") }]
    );
}

#[test]
fn a_database_from_a_newer_version_of_the_app_is_refused_and_left_alone() {
    let profile = Profile::new();
    let steam = SteamFixture::new();
    steam.install(steam.path(), 620, "Portal 2");
    profile.start(Some(steam.path())).read_installed_games().unwrap();
    mark_as_written_by_a_newer_version(&profile.database());
    let before = fs::read(profile.database()).unwrap();

    let opened = Store::open(&profile.database());

    let error = opened.err().expect("refused");
    assert!(error.to_string().contains("newer version"), "{error}");
    assert_eq!(fs::read(profile.database()).unwrap(), before);
}

/// Stands in for a database a later version of the app wrote: its schema
/// version is beyond what this version knows.
fn mark_as_written_by_a_newer_version(database: &Path) {
    let connection = rusqlite::Connection::open(database).unwrap();
    connection.pragma_update(None, "user_version", 9999).unwrap();
}

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration, _wakeup: &nicelibrary_core::Wakeup) {}
}

/// A computer whose user chooses no executable; these tests add no Local Game.
struct NoExecutableChosen;

impl nicelibrary_core::ExecutablePicker for NoExecutableChosen {
    fn choose_executable(&self) -> Result<Option<std::path::PathBuf>, String> {
        Ok(None)
    }
}
