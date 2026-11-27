//! Updates: the app asks GitHub Releases whether a newer version is out,
//! offers it, and installs its Update Package only once the user agreed and
//! the package's signature proves it was made with the project's updater
//! key. A setting switches the automatic check off.

use std::collections::HashMap;
use std::io::Cursor;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nicelibrary_core::{
    AutomaticUpdateCheck, Core, CoverCache, Downloader, Ports, ProcessStarter, Store, Update, UpdateChannel,
    UpdateCheck, UpdateInstall,
};
use nicelibrary_core::stand_ins;
use tempfile::TempDir;

const MANIFEST: &str = "https://github.com/SirKashij/NiceLibrary/releases/latest/download/latest.json";
const RELEASES: &str = "https://github.com/SirKashij/NiceLibrary/releases/download/";
const PLATFORM: &str = "windows-x86_64";
/// A version far ahead of the one that runs.
const NEWER: &str = "99.0.0";
/// The bytes of the made-up Update Package.
const PACKAGE: &[u8] = b"MZ the installer of a newer NiceLibrary";

fn package_url(version: &str) -> String {
    format!("{RELEASES}v{version}/NiceLibrary_{version}_x64-setup.exe")
}

/// A minisign key pair like the project's updater key, made for one test.
struct UpdaterKey {
    public: minisign::PublicKey,
    secret: minisign::SecretKey,
}

impl UpdaterKey {
    fn new() -> Self {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        Self { public: pair.pk, secret: pair.sk }
    }

    /// The public key as the app is given it: base64 of the minisign key file,
    /// as `tauri signer generate` writes it.
    fn public_key(&self) -> String {
        STANDARD.encode(self.public.to_box().unwrap().to_string())
    }

    /// A signature of `bytes` as `tauri signer sign --app-version` makes it:
    /// base64 of the minisign signature file, with the version in its
    /// trusted comment.
    fn sign(&self, bytes: &[u8], version: &str) -> String {
        let trusted = format!("timestamp:1795608000\tfile:NiceLibrary_{version}_x64-setup.exe\tversion:{version}");
        self.sign_with_comment(bytes, &trusted)
    }

    fn sign_with_comment(&self, bytes: &[u8], trusted_comment: &str) -> String {
        let signature = minisign::sign(
            Some(&self.public),
            &self.secret,
            Cursor::new(bytes),
            Some(trusted_comment),
            Some("signature from tauri secret key"),
        )
        .unwrap();
        STANDARD.encode(signature.to_string())
    }
}

/// GitHub Releases as these tests see it: answers each URL with what the map
/// holds, 404 for the rest, and remembers every URL asked for.
#[derive(Clone, Default)]
struct Releases {
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    asked: Arc<Mutex<Vec<String>>>,
    offline: Arc<Mutex<bool>>,
}

impl Releases {
    fn serve(&self, url: &str, bytes: &[u8]) {
        self.files.lock().unwrap().insert(url.to_owned(), bytes.to_vec());
    }

    /// Publishes `version` with `package` as its Update Package and
    /// `signature` as its signature.
    fn publish(&self, version: &str, package: &[u8], signature: &str) {
        let manifest = serde_json::json!({
            "version": version,
            "notes": format!("What is new in {version}"),
            "pub_date": "2026-11-27T12:00:00Z",
            "platforms": { PLATFORM: { "signature": signature, "url": package_url(version) } },
        });
        self.serve(MANIFEST, manifest.to_string().as_bytes());
        self.serve(&package_url(version), package);
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }

    fn go_offline(&self) {
        *self.offline.lock().unwrap() = true;
    }
}

impl Downloader for Releases {
    fn download(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        self.asked.lock().unwrap().push(url.to_owned());
        if *self.offline.lock().unwrap() {
            return Err("no network connection".into());
        }
        Ok(self.files.lock().unwrap().get(url).cloned())
    }
}

/// One program the core started.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Started {
    executable: PathBuf,
    arguments: Vec<String>,
    contents: Vec<u8>,
}

/// A computer that records what it is asked to start, and the file's bytes at that moment.
#[derive(Clone, Default)]
struct Starter(Arc<Mutex<Vec<Started>>>);

impl Starter {
    fn started(&self) -> Vec<Started> {
        self.0.lock().unwrap().clone()
    }
}

impl ProcessStarter for Starter {
    fn start(&self, executable: &Path, arguments: &[String], _working_directory: &Path) -> Result<(), String> {
        let contents = std::fs::read(executable).map_err(|error| error.to_string())?;
        self.0.lock().unwrap().push(Started {
            executable: executable.to_owned(),
            arguments: arguments.to_vec(),
            contents,
        });
        Ok(())
    }
}

/// The app's data folder and the folder Update Packages are kept in, across the starts of one test.
struct Profile {
    data: TempDir,
    downloads: TempDir,
    key: UpdaterKey,
    releases: Releases,
    starter: Starter,
}

impl Profile {
    fn new() -> Self {
        Self {
            data: tempfile::tempdir().unwrap(),
            downloads: tempfile::tempdir().unwrap(),
            key: UpdaterKey::new(),
            releases: Releases::default(),
            starter: Starter::default(),
        }
    }

    fn channel(&self) -> UpdateChannel {
        UpdateChannel {
            manifest_url: MANIFEST.to_owned(),
            package_url_prefix: RELEASES.to_owned(),
            platform: PLATFORM.to_owned(),
            public_key: self.key.public_key(),
            download_folder: self.downloads.path().to_owned(),
            installer_arguments: vec!["/P".into(), "/R".into(), "/UPDATE".into()],
        }
    }

    /// Starts the app on this profile.
    fn start(&self) -> Core {
        let ports = Ports {
            update_downloader: Box::new(self.releases.clone()),
            process_starter: Box::new(self.starter.clone()),
            ..stand_ins::ports()
        };
        let store = Store::open(&self.data.path().join("nicelibrary.db")).unwrap();
        Core::new(ports, store, CoverCache::new(self.data.path().join("covers"))).with_update_channel(self.channel())
    }

    /// Publishes `version`, its Update Package signed with the project's updater key.
    fn publish(&self, version: &str) {
        self.releases.publish(version, PACKAGE, &self.key.sign(PACKAGE, version));
    }
}

fn available(version: &str) -> UpdateCheck {
    UpdateCheck::Available { update: Update { version: version.to_owned(), notes: format!("What is new in {version}") } }
}

#[test]
fn a_newer_version_on_github_releases_is_offered() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();

    assert_eq!(core.check_for_update().unwrap(), available(NEWER));
    assert_eq!(profile.releases.asked(), [MANIFEST]);
}

#[test]
fn the_version_that_runs_and_older_ones_are_up_to_date() {
    let profile = Profile::new();
    let core = profile.start();
    let running = core.about().version;

    for version in [running.as_str(), "0.0.1"] {
        profile.publish(version);
        assert_eq!(core.check_for_update().unwrap(), UpdateCheck::UpToDate, "{version}");
    }
}

#[test]
fn a_version_tag_with_a_leading_v_is_read_as_the_version() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let manifest = String::from_utf8(profile.releases.files.lock().unwrap()[MANIFEST].clone()).unwrap();
    profile.releases.serve(MANIFEST, manifest.replace("\"99.0.0\"", "\"v99.0.0\"").as_bytes());

    assert_eq!(profile.start().check_for_update().unwrap(), available(NEWER));
}

#[test]
fn before_the_first_release_the_app_is_up_to_date() {
    let profile = Profile::new();

    assert_eq!(profile.start().check_for_update().unwrap(), UpdateCheck::UpToDate);
}

#[test]
fn a_check_without_a_connection_fails() {
    let profile = Profile::new();
    profile.publish(NEWER);
    profile.releases.go_offline();

    assert!(profile.start().check_for_update().is_err());
}

#[test]
fn a_failed_check_keeps_the_update_offered_before() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();
    core.check_for_update().unwrap();

    profile.releases.go_offline();
    assert!(core.check_for_update().is_err());

    *profile.releases.offline.lock().unwrap() = false;
    assert_eq!(core.install_update(NEWER).unwrap(), UpdateInstall::Started);
}

#[test]
fn a_check_that_finds_no_newer_version_withdraws_the_update_offered_before() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();
    core.check_for_update().unwrap();

    profile.publish("0.0.1");
    assert_eq!(core.check_for_update().unwrap(), UpdateCheck::UpToDate);

    assert_eq!(core.install_update(NEWER).unwrap(), UpdateInstall::NotOffered);
}

#[test]
fn a_manifest_that_is_not_one_fails() {
    let profile = Profile::new();
    profile.releases.serve(MANIFEST, b"<html>sign in to this network</html>");

    assert!(profile.start().check_for_update().is_err());
}

#[test]
fn a_package_outside_the_projects_releases_is_refused() {
    let profile = Profile::new();
    let manifest = serde_json::json!({
        "version": NEWER,
        "platforms": { PLATFORM: {
            "signature": profile.key.sign(PACKAGE, NEWER),
            "url": "https://example.com/NiceLibrary_99.0.0_x64-setup.exe",
        } },
    });
    profile.releases.serve(MANIFEST, manifest.to_string().as_bytes());

    assert!(profile.start().check_for_update().is_err());
}

// Installing

#[test]
fn an_update_the_user_agreed_to_is_verified_and_its_package_started_to_install_over_the_app() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();
    core.check_for_update().unwrap();

    assert_eq!(core.install_update(NEWER).unwrap(), UpdateInstall::Started);

    let started = profile.starter.started();
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].contents, PACKAGE);
    assert_eq!(started[0].arguments, ["/P", "/R", "/UPDATE"]);
    assert!(started[0].executable.starts_with(profile.downloads.path()));
    assert_eq!(started[0].executable.extension().unwrap(), "exe");
}

#[test]
fn checking_never_installs_anything() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();

    core.check_for_update().unwrap();
    core.check_for_updates_automatically(|_| ControlFlow::Break(()));

    assert_eq!(profile.starter.started(), []);
    assert!(!profile.releases.asked().contains(&package_url(NEWER)));
}

#[test]
fn a_version_no_check_offered_is_not_installed() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();

    assert_eq!(core.install_update(NEWER).unwrap(), UpdateInstall::NotOffered);

    core.check_for_update().unwrap();
    assert_eq!(core.install_update("98.0.0").unwrap(), UpdateInstall::NotOffered);
    assert_eq!(profile.starter.started(), []);
}

/// Installs the Update `version` after a check offered it, its package
/// `package` signed with `signature`; answers the outcome and whether anything started.
fn install_signed(profile: &Profile, package: &[u8], signature: &str) -> (UpdateInstall, bool) {
    profile.releases.publish(NEWER, package, signature);
    let core = profile.start();
    assert_eq!(core.check_for_update().unwrap(), available(NEWER));
    let outcome = core.install_update(NEWER).unwrap();
    (outcome, !profile.starter.started().is_empty())
}

#[test]
fn a_package_signed_with_another_key_is_refused() {
    let profile = Profile::new();
    let signature = UpdaterKey::new().sign(PACKAGE, NEWER);

    assert_eq!(install_signed(&profile, PACKAGE, &signature), (UpdateInstall::InvalidSignature, false));
}

#[test]
fn a_package_changed_after_it_was_signed_is_refused() {
    let profile = Profile::new();
    let signature = profile.key.sign(PACKAGE, NEWER);

    assert_eq!(
        install_signed(&profile, b"MZ something else entirely", &signature),
        (UpdateInstall::InvalidSignature, false)
    );
}

#[test]
fn a_package_signed_for_another_version_is_refused() {
    let profile = Profile::new();
    let signature = profile.key.sign(PACKAGE, "0.0.9");

    assert_eq!(install_signed(&profile, PACKAGE, &signature), (UpdateInstall::InvalidSignature, false));
}

#[test]
fn a_package_signed_without_a_version_is_refused() {
    let profile = Profile::new();
    let signature = profile.key.sign_with_comment(PACKAGE, "timestamp:1795608000\tfile:setup.exe");

    assert_eq!(install_signed(&profile, PACKAGE, &signature), (UpdateInstall::InvalidSignature, false));
}

#[test]
fn a_signature_that_is_not_one_is_refused() {
    let profile = Profile::new();

    for signature in ["", "not base64 at all!", &STANDARD.encode("untrusted comment: nothing\n")] {
        assert_eq!(install_signed(&profile, PACKAGE, signature), (UpdateInstall::InvalidSignature, false));
    }
}

#[test]
fn a_package_that_cannot_be_downloaded_installs_nothing() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();
    core.check_for_update().unwrap();

    profile.releases.go_offline();

    assert!(core.install_update(NEWER).is_err());
    assert_eq!(profile.starter.started(), []);
}

// The setting

#[test]
fn the_automatic_check_is_on_until_the_user_turns_it_off() {
    let profile = Profile::new();
    let core = profile.start();

    assert!(core.checks_for_updates().unwrap());

    core.set_check_for_updates(false).unwrap();
    assert!(!core.checks_for_updates().unwrap());
    assert!(!profile.start().checks_for_updates().unwrap(), "kept after a restart");
}

/// The events of the first `rounds` rounds of the automatic Update Check.
fn automatic_rounds(core: &Core, rounds: usize) -> Vec<AutomaticUpdateCheck> {
    let mut events = Vec::new();
    core.check_for_updates_automatically(|event| {
        events.push(event);
        if events.len() == rounds { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
    });
    events
}

#[test]
fn the_automatic_check_offers_a_newer_version_for_the_interface_to_ask_about() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();

    let update = Update { version: NEWER.to_owned(), notes: format!("What is new in {NEWER}") };
    assert_eq!(automatic_rounds(&core, 1), [AutomaticUpdateCheck::Found { update }]);
    assert_eq!(profile.starter.started(), []);
    assert_eq!(core.install_update(NEWER).unwrap(), UpdateInstall::Started);
}

#[test]
fn with_the_setting_off_the_app_makes_no_update_request() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();
    core.set_check_for_updates(false).unwrap();

    assert_eq!(automatic_rounds(&core, 3), vec![AutomaticUpdateCheck::Skipped; 3]);
    assert_eq!(profile.releases.asked(), Vec::<String>::new());
}

#[test]
fn with_the_setting_off_the_user_can_still_check_by_hand() {
    let profile = Profile::new();
    profile.publish(NEWER);
    let core = profile.start();
    core.set_check_for_updates(false).unwrap();

    assert_eq!(core.check_for_update().unwrap(), available(NEWER));
}

#[test]
fn an_automatic_check_that_fails_says_so_and_the_next_round_tries_again() {
    let profile = Profile::new();
    profile.releases.go_offline();
    let core = profile.start();

    let events = automatic_rounds(&core, 2);

    assert!(events.iter().all(|event| matches!(event, AutomaticUpdateCheck::Failed { .. })), "{events:?}");
    assert_eq!(profile.releases.asked(), [MANIFEST, MANIFEST]);
}

#[test]
fn a_core_without_an_update_channel_asks_nothing() {
    let releases = Releases::default();
    let data = tempfile::tempdir().unwrap();
    let ports = Ports { update_downloader: Box::new(releases.clone()), ..stand_ins::ports() };
    let store = Store::open(&data.path().join("nicelibrary.db")).unwrap();
    let core = Core::new(ports, store, CoverCache::new(data.path().join("covers")));

    assert!(core.check_for_update().is_err());
    assert_eq!(releases.asked(), Vec::<String>::new());
}
