//! Steam Source, Web API: the user's own Steam Web API key (ADR-0002) and
//! the requests made with it.
//!
//! The key is personal and confidential. It lives only in the operating
//! system's secret store, never in the database, a file or an error message.

use serde::Serialize;

/// Port: the operating system's store for secrets.
///
/// Contract: `read` answers what the last `write` under the same name stored,
/// also after a restart, or `None` when nothing is stored under it. `write`
/// replaces what was stored under the name. `delete` removes it and is `Ok`
/// when nothing was stored. `Err` carries a reason for a log or a developer
/// and never contains a secret.
pub trait SecretStore: Send + Sync {
    fn read(&self, name: &str) -> Result<Option<String>, String>;
    fn write(&self, name: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;
}

/// Port: sends a GET request to a web API.
///
/// Contract: `Ok` with the status and body of every answer the server gives,
/// whatever its status; `Err` with a reason only when no answer arrives: no
/// network, a timeout, a failed TLS handshake. The URL may carry a secret, so
/// the port never writes it to a log.
pub trait WebApi: Send + Sync {
    fn get(&self, url: &str) -> Result<WebAnswer, String>;
}

/// The Steam Web API gave no answer: no network, a timeout, a failed TLS
/// handshake. It carries no reason on purpose, since the port's reason may
/// repeat the URL and with it the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Unanswered;

/// Asks the Steam Web API `method`, such as `IPlayerService/GetOwnedGames/v1`,
/// with `key`, when one is needed, and the query `params` in their order.
/// Every request to the Steam Web API is built here, so the URL, and with it
/// the key, never reaches an error text: a request that is not answered is
/// only [`Unanswered`].
pub(crate) fn steam_get(
    api: &dyn WebApi,
    method: &str,
    key: Option<&str>,
    params: &[(&str, &str)],
) -> Result<WebAnswer, Unanswered> {
    let mut url = format!("https://api.steampowered.com/{method}/?");
    let key = key.map(|key| ("key", key));
    for (index, (name, value)) in key.iter().chain(params).enumerate() {
        if index > 0 {
            url.push('&');
        }
        url.push_str(name);
        url.push('=');
        url.push_str(value);
    }
    api.get(&url).map_err(|_| Unanswered)
}

/// A web server's answer to a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebAnswer {
    /// The HTTP status code.
    pub status: u16,
    pub body: Vec<u8>,
}

/// How storing a Steam Web API key went, when nothing failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum SteamWebApiKeyCheck {
    /// Steam accepted the key, and it is stored in place of any earlier one.
    Stored,
    /// The key is not a Steam Web API key, or Steam does not accept it.
    /// Nothing changed.
    Invalid,
    /// Steam could not be asked or gave no usable answer, so it is unknown
    /// whether the key is valid. Nothing changed.
    SteamUnreachable,
}

/// The name the key is kept under in the secret store.
pub(crate) const KEY_SECRET: &str = "steam-web-api-key";

/// The Steam page where a user creates their key.
pub(crate) const KEY_PAGE: &str = "https://steamcommunity.com/dev/apikey";

/// The key in `text`, if it has the form of a Steam Web API key: 32
/// hexadecimal digits, written in capitals as Steam issues them, without
/// surrounding spaces. Nothing else may reach a URL.
pub(crate) fn key_in(text: &str) -> Option<String> {
    let text = text.trim();
    (text.len() == 32 && text.bytes().all(|b| b.is_ascii_hexdigit())).then(|| text.to_ascii_uppercase())
}

/// Whether Steam accepts `key`, asked with the smallest request that needs a
/// key: the first entry of Steam's app list.
pub(crate) fn check_key(api: &dyn WebApi, key: &str) -> SteamWebApiKeyCheck {
    let Ok(answer) = steam_get(api, "IStoreService/GetAppList/v1", Some(key), &[("max_results", "1")]) else {
        return SteamWebApiKeyCheck::SteamUnreachable;
    };
    match answer.status {
        // Steam's answer to a key it does not know.
        401 | 403 => SteamWebApiKeyCheck::Invalid,
        // Something in between, such as a network's sign-in page, may answer
        // in Steam's place; only Steam's own JSON confirms the key.
        200 if is_steam_response(&answer.body) => SteamWebApiKeyCheck::Stored,
        _ => SteamWebApiKeyCheck::SteamUnreachable,
    }
}

/// Whether `body` is the JSON Steam's services answer with: `{"response": {...}}`.
fn is_steam_response(body: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(body).is_ok_and(|value| value.get("response").is_some_and(|r| r.is_object()))
}
