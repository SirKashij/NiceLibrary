//! A Steam Game's Cover is Steam's upright library image, else its header
//! image, else there is none and the interface shows a placeholder. Covers
//! are downloaded at runtime, once, and served from a cache on disk
//! afterwards, also without a network connection (ADR-0005).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nicelibrary_core::{Core, CoverCache, Downloader, OpenError, Opener, Ports, SteamLocator, Store};
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

struct NoSteam;

impl SteamLocator for NoSteam {
    fn steam_folder(&self) -> Option<PathBuf> {
        None
    }
}

/// A substitute for Steam's artwork servers: serves the files the test puts
/// there, answers "not found" for every other address, and can go offline.
#[derive(Clone, Default)]
struct FakeSteam {
    state: Arc<Mutex<FakeSteamState>>,
}

#[derive(Default)]
struct FakeSteamState {
    files: HashMap<String, Vec<u8>>,
    offline: bool,
    requests: Vec<String>,
}

impl FakeSteam {
    fn serve(&self, url: String, bytes: Vec<u8>) {
        self.state.lock().unwrap().files.insert(url, bytes);
    }

    fn go_offline(&self) {
        self.state.lock().unwrap().offline = true;
    }

    fn go_online(&self) {
        self.state.lock().unwrap().offline = false;
    }

    fn requests(&self) -> Vec<String> {
        self.state.lock().unwrap().requests.clone()
    }
}

impl Downloader for FakeSteam {
    fn download(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        let mut state = self.state.lock().unwrap();
        state.requests.push(url.to_owned());
        if state.offline {
            return Err("no network connection".into());
        }
        Ok(state.files.get(url).cloned())
    }
}

/// Where Steam keeps a Game's upright library image.
fn library_image(app_id: u32) -> String {
    format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{app_id}/library_600x900.jpg")
}

/// Where Steam keeps a Game's wide header image.
fn header_image(app_id: u32) -> String {
    format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{app_id}/header.jpg")
}

/// A PNG image of the given size in one colour, generated for the test.
fn generated_png(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&rgb.repeat((width * height) as usize)).unwrap();
    writer.finish().unwrap();
    bytes
}

/// The user profile of one test: its database and its cache folder.
struct Profile(TempDir);

impl Profile {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    fn covers(&self) -> &Path {
        self.0.path()
    }

    /// Starts the app over this profile, reaching Steam's artwork through `steam`.
    fn start(&self, steam: &FakeSteam) -> Core {
        Core::new(
            Ports { steam_locator: Box::new(NoSteam), downloader: Box::new(steam.clone()), opener: Box::new(NoUrls), secrets: Box::new(NoSecretsNoWebApi), web_api: Box::new(NoSecretsNoWebApi), clock: Box::new(StoppedClock), executable_picker: Box::new(NoExecutableChosen), autostart: Box::new(NoAutostart), processes: Box::new(NothingRuns), process_starter: Box::new(NothingStarted), icon_extractor: Box::new(NoIconsNoImages), image_picker: Box::new(NoIconsNoImages) },
            Store::open(&self.0.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(self.covers().join("covers")),
        )
    }
}

#[test]
fn a_steam_games_cover_is_its_upright_library_image() {
    let steam = FakeSteam::default();
    let upright = generated_png(6, 9, [200, 40, 40]);
    steam.serve(library_image(620), upright.clone());
    steam.serve(header_image(620), generated_png(46, 21, [40, 40, 200]));
    let profile = Profile::new();

    let cover = profile.start(&steam).cover("steam:620").unwrap().expect("a Cover");

    assert_eq!(cover.bytes, upright);
    assert_eq!(cover.media_type, "image/png");
}

#[test]
fn without_a_library_image_the_cover_is_the_header_image() {
    let steam = FakeSteam::default();
    let header = generated_png(46, 21, [40, 40, 200]);
    steam.serve(header_image(228980), header.clone());
    let profile = Profile::new();

    let cover = profile.start(&steam).cover("steam:228980").unwrap().expect("a Cover");

    assert_eq!(cover.bytes, header);
}

#[test]
fn a_game_steam_has_no_image_for_has_no_cover() {
    let steam = FakeSteam::default();
    let profile = Profile::new();

    assert_eq!(profile.start(&steam).cover("steam:3527290").unwrap(), None);
}

#[test]
fn steam_is_asked_once_per_start_for_a_game_without_an_image() {
    let steam = FakeSteam::default();
    let profile = Profile::new();
    let core = profile.start(&steam);
    core.cover("steam:3527290").unwrap();
    let asked = steam.requests().len();

    assert_eq!(core.cover("steam:3527290").unwrap(), None);
    assert_eq!(steam.requests().len(), asked, "asked Steam again");

    profile.start(&steam).cover("steam:3527290").unwrap();
    assert!(steam.requests().len() > asked, "Steam may have added an image since the last start");
}

#[test]
fn a_game_that_is_not_a_steam_game_has_no_cover_from_steam() {
    let steam = FakeSteam::default();
    let profile = Profile::new();
    let core = profile.start(&steam);

    assert_eq!(core.cover("local:7").unwrap(), None);
    assert_eq!(core.cover("steam:not-a-number").unwrap(), None);
    assert!(steam.requests().is_empty());
}

#[test]
fn a_cover_is_downloaded_once_and_served_from_the_cache_afterwards() {
    let steam = FakeSteam::default();
    steam.serve(library_image(620), generated_png(6, 9, [200, 40, 40]));
    let profile = Profile::new();
    let core = profile.start(&steam);
    let first = core.cover("steam:620").unwrap();

    let second = core.cover("steam:620").unwrap();

    assert_eq!(second, first);
    assert_eq!(steam.requests(), vec![library_image(620)]);
}

#[test]
fn a_cached_cover_is_served_without_a_network_connection_after_a_restart() {
    let steam = FakeSteam::default();
    let upright = generated_png(6, 9, [200, 40, 40]);
    steam.serve(library_image(620), upright.clone());
    let profile = Profile::new();
    profile.start(&steam).cover("steam:620").unwrap();
    steam.go_offline();

    let cover = profile.start(&steam).cover("steam:620").unwrap().expect("the cached Cover");

    assert_eq!(cover.bytes, upright);
    assert_eq!(cover.media_type, "image/png");
}

#[test]
fn without_a_network_connection_an_uncached_cover_fails_and_is_downloaded_once_the_network_is_back() {
    let steam = FakeSteam::default();
    let upright = generated_png(6, 9, [200, 40, 40]);
    steam.serve(library_image(620), upright.clone());
    steam.go_offline();
    let profile = Profile::new();
    let core = profile.start(&steam);

    assert!(core.cover("steam:620").is_err(), "a failed download is not \"no Cover\"");

    steam.go_online();
    assert_eq!(core.cover("steam:620").unwrap().expect("a Cover").bytes, upright);
}

#[test]
fn an_answer_that_is_not_an_image_is_neither_a_cover_nor_kept() {
    let steam = FakeSteam::default();
    // What a hotel network's sign-in page sends instead of the image.
    steam.serve(library_image(620), b"<!doctype html><title>Sign in</title>".to_vec());
    let profile = Profile::new();
    let core = profile.start(&steam);

    assert!(core.cover("steam:620").is_err());

    let upright = generated_png(6, 9, [200, 40, 40]);
    steam.serve(library_image(620), upright.clone());
    assert_eq!(core.cover("steam:620").unwrap().expect("a Cover").bytes, upright);
}

#[test]
fn a_cover_steam_serves_as_jpeg_is_served_as_jpeg() {
    let steam = FakeSteam::default();
    // The start of a JPEG file (JFIF): the core only recognises the format.
    let jpeg = [&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10][..], b"JFIF\0", &[0; 32]].concat();
    steam.serve(library_image(620), jpeg);
    let profile = Profile::new();

    assert_eq!(profile.start(&steam).cover("steam:620").unwrap().expect("a Cover").media_type, "image/jpeg");
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

/// A computer on which no process runs; these tests record no Sessions.
struct NothingRuns;

impl nicelibrary_core::ProcessList for NothingRuns {
    fn running_executables(&self) -> Result<Vec<std::path::PathBuf>, String> {
        Ok(Vec::new())
    }
}

/// A computer that starts no process; these tests launch no Local Game.
struct NothingStarted;

impl nicelibrary_core::ProcessStarter for NothingStarted {
    fn start(&self, executable: &Path, _arguments: &[String], _working_directory: &Path) -> Result<(), String> {
        Err(format!("this test starts no process, not even {}", executable.display()))
    }
}
