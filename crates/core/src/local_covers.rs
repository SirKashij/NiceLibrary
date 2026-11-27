//! Covers of Local Games: an image the user chose, else the icon of the
//! Game's executable on a neutral background (ADR-0005).
//!
//! The chain a Local Game's Cover is looked for in, first match wins:
//!
//! 1. the image the user chose, copied into the app's data folder;
//! 2. the Steam Cover of its Cover Match, once Cover Matches exist (#37);
//! 3. its executable's icon, at the largest size the executable has.
//!
//! Without any of them the Game has no Cover and the interface shows a
//! placeholder.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;

use serde::Serialize;

use crate::covers::{self, Cover, CoverCache};
use crate::local_games::StoredLocalGame;
use crate::Error;

/// Port: reads the icon an executable shows itself with.
///
/// Contract: `Ok(Some(icon))` with the largest image of the executable's
/// first icon, at its own size, unscaled; `Ok(None)` when the executable has
/// no icon of its own. `Err` when the file cannot be read as an executable,
/// for instance because it is gone, described for a log or a developer.
pub trait IconExtractor: Send + Sync {
    fn largest_icon(&self, executable: &Path) -> Result<Option<Icon>, String>;
}

/// Port: asks the user to choose an image, as a file dialog does.
///
/// Contract: `Ok(Some(path))` with the absolute path of the file the user
/// chose, which exists; `Ok(None)` when they cancel. Blocks until they have
/// decided. `Err` when no dialog can be shown, described for a log or a
/// developer.
pub trait ImagePicker: Send + Sync {
    fn choose_image(&self) -> Result<Option<PathBuf>, String>;
}

/// An executable's icon as pixels: `width` × `height`, row by row from the
/// top, four bytes per pixel in the order red, green, blue, alpha. Alpha is
/// not premultiplied: 0 is fully transparent, 255 fully opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// How choosing an image as a Local Game's Cover went, when nothing failed.
/// Every outcome but `Chosen` changed nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum CoverImageChoice {
    /// The image is the Game's Cover now.
    Chosen,
    /// The user chose no image.
    Cancelled,
    /// The file is not an image a Cover may be: PNG or JPEG.
    NotAnImage,
}

/// The Cover of the Local Game `game`, following the chain at the top of
/// this module.
pub(crate) fn cover(
    covers: &CoverCache,
    icons: &dyn IconExtractor,
    game: &StoredLocalGame,
) -> Result<Option<Cover>, Error> {
    if let Some(chosen) = chosen_image(covers, game.id)? {
        return Ok(Some(chosen));
    }
    // A Cover Match (#37) goes here: the matched title's Steam Cover, through
    // `CoverCache::steam_cover`, before the icon.
    icon_cover(covers, icons, &game.executable)
}

/// The image the user chose as the Cover of the Local Game `id`, if they chose one.
pub(crate) fn chosen_image(covers: &CoverCache, id: i64) -> Result<Option<Cover>, Error> {
    let file = chosen_image_file(covers, id);
    match fs::read(&file) {
        Ok(bytes) => Ok(Cover::from_bytes(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::new(format!("cannot read {}: {error}", file.display()))),
    }
}

/// Copies the image at `image` into the app's data folder as the Cover of
/// the Local Game `id`, in place of any image chosen before.
pub(crate) fn keep_chosen_image(covers: &CoverCache, id: i64, image: &Path) -> Result<CoverImageChoice, Error> {
    let bytes = fs::read(image).map_err(|error| Error::new(format!("cannot read {}: {error}", image.display())))?;
    if Cover::from_bytes(bytes.clone()).is_none() {
        return Ok(CoverImageChoice::NotAnImage);
    }
    let file = chosen_image_file(covers, id);
    covers::keep(&covers.chosen_images, &file, &bytes)
        .map_err(|error| Error::new(format!("cannot keep {}: {error}", file.display())))?;
    Ok(CoverImageChoice::Chosen)
}

/// Forgets the image the user chose as the Cover of the Local Game `id`, if
/// they chose one.
pub(crate) fn remove_chosen_image(covers: &CoverCache, id: i64) -> Result<(), Error> {
    let file = chosen_image_file(covers, id);
    match fs::remove_file(&file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::new(format!("cannot remove {}: {error}", file.display()))),
    }
}

/// Where the image chosen for the Local Game `id` is kept. Local Games' ids
/// are never used again, so a file left behind never shows for another Game.
fn chosen_image_file(covers: &CoverCache, id: i64) -> PathBuf {
    covers.chosen_images.join(format!("local-{id}"))
}

/// The Cover made from the icon of the executable at `executable`, made once
/// per start of the app.
fn icon_cover(covers: &CoverCache, icons: &dyn IconExtractor, executable: &str) -> Result<Option<Cover>, Error> {
    let mut made = covers.icon_covers.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(cover) = made.get(executable) {
        return Ok(cover.clone());
    }
    let icon = icons
        .largest_icon(Path::new(executable))
        .map_err(|reason| Error::new(format!("cannot read the icon of {executable}: {reason}")))?;
    let cover = icon.filter(Icon::is_whole).map(|icon| icon_on_background(&icon));
    made.insert(executable.to_owned(), cover.clone());
    Ok(cover)
}

/// The size of a Cover made from an icon: upright, 2:3 like Steam's Covers.
const COVER_WIDTH: u32 = 600;
const COVER_HEIGHT: u32 = 900;

/// The neutral graphite an icon is placed on.
const BACKGROUND: [u8; 3] = [0x2a, 0x2c, 0x2f];

/// Icons smaller than this are enlarged by whole pixels towards it, so they stay sharp.
const SMALL_ICON: u32 = 256;

/// Icons larger than this are shrunk to it or below, so the background frames them.
const LARGE_ICON: u32 = 400;

impl Icon {
    /// Whether the pixels are as many as the size says, and there are any.
    fn is_whole(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.rgba.len() as u64 == u64::from(self.width) * u64::from(self.height) * 4
    }

    /// The pixel at `x`, `y` on the background, as red, green and blue.
    fn on_background(&self, x: u32, y: u32) -> [u8; 3] {
        let at = ((y * self.width + x) * 4) as usize;
        let alpha = u32::from(self.rgba[at + 3]);
        std::array::from_fn(|channel| {
            let (icon, background) = (u32::from(self.rgba[at + channel]), u32::from(BACKGROUND[channel]));
            ((icon * alpha + background * (255 - alpha) + 127) / 255) as u8
        })
    }
}

/// `icon` in the middle of a Cover-sized neutral background, as a PNG image.
/// A small icon is enlarged by a whole factor, a large one shrunk by one.
fn icon_on_background(icon: &Icon) -> Cover {
    let longest = icon.width.max(icon.height);
    let (enlarge, shrink) = if longest < SMALL_ICON { (SMALL_ICON / longest, 1) } else { (1, longest.div_ceil(LARGE_ICON)) };
    let width = (icon.width * enlarge / shrink).clamp(1, COVER_WIDTH);
    let height = (icon.height * enlarge / shrink).clamp(1, COVER_HEIGHT);
    let (left, top) = ((COVER_WIDTH - width) / 2, (COVER_HEIGHT - height) / 2);
    let mut pixels = BACKGROUND.repeat((COVER_WIDTH * COVER_HEIGHT) as usize);
    for y in 0..height {
        for x in 0..width {
            let rgb = if shrink == 1 {
                icon.on_background(x / enlarge, y / enlarge)
            } else {
                average(icon, x * shrink, y * shrink, shrink)
            };
            let at = (((top + y) * COVER_WIDTH + left + x) * 3) as usize;
            pixels[at..at + 3].copy_from_slice(&rgb);
        }
    }
    Cover { bytes: png(&pixels), media_type: "image/png" }
}

/// The average of the `size` × `size` pixels of `icon` from `x`, `y` on, on the background.
fn average(icon: &Icon, x: u32, y: u32, size: u32) -> [u8; 3] {
    let mut sum = [0u32; 3];
    let mut count = 0;
    for y in y..(y + size).min(icon.height) {
        for x in x..(x + size).min(icon.width) {
            for (sum, value) in sum.iter_mut().zip(icon.on_background(x, y)) {
                *sum += u32::from(value);
            }
            count += 1;
        }
    }
    sum.map(|sum| (sum / count.max(1)) as u8)
}

/// A Cover-sized picture of red, green and blue `pixels` as a PNG image.
fn png(pixels: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, COVER_WIDTH, COVER_HEIGHT);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    // Writing to memory fails only for a wrong size, which the constants rule out.
    let mut writer = encoder.write_header().expect("a PNG header");
    writer.write_image_data(pixels).expect("pixels of the Cover's size");
    writer.finish().expect("a whole PNG image");
    bytes
}
