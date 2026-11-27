//! Updates: a newer version of the app, published on GitHub Releases, which
//! the app offers and installs once the user agreed. Its Update Package is
//! installed only when its signature proves it was made with the project's
//! updater key (ADR-0006), whose private half exists only as a CI secret.
//!
//! The release workflow (.github/workflows/release.yml) publishes the
//! manifest in the shape Tauri's updater reads, `latest.json`:
//!
//! ```json
//! { "version": "0.2.0", "notes": "...", "pub_date": "...",
//!   "platforms": { "windows-x86_64": { "url": "https://...", "signature": "<base64>" } } }
//! ```
//!
//! The signature is `tauri signer sign --app-version <version>`'s: base64 of
//! a minisign signature whose trusted comment names the version, so an older
//! signed package cannot be passed off as a newer one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::store::Store;

/// The setting that switches the automatic Update Check off ("false"); on while it is missing.
pub(crate) const CHECK_FOR_UPDATES: &str = "check_for_updates";

/// How long after the start the first automatic Update Check runs.
pub(crate) const FIRST_AUTOMATIC_CHECK_AFTER: Duration = Duration::from_secs(60);

/// How long the automatic Update Checks wait after one before the next.
pub(crate) const AUTOMATIC_CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

/// Where the app looks for Updates and what it trusts, given by the desktop shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateChannel {
    /// The manifest of the latest release (`latest.json`).
    pub manifest_url: String,
    /// Every Update Package's URL begins with this; any other is refused.
    pub package_url_prefix: String,
    /// The manifest's key for this computer's Update Package, such as `windows-x86_64`.
    pub platform: String,
    /// The public updater key: base64 of the minisign public key file, as
    /// `tauri signer generate` writes it.
    pub public_key: String,
    /// The folder an Update Package is written to before it is started.
    pub download_folder: PathBuf,
    /// What the Update Package is started with, so it installs over the app
    /// that runs and starts it again.
    pub installer_arguments: Vec<String>,
}

/// A newer version of the app, as the interface shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    /// Such as `0.2.0`.
    pub version: String,
    /// What is new, as the release says; empty when it says nothing.
    pub notes: String,
}

/// What an Update Check found, when nothing failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum UpdateCheck {
    /// The version that runs is the latest, or no release is published yet.
    UpToDate,
    /// A newer version is out; it is installed only once the user agrees
    /// (see [`crate::Core::install_update`]).
    Available { update: Update },
}

/// How installing an Update went, when nothing failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum UpdateInstall {
    /// Its Update Package is verified and runs; the app quits so it can be
    /// replaced, and the installer starts it again.
    Started,
    /// The Update Package's signature is not one of the updater key, or not
    /// for this version: nothing was installed.
    InvalidSignature,
    /// No Update Check offered this version since the app started, so the
    /// user cannot have agreed to it: nothing was installed.
    NotOffered,
}

/// What an automatic Update Check found, as it happens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum AutomaticUpdateCheck {
    /// The automatic Update Check is switched off: nothing was asked.
    Skipped,
    UpToDate,
    /// A newer version is out; the interface asks the user whether to install it.
    Found { update: Update },
    Failed { reason: Error },
}

/// The Update a check offered, with what installing it needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Offered {
    pub update: Update,
    pub url: String,
    pub signature: String,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    platforms: HashMap<String, Platform>,
}

#[derive(Deserialize)]
struct Platform {
    url: String,
    signature: String,
}

/// What the manifest `bytes` offers to the app of version `running`:
/// `None` when it is no newer.
pub(crate) fn offered(channel: &UpdateChannel, running: &str, bytes: &[u8]) -> Result<Option<Offered>, Error> {
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|error| Error::new(format!("update check: the release manifest cannot be read: {error}")))?;
    let version = parse_version(&manifest.version)?;
    let running = parse_version(running)?;
    if version <= running {
        return Ok(None);
    }
    let Some(platform) = manifest.platforms.get(&channel.platform) else {
        return Err(Error::new(format!(
            "update check: the release {version} has no update package for {}",
            channel.platform
        )));
    };
    if !platform.url.starts_with(&channel.package_url_prefix) {
        return Err(Error::new(format!("update check: the release {version} names a package outside its releases")));
    }
    Ok(Some(Offered {
        update: Update { version: version.to_string(), notes: manifest.notes.unwrap_or_default() },
        url: platform.url.clone(),
        signature: platform.signature.clone(),
    }))
}

fn parse_version(text: &str) -> Result<semver::Version, Error> {
    semver::Version::parse(text.trim().trim_start_matches('v'))
        .map_err(|_| Error::new(format!("update check: not a version: {text}")))
}

/// Whether `signature` is the updater key's (`public_key`) signature of
/// `package` as the Update Package of `version`.
pub(crate) fn signed(public_key: &str, package: &[u8], signature: &str, version: &str) -> bool {
    let Some(public_key) = decoded(public_key).and_then(|text| PublicKey::decode(&text).ok()) else {
        return false;
    };
    let Some(signature) = decoded(signature).and_then(|text| Signature::decode(&text).ok()) else {
        return false;
    };
    // Tauri's signer signs the package's hash (minisign's "ED"); a legacy signature of the bytes themselves is refused.
    if public_key.verify(package, &signature, false).is_err() {
        return false;
    }
    let wanted = format!("version:{version}");
    signature.trusted_comment().split('\t').any(|field| field == wanted)
}

/// Base64 `text` decoded to text.
fn decoded(text: &str) -> Option<String> {
    String::from_utf8(STANDARD.decode(text.trim()).ok()?).ok()
}

/// Whether the app checks for Updates on its own.
pub(crate) fn checks_for_updates(store: &Store) -> Result<bool, Error> {
    Ok(store.setting(CHECK_FOR_UPDATES)?.as_deref() != Some("false"))
}

pub(crate) fn set_check_for_updates(store: &Store, on: bool) -> Result<(), Error> {
    store.set_setting(CHECK_FOR_UPDATES, if on { "true" } else { "false" })
}

/// Writes the verified Update Package of `version` into `folder`, and
/// answers the file. The name comes from the parsed version, never from the
/// release, so it is a plain name inside `folder`.
pub(crate) fn write_package(folder: &Path, version: &str, package: &[u8]) -> Result<PathBuf, Error> {
    let cannot = |error: std::io::Error| Error::new(format!("update: the update package cannot be kept: {error}"));
    std::fs::create_dir_all(folder).map_err(cannot)?;
    let file = folder.join(format!("NiceLibrary-{version}-setup.exe"));
    std::fs::write(&file, package).map_err(cannot)?;
    Ok(file)
}
