//! Unreadable Database: when the database cannot be opened, the store names
//! why (a newer version wrote it, it is damaged, or its folder cannot be
//! written) and where it lies, and leaves the file as it is. The app then
//! starts in a reduced state that can only say so, show the folder and quit.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nicelibrary_core::{
    Core, CoverCache, Database, Log, OpenError, Opener, Ports, Store, UnreadableCore, UnreadableReason, stand_ins,
};
use tempfile::TempDir;

/// A data folder of its own for each test.
struct Profile {
    folder: TempDir,
}

impl Profile {
    fn new() -> Self {
        Self { folder: tempfile::tempdir().unwrap() }
    }

    fn database(&self) -> PathBuf {
        self.folder.path().join("nicelibrary.db")
    }

    fn logs(&self) -> PathBuf {
        self.folder.path().join("logs")
    }

    fn log_text(&self) -> String {
        fs::read_to_string(self.logs().join("nicelibrary.log")).unwrap_or_default()
    }
}

/// Stands in for a database a later version of the app wrote: its schema
/// version is beyond what this version knows.
fn written_by_a_newer_version(database: &Path) {
    Store::open(database).unwrap();
    let connection = rusqlite::Connection::open(database).unwrap();
    connection.pragma_update(None, "user_version", 9999).unwrap();
}

#[test]
fn a_database_from_a_newer_version_is_unreadable_and_left_unchanged() {
    let profile = Profile::new();
    written_by_a_newer_version(&profile.database());
    let before = fs::read(profile.database()).unwrap();

    let unreadable = Store::open(&profile.database()).err().expect("refused");

    assert_eq!(unreadable.reason, UnreadableReason::NewerVersion);
    assert_eq!(unreadable.path, profile.database());
    assert_eq!(fs::read(profile.database()).unwrap(), before);
}

#[test]
fn a_file_that_is_not_a_database_is_damaged_and_left_unchanged() {
    let profile = Profile::new();
    let garbage = b"This is not a SQLite database, just some text that is long enough to look like a header.".repeat(20);
    fs::write(profile.database(), &garbage).unwrap();

    let unreadable = Store::open(&profile.database()).err().expect("refused");

    assert_eq!(unreadable.reason, UnreadableReason::Damaged);
    assert_eq!(unreadable.path, profile.database());
    assert_eq!(fs::read(profile.database()).unwrap(), garbage);
}

#[test]
fn a_data_folder_that_cannot_be_created_is_not_writable() {
    let profile = Profile::new();
    // A file where the data folder would have to be.
    let blocker = profile.folder.path().join("data");
    fs::write(&blocker, b"in the way").unwrap();
    let database = blocker.join("nicelibrary.db");

    let unreadable = Store::open(&database).err().expect("refused");

    assert_eq!(unreadable.reason, UnreadableReason::FolderNotWritable);
    assert_eq!(unreadable.path, database);
}

#[test]
fn a_database_that_cannot_be_written_is_not_writable_and_left_unchanged() {
    let profile = Profile::new();
    Store::open(&profile.database()).unwrap();
    let before = fs::read(profile.database()).unwrap();
    set_read_only(&profile.database(), true);

    let opened = Store::open(&profile.database());

    set_read_only(&profile.database(), false);
    let unreadable = opened.err().expect("refused");
    assert_eq!(unreadable.reason, UnreadableReason::FolderNotWritable);
    assert_eq!(fs::read(profile.database()).unwrap(), before);
}

fn set_read_only(file: &Path, read_only: bool) {
    let mut permissions = fs::metadata(file).unwrap().permissions();
    permissions.set_readonly(read_only);
    fs::set_permissions(file, permissions).unwrap();
}

#[test]
fn an_open_store_reports_the_database_as_open() {
    let profile = Profile::new();
    let store = Store::open(&profile.database()).unwrap();
    let core = Core::new(stand_ins::ports(), store, CoverCache::new(profile.folder.path().join("covers")));

    assert_eq!(core.database(), Database::Open);
}

/// Shows folders and remembers which.
#[derive(Clone, Default)]
struct FolderOpener {
    opened: Arc<Mutex<Vec<PathBuf>>>,
}

impl Opener for FolderOpener {
    fn open_url(&self, _url: &str) -> Result<(), OpenError> {
        Err(OpenError::Failed("no URLs in these tests".into()))
    }

    fn open_folder(&self, folder: &Path) -> Result<(), OpenError> {
        self.opened.lock().unwrap().push(folder.to_owned());
        Ok(())
    }
}

fn unreadable_core(profile: &Profile, opener: &FolderOpener, log: Log) -> UnreadableCore {
    written_by_a_newer_version(&profile.database());
    let unreadable = Store::open(&profile.database()).err().expect("refused");
    UnreadableCore::new(Ports { opener: Box::new(opener.clone()), ..stand_ins::ports() }, unreadable, log)
}

#[test]
fn the_unreadable_database_state_reports_the_reason_and_the_path() {
    let profile = Profile::new();
    let core = unreadable_core(&profile, &FolderOpener::default(), Log::new(profile.logs()));

    assert_eq!(
        core.database(),
        Database::Unreadable {
            reason: UnreadableReason::NewerVersion,
            path: profile.database().display().to_string()
        }
    );
}

#[test]
fn the_unreadable_database_state_shows_the_database_folder() {
    let profile = Profile::new();
    let opener = FolderOpener::default();
    let core = unreadable_core(&profile, &opener, Log::new(profile.logs()));

    core.open_database_folder().unwrap();

    assert_eq!(*opener.opened.lock().unwrap(), [profile.folder.path().to_owned()]);
}

#[test]
fn the_failed_open_is_written_to_the_log() {
    let profile = Profile::new();

    unreadable_core(&profile, &FolderOpener::default(), Log::new(profile.logs()));

    let log = profile.log_text();
    assert!(log.contains("ERROR"), "{log}");
    assert!(log.contains("newer version"), "{log}");
}

#[test]
fn a_log_that_cannot_be_written_does_not_stop_the_unreadable_database_state() {
    let profile = Profile::new();
    // A file where the log folder would have to be.
    fs::write(profile.logs(), b"in the way").unwrap();

    let core = unreadable_core(&profile, &FolderOpener::default(), Log::new(profile.logs()));

    assert!(matches!(core.database(), Database::Unreadable { .. }));
}
