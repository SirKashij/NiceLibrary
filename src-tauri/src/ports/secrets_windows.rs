//! Keeps secrets in the Windows Credential Manager, as generic credentials
//! of this computer that do not roam to other computers.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use nicelibrary_core::SecretStore;
use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, GetLastError};
use windows_sys::Win32::Security::Credentials::{
    CRED_MAX_CREDENTIAL_BLOB_SIZE, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW,
    CredFree, CredReadW, CredWriteW,
};

/// The prefix of the app's entries in the Credential Manager.
const APP_PREFIX: &str = "io.github.sirkashij.nicelibrary/";

/// The Credential Manager, holding each secret as `<prefix><name>`.
pub struct WindowsSecretStore {
    prefix: String,
}

impl WindowsSecretStore {
    /// The app's own secrets.
    pub fn new() -> Self {
        Self::under(APP_PREFIX)
    }

    /// The secrets whose entries start with `prefix`; for tests that must
    /// not touch the app's own entries.
    pub fn under(prefix: &str) -> Self {
        Self { prefix: prefix.to_owned() }
    }

    fn target(&self, name: &str) -> Vec<u16> {
        wide(&format!("{}{name}", self.prefix))
    }
}

impl Default for WindowsSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for WindowsSecretStore {
    fn read(&self, name: &str) -> Result<Option<String>, String> {
        let target = self.target(name);
        let mut credential: *mut CREDENTIALW = ptr::null_mut();
        // SAFETY: plain Win32 call; on success `credential` points to memory
        // Windows allocated, which is read once and freed below.
        unsafe {
            if CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) == 0 {
                return match GetLastError() {
                    ERROR_NOT_FOUND => Ok(None),
                    code => Err(format!("the Credential Manager could not be read (error {code})")),
                };
            }
            let blob = &*credential;
            let bytes = if blob.CredentialBlob.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(blob.CredentialBlob, blob.CredentialBlobSize as usize).to_vec()
            };
            CredFree(credential.cast());
            String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| "the Credential Manager holds something that is not text".to_owned())
        }
    }

    fn write(&self, name: &str, secret: &str) -> Result<(), String> {
        if secret.len() > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize {
            return Err("the secret is too long for the Credential Manager".into());
        }
        let mut target = self.target(name);
        let mut bytes = secret.as_bytes().to_vec();
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            CredentialBlobSize: bytes.len() as u32,
            CredentialBlob: bytes.as_mut_ptr(),
            // This computer only; a secret never roams with the profile.
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..CREDENTIALW::default()
        };
        // SAFETY: plain Win32 call; `credential` and the buffers it points to
        // outlive the call, and Windows copies what it keeps.
        let written = unsafe { CredWriteW(&credential, 0) != 0 };
        if written {
            Ok(())
        } else {
            // SAFETY: plain Win32 call.
            Err(format!("the Credential Manager refused the secret (error {})", unsafe { GetLastError() }))
        }
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        let target = self.target(name);
        // SAFETY: plain Win32 calls with a null-terminated name.
        unsafe {
            if CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) != 0 {
                return Ok(());
            }
            match GetLastError() {
                ERROR_NOT_FOUND => Ok(()),
                code => Err(format!("the Credential Manager could not delete the secret (error {code})")),
            }
        }
    }
}

/// `text` as a null-terminated UTF-16 string.
fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect()
}
