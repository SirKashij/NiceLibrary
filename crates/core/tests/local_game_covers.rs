//! A Local Game's Cover is its executable's icon, at the largest size the
//! executable has, on a neutral background in Cover proportions, until the
//! user chooses an image of their own. A chosen image is copied into the
//! app's data folder, so it stays when the original is moved; the user can
//! go back to the icon at any time.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nicelibrary_core::{
    AddLocalGame, Core, CoverCache, CoverImageChoice, Icon, IconExtractor, ImagePicker, Ports, Store,
};
use tempfile::TempDir;
use nicelibrary_core::stand_ins;

/// The executables on this computer and the icons they show, as the test
/// puts them there.
#[derive(Clone, Default)]
struct Icons(Arc<Mutex<HashMap<PathBuf, Icon>>>);

impl Icons {
    fn give(&self, executable: &Path, icon: Icon) {
        self.0.lock().unwrap().insert(executable.to_path_buf(), icon);
    }
}

impl IconExtractor for Icons {
    fn largest_icon(&self, executable: &Path) -> Result<Option<Icon>, String> {
        if !executable.is_file() {
            return Err(format!("cannot read {}", executable.display()));
        }
        Ok(self.0.lock().unwrap().get(executable).cloned())
    }
}

/// A substitute for the operating system's file dialog: answers the image
/// the test says the user chose, that they cancelled, or that no dialog
/// could be shown.
#[derive(Clone)]
struct UserChooses(Arc<Mutex<Result<Option<PathBuf>, String>>>);

impl Default for UserChooses {
    /// Until the test says otherwise, the user cancels.
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Ok(None))))
    }
}

impl UserChooses {
    fn image(&self, path: &Path) {
        *self.0.lock().unwrap() = Ok(Some(path.to_path_buf()));
    }

    fn to_cancel(&self) {
        *self.0.lock().unwrap() = Ok(None);
    }

    fn to_see_no_dialog(&self) {
        *self.0.lock().unwrap() = Err("no dialog today".into());
    }
}

impl ImagePicker for UserChooses {
    fn choose_image(&self) -> Result<Option<PathBuf>, String> {
        self.0.lock().unwrap().clone()
    }
}

/// One square icon in one opaque colour.
fn icon(size: u32, rgb: [u8; 3]) -> Icon {
    Icon { width: size, height: size, rgba: [rgb[0], rgb[1], rgb[2], 255].repeat((size * size) as usize) }
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

/// The first bytes of a JPEG image; a Cover is recognised by them.
const JPEG: &[u8] = b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00 generated for the test";

/// A Cover decoded to pixels, to look at.
struct Picture {
    width: u32,
    height: u32,
    rgb: Vec<[u8; 3]>,
}

impl Picture {
    fn of(bytes: &[u8]) -> Self {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
        let frame = reader.next_frame(&mut buffer).unwrap();
        let channels = frame.color_type.samples();
        let rgb = buffer[..frame.buffer_size()].chunks(channels).map(|pixel| [pixel[0], pixel[1], pixel[2]]).collect();
        Self { width: frame.width, height: frame.height, rgb }
    }

    fn at(&self, x: u32, y: u32) -> [u8; 3] {
        self.rgb[(y * self.width + x) as usize]
    }

    /// How many pixels of the middle row have the colour `rgb`.
    fn across_the_middle(&self, rgb: [u8; 3]) -> usize {
        (0..self.width).filter(|&x| self.at(x, self.height / 2) == rgb).count()
    }
}

fn is_neutral([r, g, b]: [u8; 3]) -> bool {
    let (low, high) = (r.min(g).min(b), r.max(g).max(b));
    high - low <= 8
}

const RED: [u8; 3] = [210, 30, 40];
const BLUE: [u8; 3] = [30, 60, 220];

/// One running app with a disk of its own that games and images can be put on.
struct App {
    core: Core,
    icons: Icons,
    user: UserChooses,
    profile: TempDir,
    disk: TempDir,
}

impl App {
    fn start() -> Self {
        let profile = tempfile::tempdir().unwrap();
        let (icons, user) = (Icons::default(), UserChooses::default());
        let core = Self::core_in(profile.path(), &icons, &user);
        Self { core, icons, user, profile, disk: tempfile::tempdir().unwrap() }
    }

    fn core_in(profile: &Path, icons: &Icons, user: &UserChooses) -> Core {
        Core::new(
            Ports {
                icon_extractor: Box::new(icons.clone()),
                image_picker: Box::new(user.clone()),
                ..stand_ins::ports()
            },
            Store::open(&profile.join("data").join("nicelibrary.db")).unwrap(),
            CoverCache::new(profile.join("cache").join("covers")).with_chosen_images_in(profile.join("data").join("covers")),
        )
    }

    /// The app started again over the same profile.
    fn restart(self) -> Self {
        drop(self.core);
        let core = Self::core_in(self.profile.path(), &self.icons, &self.user);
        Self { core, ..self }
    }

    /// Puts a file with `bytes` at `relative` (folders separated by `/`) on the disk.
    fn put(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = relative.split('/').fold(self.disk.path().to_path_buf(), |path, part| path.join(part));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }

    /// Adds the executable at `relative` as a Local Game, showing `icon`, and answers its id.
    fn add_game(&self, relative: &str, icon: Option<Icon>) -> String {
        let executable = self.put(relative, b"MZ");
        if let Some(icon) = icon {
            self.icons.give(&executable, icon);
        }
        match self.core.add_local_game(executable.to_str().unwrap(), "Celeste").unwrap() {
            AddLocalGame::Added { game } => game.id,
            other => panic!("expected the game to be added, got {other:?}"),
        }
    }

    /// The user chooses the image at `path` as the Cover of `game_id`.
    fn choose_image(&self, game_id: &str, path: &Path) -> CoverImageChoice {
        self.user.image(path);
        self.core.choose_cover_image(game_id).unwrap()
    }

    fn cover_bytes(&self, game_id: &str) -> Vec<u8> {
        self.core.cover(game_id).unwrap().expect("a Cover").bytes
    }
}

#[test]
fn a_local_games_cover_is_its_executables_icon_on_a_neutral_background_in_cover_proportions() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));

    let cover = app.core.cover(&game).unwrap().expect("a Cover");

    assert_eq!(cover.media_type, "image/png");
    let picture = Picture::of(&cover.bytes);
    assert_eq!(picture.width * 3, picture.height * 2, "upright, 2:3 like Steam's Covers");
    let background = picture.at(0, 0);
    assert!(is_neutral(background), "{background:?} is not neutral");
    for (x, y) in [(picture.width - 1, 0), (0, picture.height - 1), (picture.width - 1, picture.height - 1)] {
        assert_eq!(picture.at(x, y), background, "the background is one colour");
    }
    assert_eq!(picture.at(picture.width / 2, picture.height / 2), RED, "the icon sits in the middle");
}

#[test]
fn a_large_icon_keeps_its_own_size() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));

    let picture = Picture::of(&app.cover_bytes(&game));

    assert_eq!(picture.across_the_middle(RED), 256);
}

#[test]
fn a_small_icon_is_enlarged_by_whole_pixels_so_it_stays_sharp() {
    let app = App::start();
    let mut checkered = icon(32, RED);
    // The left half blue, so a blurred edge would show colours in between.
    for y in 0..32 {
        for x in 0..16 {
            let at = ((y * 32 + x) * 4) as usize;
            checkered.rgba[at..at + 3].copy_from_slice(&BLUE);
        }
    }
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(checkered));

    let picture = Picture::of(&app.cover_bytes(&game));

    let (red, blue) = (picture.across_the_middle(RED), picture.across_the_middle(BLUE));
    assert_eq!(red, blue);
    assert!(red * 2 >= 128, "the icon is enlarged, {red} pixels of each half");
    assert_eq!(red % 16, 0, "by a whole factor");
    let background = picture.at(0, 0);
    let middle = picture.height / 2;
    assert!(
        (0..picture.width).all(|x| [RED, BLUE, background].contains(&picture.at(x, middle))),
        "no colours in between"
    );
}

#[test]
fn an_icon_larger_than_the_cover_fits_into_it() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(1024, RED)));

    let picture = Picture::of(&app.cover_bytes(&game));

    let red = picture.across_the_middle(RED);
    assert!(red > 0 && red < picture.width as usize, "{red} of {} pixels", picture.width);
    assert_eq!(picture.at(0, picture.height / 2), picture.at(0, 0), "background on both sides");
}

#[test]
fn the_background_shows_through_where_the_icon_is_transparent() {
    let app = App::start();
    let mut round = icon(256, RED);
    for pixel in round.rgba.chunks_mut(4).step_by(2) {
        pixel[3] = 0;
    }
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(round));

    let picture = Picture::of(&app.cover_bytes(&game));

    let background = picture.at(0, 0);
    let middle = picture.height / 2;
    let (left, right) = ((picture.width - 256) / 2, (picture.width + 256) / 2);
    let shown: Vec<[u8; 3]> = (left..right).map(|x| picture.at(x, middle)).collect();
    assert_eq!(shown.iter().filter(|&&rgb| rgb == background).count(), 128);
    assert_eq!(shown.iter().filter(|&&rgb| rgb == RED).count(), 128);
}

#[test]
fn a_half_transparent_icon_pixel_is_blended_with_the_background() {
    let app = App::start();
    let mut faint = icon(256, [255, 255, 255]);
    for pixel in faint.rgba.chunks_mut(4) {
        pixel[3] = 128;
    }
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(faint));

    let picture = Picture::of(&app.cover_bytes(&game));

    let [background, ..] = picture.at(0, 0);
    let [blended, ..] = picture.at(picture.width / 2, picture.height / 2);
    let expected = (u32::from(background) * 127 + 255 * 128) / 255;
    assert!(u32::from(blended).abs_diff(expected) <= 1, "{blended} instead of about {expected}");
}

#[test]
fn an_executable_without_an_icon_leaves_the_game_without_a_cover() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", None);

    assert_eq!(app.core.cover(&game).unwrap(), None);
}

#[test]
fn choosing_an_image_makes_it_the_cover() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));
    let image = generated_png(60, 90, BLUE);

    assert_eq!(app.choose_image(&game, &app.put("Pictures/celeste.png", &image)), CoverImageChoice::Chosen);

    let cover = app.core.cover(&game).unwrap().expect("a Cover");
    assert_eq!(cover.bytes, image);
    assert_eq!(cover.media_type, "image/png");
    assert!(app.core.cover_image_chosen(&game).unwrap());
}

#[test]
fn a_jpeg_image_may_be_chosen_too() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", None);

    assert_eq!(app.choose_image(&game, &app.put("Pictures/celeste.jpg", JPEG)), CoverImageChoice::Chosen);

    let cover = app.core.cover(&game).unwrap().expect("a Cover");
    assert_eq!((cover.bytes.as_slice(), cover.media_type), (JPEG, "image/jpeg"));
}

#[test]
fn a_chosen_image_stays_the_cover_after_the_original_is_moved_and_the_app_restarts() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));
    let image = generated_png(60, 90, BLUE);
    let original = app.put("Pictures/celeste.png", &image);
    app.choose_image(&game, &original);

    fs::rename(&original, app.disk.path().join("moved.png")).unwrap();
    let app = app.restart();

    assert_eq!(app.cover_bytes(&game), image);
    assert!(app.core.cover_image_chosen(&game).unwrap());
}

#[test]
fn a_chosen_image_is_kept_in_the_apps_data_folder_not_in_the_cache() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));
    let image = generated_png(60, 90, BLUE);
    app.choose_image(&game, &app.put("Pictures/celeste.png", &image));

    // Whatever clears the cache leaves the data folder alone. The cache may
    // not even exist: these Games have no Cover from Steam.
    let _ = fs::remove_dir_all(app.profile.path().join("cache"));
    let app = app.restart();

    assert_eq!(app.cover_bytes(&game), image);
}

#[test]
fn going_back_to_the_icon_makes_the_icon_the_cover_again_also_after_a_restart() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));
    let icon_cover = app.cover_bytes(&game);
    app.choose_image(&game, &app.put("Pictures/celeste.png", &generated_png(60, 90, BLUE)));

    app.core.remove_cover_image(&game).unwrap();

    assert_eq!(app.cover_bytes(&game), icon_cover);
    assert!(!app.core.cover_image_chosen(&game).unwrap());
    let app = app.restart();
    assert_eq!(app.cover_bytes(&game), icon_cover);
    assert!(!app.core.cover_image_chosen(&game).unwrap());
}

#[test]
fn going_back_to_the_icon_without_a_chosen_image_changes_nothing() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));
    let icon_cover = app.cover_bytes(&game);

    app.core.remove_cover_image(&game).unwrap();

    assert_eq!(app.cover_bytes(&game), icon_cover);
}

#[test]
fn choosing_another_image_replaces_the_first() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", None);
    app.choose_image(&game, &app.put("Pictures/first.png", &generated_png(60, 90, BLUE)));
    let second = generated_png(60, 90, RED);

    app.choose_image(&game, &app.put("Pictures/second.png", &second));

    assert_eq!(app.cover_bytes(&game), second);
}

#[test]
fn a_file_that_is_not_an_image_is_refused_and_the_cover_stays_as_it_was() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", None);
    let image = generated_png(60, 90, BLUE);
    app.choose_image(&game, &app.put("Pictures/celeste.png", &image));

    let choice = app.choose_image(&game, &app.put("Pictures/notes.png", b"not an image at all"));

    assert_eq!(choice, CoverImageChoice::NotAnImage);
    assert_eq!(app.cover_bytes(&game), image);
}

#[test]
fn cancelling_the_choice_changes_nothing() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));
    let icon_cover = app.cover_bytes(&game);
    app.user.to_cancel();

    assert_eq!(app.core.choose_cover_image(&game).unwrap(), CoverImageChoice::Cancelled);

    assert_eq!(app.cover_bytes(&game), icon_cover);
    assert!(!app.core.cover_image_chosen(&game).unwrap());
}

#[test]
fn a_failing_file_dialog_is_an_error() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", None);
    app.user.to_see_no_dialog();

    assert!(app.core.choose_cover_image(&game).is_err());
}

#[test]
fn only_a_local_game_in_the_library_can_get_an_image() {
    let app = App::start();
    app.user.image(&app.put("Pictures/celeste.png", &generated_png(60, 90, BLUE)));

    assert!(app.core.choose_cover_image("steam:620").is_err());
    assert!(app.core.choose_cover_image("local:999").is_err());
    assert!(!app.core.cover_image_chosen("local:999").unwrap());
}

#[test]
fn each_local_game_has_its_own_cover() {
    let app = App::start();
    let celeste = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));
    let hollow_knight = app.add_game("Games/HK/hollow_knight.exe", Some(icon(256, BLUE)));
    let image = generated_png(60, 90, BLUE);
    app.choose_image(&celeste, &app.put("Pictures/celeste.png", &image));

    assert_eq!(app.cover_bytes(&celeste), image);
    let picture = Picture::of(&app.cover_bytes(&hollow_knight));
    assert_eq!(picture.at(picture.width / 2, picture.height / 2), BLUE);
}

#[test]
fn a_chosen_image_shows_even_when_the_executable_is_gone() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));
    let image = generated_png(60, 90, BLUE);
    app.choose_image(&game, &app.put("Pictures/celeste.png", &image));

    fs::remove_dir_all(app.disk.path().join("Games")).unwrap();

    assert_eq!(app.cover_bytes(&game), image);
}

/// The distance between the hue of `game_id`'s Cover and `degrees` on the colour wheel.
fn cover_hue_distance(app: &App, game_id: &str, degrees: f64) -> f64 {
    let hue = app.core.cover_hue(game_id).unwrap().expect("a Cover hue").hue;
    let d = (hue - degrees).rem_euclid(360.0);
    d.min(360.0 - d)
}

#[test]
fn the_detail_page_of_a_local_game_takes_its_tint_from_the_icon_cover() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));

    // RED is a hue of about 357 degrees; the neutral background does not count.
    assert!(cover_hue_distance(&app, &game, 357.0) <= 8.0);
}

#[test]
fn the_detail_page_of_a_local_game_takes_its_tint_from_a_chosen_image() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", Some(icon(256, RED)));
    app.choose_image(&game, &app.put("Pictures/celeste.png", &generated_png(60, 90, BLUE)));

    // BLUE is a hue of about 230 degrees.
    assert!(cover_hue_distance(&app, &game, 230.0) <= 8.0);
}

#[test]
fn a_local_game_without_a_cover_has_no_hue() {
    let app = App::start();
    let game = app.add_game("Games/Celeste/Celeste.exe", None);

    assert_eq!(app.core.cover_hue(&game).unwrap(), None);
}
