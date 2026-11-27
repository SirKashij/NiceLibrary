//! Covers: resolves a Game's Cover and caches the image on disk.
//!
//! Covers are downloaded from Steam at runtime and kept only on this computer
//! (ADR-0005). A Steam Game's Cover is its upright library image; a Local
//! Game's is found in `local_covers`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::Error;

/// Port: fetches a file from the web.
///
/// Contract: `Ok(Some(bytes))` when the server sends the file, `Ok(None)`
/// when the server answers that it does not exist (HTTP 404 or 410), and
/// `Err` with a reason when the file could not be fetched for any other
/// reason: no network, a server error, a timeout.
pub trait Downloader: Send + Sync {
    fn download(&self, url: &str) -> Result<Option<Vec<u8>>, String>;
}

/// The image that represents a Game in the Library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cover {
    pub bytes: Vec<u8>,
    /// The image format, as a media type such as `image/jpeg`.
    pub media_type: &'static str,
}

/// The folder on this computer that keeps downloaded Covers, one file per
/// Game, named after it, and the folder that keeps the images the user chose
/// as Covers.
pub struct CoverCache {
    folder: PathBuf,
    /// The Steam Games Steam had no image for since the app started. They
    /// are asked again on the next start, in case Steam added one.
    without_image: Mutex<HashSet<u32>>,
    /// Where the images the user chose are kept: data, not cache, since they
    /// cannot be fetched again.
    pub(crate) chosen_images: PathBuf,
    /// The Covers made from executables' icons since the app started, by
    /// executable; `None` for one without an icon.
    pub(crate) icon_covers: Mutex<HashMap<String, Option<Cover>>>,
}

impl CoverCache {
    /// The cache in `folder`; the folder is created when the first Cover is
    /// kept. Images the user chooses are kept in its `chosen` folder unless
    /// [`Self::with_chosen_images_in`] names another.
    pub fn new(folder: PathBuf) -> Self {
        let chosen_images = folder.join("chosen");
        Self { folder, without_image: Mutex::default(), chosen_images, icon_covers: Mutex::default() }
    }

    /// Keeps the images the user chooses as Covers in `folder`, which
    /// belongs in the app's data folder: a cache may be cleared, and a
    /// chosen image cannot be fetched again.
    pub fn with_chosen_images_in(self, folder: PathBuf) -> Self {
        Self { chosen_images: folder, ..self }
    }

    fn without_image(&self) -> MutexGuard<'_, HashSet<u32>> {
        // The set stays consistent even if a panic interrupted another user.
        self.without_image.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The Cover of the Steam Game `app_id`: from the cache when it is there,
    /// else downloaded from Steam and kept.
    ///
    /// Fails when Steam cannot be reached; that says nothing about whether
    /// the Game has a Cover.
    pub(crate) fn steam_cover(&self, downloader: &dyn Downloader, app_id: u32) -> Result<Option<Cover>, Error> {
        let file = self.folder.join(format!("steam-{app_id}"));
        if let Some(cover) = fs::read(&file).ok().and_then(Cover::from_bytes) {
            return Ok(Some(cover));
        }
        if self.without_image().contains(&app_id) {
            return Ok(None);
        }
        let cover = download(downloader, app_id)?;
        match &cover {
            // A Cover that cannot be kept is downloaded again next time.
            Some(cover) => {
                let _ = keep(&self.folder, &file, &cover.bytes);
            }
            None => {
                self.without_image().insert(app_id);
            }
        }
        Ok(cover)
    }
}

/// Writes `bytes` to `file` in `folder` in one step, so that a crash never
/// leaves a half-written Cover behind.
pub(crate) fn keep(folder: &Path, file: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static WRITES: AtomicU64 = AtomicU64::new(0);
    let partial =
        file.with_extension(format!("partial-{}-{}", std::process::id(), WRITES.fetch_add(1, Ordering::Relaxed)));
    let kept = fs::create_dir_all(folder)
        .and_then(|()| fs::write(&partial, bytes))
        .and_then(|()| fs::rename(&partial, file));
    if kept.is_err() {
        let _ = fs::remove_file(&partial);
    }
    kept
}

impl Cover {
    /// The Cover in `bytes`, if they hold an image in a format a Cover may have.
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Option<Self> {
        let media_type = media_type(&bytes)?;
        Some(Self { bytes, media_type })
    }
}

/// The images Steam may have for a Steam Game, in the order they are
/// preferred as its Cover: the upright library image, then the wide header.
fn steam_images(app_id: u32) -> [String; 2] {
    ["library_600x900.jpg", "header.jpg"]
        .map(|file| format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{app_id}/{file}"))
}

fn download(downloader: &dyn Downloader, app_id: u32) -> Result<Option<Cover>, Error> {
    for url in steam_images(app_id) {
        let bytes = downloader.download(&url).map_err(|reason| Error::new(format!("cannot download {url}: {reason}")))?;
        if let Some(bytes) = bytes {
            // Something in between, such as a network's sign-in page, may
            // answer in Steam's place.
            return Cover::from_bytes(bytes)
                .map(Some)
                .ok_or_else(|| Error::new(format!("{url} did not answer with an image")));
        }
    }
    Ok(None)
}

/// The image format of `bytes`, recognised by its signature; `None` when it
/// is not an image format a Cover may have.
fn media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else {
        None
    }
}
