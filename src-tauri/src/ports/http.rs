//! Downloads files over HTTP(S), for Covers from Steam's artwork host.

use std::time::Duration;

use nicelibrary_core::Downloader;
use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

/// The largest file accepted; a Cover is far smaller.
const MAX_SIZE: u64 = 16 * 1024 * 1024;

/// Fetches files with the operating system's TLS and its trusted root
/// certificates (schannel on Windows).
pub struct HttpDownloader {
    agent: Agent,
}

impl HttpDownloader {
    pub fn new() -> Self {
        let tls = TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build();
        let agent = Agent::config_builder()
            .tls_config(tls)
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(Duration::from_secs(30)))
            // The status is answered below, not turned into an error.
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

impl Default for HttpDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Downloader for HttpDownloader {
    fn download(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        let mut response = self.agent.get(url).call().map_err(|e| e.to_string())?;
        match response.status().as_u16() {
            200 => response
                .body_mut()
                .with_config()
                .limit(MAX_SIZE)
                .read_to_vec()
                .map(Some)
                .map_err(|e| e.to_string()),
            404 | 410 => Ok(None),
            status => Err(format!("the server answered with HTTP {status}")),
        }
    }
}
