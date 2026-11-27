//! A Detail Page takes its Tint from the dominant hue of the Game's Cover:
//! the hue that most of the Cover's colourful pixels share, read from the
//! cached Cover image. Grey, black and white do not count. A Game without a
//! Cover, or with a Cover that has no colour to speak of, has no Cover hue
//! and its Detail Page keeps the neutral look.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nicelibrary_core::{Core, CoverCache, CoverHue, Downloader, Ports, Store};
use tempfile::TempDir;
use nicelibrary_core::stand_ins;

/// A substitute for Steam's artwork servers: serves the files the test puts
/// there, answers "not found" for every other address, and can go offline.
#[derive(Clone, Default)]
struct FakeSteam {
    state: Arc<Mutex<(HashMap<String, Vec<u8>>, bool)>>,
}

impl FakeSteam {
    /// Serves `bytes` as the upright library image of the Steam Game `app_id`.
    fn serve_cover(&self, app_id: u32, bytes: Vec<u8>) {
        let url = format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{app_id}/library_600x900.jpg");
        self.state.lock().unwrap().0.insert(url, bytes);
    }

    fn go_offline(&self) {
        self.state.lock().unwrap().1 = true;
    }
}

impl Downloader for FakeSteam {
    fn download(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        let state = self.state.lock().unwrap();
        if state.1 {
            return Err("no network connection".into());
        }
        Ok(state.0.get(url).cloned())
    }
}

/// The pixels of a 60 x 90 Cover, row by row, painted by `paint(x, y)`.
fn pixels(paint: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    (0..90).flat_map(|y| (0..60).map(move |x| (x, y))).flat_map(|(x, y)| paint(x, y)).collect()
}

/// A 60 x 90 PNG Cover painted by `paint(x, y)`.
fn png_cover(paint: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 60, 90);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels(paint)).unwrap();
    writer.finish().unwrap();
    bytes
}

/// A 60 x 90 JPEG Cover, the format Steam serves, painted by `paint(x, y)`.
fn jpeg_cover(paint: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let encoder = jpeg_encoder::Encoder::new(&mut bytes, 90);
    encoder.encode(&pixels(paint), 60, 90, jpeg_encoder::ColorType::Rgb).unwrap();
    bytes
}

/// The user profile of one test: its database and its cache folder.
struct Profile(TempDir);

impl Profile {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    /// Starts the app over this profile, reaching Steam's artwork through `steam`.
    fn start(&self, steam: &FakeSteam) -> Core {
        Core::new(
            Ports {
                downloader: Box::new(steam.clone()),
                ..stand_ins::ports()
            },
            Store::open(&self.0.path().join("nicelibrary.db")).unwrap(),
            CoverCache::new(self.0.path().join("covers")),
        )
    }
}

/// The Cover hue of the Steam Game `app_id` whose Cover is `cover`.
fn hue_of(app_id: u32, cover: Vec<u8>) -> Option<CoverHue> {
    let steam = FakeSteam::default();
    steam.serve_cover(app_id, cover);
    Profile::new().start(&steam).cover_hue(&format!("steam:{app_id}")).unwrap()
}

/// The distance between two hues on the colour wheel, in degrees.
fn hue_distance(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

#[track_caller]
fn assert_hue(hue: Option<CoverHue>, degrees: f64) {
    let hue = hue.expect("a Cover hue");
    assert!(hue_distance(hue.hue, degrees) <= 8.0, "expected a hue near {degrees}, got {hue:?}");
    assert!((0.0..360.0).contains(&hue.hue), "{hue:?}");
    assert!((0.0..=1.0).contains(&hue.saturation), "{hue:?}");
}

#[test]
fn a_steam_cover_in_one_colour_has_that_colours_hue() {
    // Steam serves JPEG images; a teal of hue 180.
    assert_hue(hue_of(620, jpeg_cover(|_, _| [30, 170, 170])), 180.0);
}

#[test]
fn a_png_cover_has_a_hue_as_well() {
    // An orange of hue 30.
    assert_hue(hue_of(620, png_cover(|_, _| [230, 130, 30])), 30.0);
}

#[test]
fn the_dominant_hue_is_the_one_most_of_the_cover_shows_not_the_average() {
    // Two thirds blue (hue 220), one third red (hue 0): the average would be purple.
    let cover = png_cover(|x, _| if x < 40 { [40, 90, 210] } else { [210, 30, 30] });

    assert_hue(hue_of(620, cover), 220.0);
}

#[test]
fn grey_black_and_white_do_not_count() {
    // A dark grey Cover with a white title, and a small green emblem.
    let cover = jpeg_cover(|x, y| match (x, y) {
        (20..40, 30..50) => [40, 190, 70],
        (_, 70..80) => [250, 250, 250],
        _ => [35, 36, 38],
    });

    assert_hue(hue_of(620, cover), 132.0);
}

#[test]
fn a_cover_without_colour_has_no_hue() {
    let grey = png_cover(|x, y| {
        let v = ((x + y) * 2) as u8;
        [v, v, v]
    });

    assert_eq!(hue_of(620, grey), None);
}

#[test]
fn a_cover_whose_colour_is_only_a_speck_has_no_hue() {
    let speck = png_cover(|x, y| if x < 3 && y < 3 { [220, 20, 20] } else { [20, 20, 22] });

    assert_eq!(hue_of(620, speck), None);
}

#[test]
fn a_faded_cover_has_a_lower_saturation_than_a_vivid_one() {
    let vivid = hue_of(620, png_cover(|_, _| [220, 30, 30])).unwrap();
    let faded = hue_of(620, png_cover(|_, _| [170, 110, 110])).unwrap();

    assert!(faded.saturation < vivid.saturation, "{faded:?} {vivid:?}");
}

#[test]
fn a_game_without_a_cover_has_no_hue() {
    let steam = FakeSteam::default();

    assert_eq!(Profile::new().start(&steam).cover_hue("steam:3527290").unwrap(), None);
}

#[test]
fn a_game_that_is_not_in_the_library_has_no_hue() {
    let steam = FakeSteam::default();

    assert_eq!(Profile::new().start(&steam).cover_hue("local:1").unwrap(), None);
}

#[test]
fn the_hue_is_read_from_the_cached_cover_also_without_a_network_connection() {
    let steam = FakeSteam::default();
    steam.serve_cover(620, jpeg_cover(|_, _| [30, 170, 170]));
    let profile = Profile::new();
    profile.start(&steam).cover("steam:620").unwrap();
    steam.go_offline();

    assert_hue(profile.start(&steam).cover_hue("steam:620").unwrap(), 180.0);
}

#[test]
fn a_cover_that_cannot_be_downloaded_now_is_a_failure_not_a_missing_hue() {
    let steam = FakeSteam::default();
    steam.go_offline();

    assert!(Profile::new().start(&steam).cover_hue("steam:620").is_err());
}
