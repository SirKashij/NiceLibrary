//! The `cover` URL scheme hands the core's Covers to the interface's `<img>`
//! elements. The interface builds the address with Tauri's `convertFileSrc`,
//! which percent-encodes the Game id (`steam:620` becomes `steam%3A620`).

use std::path::PathBuf;

use nicelibrary_core::{Core, CoverCache, Downloader, OpenError, Opener, Ports, SteamLocator, Store};
use nicelibrary_lib::cover_response;
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

/// Steam's artwork host with an image for app 620 only, or no network at all.
struct FakeSteam {
    online: bool,
}

const IMAGE: &[u8] = b"\x89PNG\r\n\x1a\n generated for the test";

impl Downloader for FakeSteam {
    fn download(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        if !self.online {
            return Err("no network connection".into());
        }
        Ok(url.contains("/apps/620/").then(|| IMAGE.to_vec()))
    }
}

fn core(online: bool) -> (Core, TempDir) {
    let profile = tempfile::tempdir().unwrap();
    let core = Core::new(
        Ports { steam_locator: Box::new(NoSteam), downloader: Box::new(FakeSteam { online }), opener: Box::new(NoUrls), secrets: Box::new(NoSecretsNoWebApi), web_api: Box::new(NoSecretsNoWebApi), clock: Box::new(StoppedClock) },
        Store::open(&profile.path().join("nicelibrary.db")).unwrap(),
        CoverCache::new(profile.path().join("covers")),
    );
    (core, profile)
}

#[test]
fn answers_with_the_cover_image_and_its_format() {
    let (core, _profile) = core(true);

    let response = cover_response(&core, "/steam%3A620");

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "image/png");
    assert_eq!(response.body(), IMAGE);
}

#[test]
fn a_game_without_a_cover_is_not_found() {
    let (core, _profile) = core(true);

    assert_eq!(cover_response(&core, "/steam%3A3527290").status(), 404);
}

#[test]
fn a_cover_that_cannot_be_downloaded_is_unavailable() {
    let (core, _profile) = core(false);

    assert_eq!(cover_response(&core, "/steam%3A620").status(), 503);
}

/// A clock that stands still; these tests do not depend on the time.
struct StoppedClock;

impl nicelibrary_core::Clock for StoppedClock {
    fn now(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
    }

    fn sleep(&self, _duration: std::time::Duration) {}
}
