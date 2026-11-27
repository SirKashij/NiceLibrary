//! HTTP(S): downloads Covers and Achievement icons from Steam's image hosts and asks the Steam Web API.

use std::time::Duration;

use nicelibrary_core::{Downloader, WebAnswer, WebApi};
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
        Self { agent: agent() }
    }
}

/// An HTTP client with the operating system's TLS and its trusted root
/// certificates, which hands every status to the caller.
fn agent() -> Agent {
    let tls = TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build();
    Agent::config_builder()
        .tls_config(tls)
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(30)))
        // The status is answered by the caller, not turned into an error.
        .http_status_as_error(false)
        .build()
        .into()
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

/// Asks web APIs, the Steam Web API among them, with the operating system's
/// TLS. A URL may carry the Steam Web API key, so it is never logged or
/// repeated in an error.
pub struct HttpWebApi {
    agent: Agent,
}

impl HttpWebApi {
    pub fn new() -> Self {
        Self { agent: agent() }
    }
}

impl Default for HttpWebApi {
    fn default() -> Self {
        Self::new()
    }
}

impl WebApi for HttpWebApi {
    fn get(&self, url: &str) -> Result<WebAnswer, String> {
        let mut response = self.agent.get(url).call().map_err(|e| without_url(&e))?;
        let status = response.status().as_u16();
        let body = response.body_mut().with_config().limit(MAX_SIZE).read_to_vec().map_err(|e| without_url(&e))?;
        Ok(WebAnswer { status, body })
    }
}

/// What went wrong, in words that never include the URL; some of ureq's own
/// messages repeat it.
fn without_url(error: &ureq::Error) -> String {
    match error {
        ureq::Error::Timeout(_) => "the server did not answer in time".into(),
        ureq::Error::Io(e) => format!("network error: {}", e.kind()),
        ureq::Error::HostNotFound => "the server's name could not be resolved".into(),
        ureq::Error::ConnectionFailed => "no connection to the server".into(),
        ureq::Error::Tls(_) => "the secure connection failed".into(),
        ureq::Error::BodyExceedsLimit(_) => "the answer is too large".into(),
        _ => "the request failed".into(),
    }
}
