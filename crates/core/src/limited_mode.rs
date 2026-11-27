//! Limited Mode: why the app cannot read the Steam Account's data from
//! Steam, as the interface names it to the user.

use serde::Serialize;

use crate::sync::SyncOutcome;

/// The page of Steam where the signed-in user sets who sees their profile
/// and their game details. Fixed here; the interface never passes a URL.
pub(crate) const PRIVACY_SETTINGS_PAGE: &str = "https://steamcommunity.com/my/edit/settings";

/// Whether the app runs in Limited Mode, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum LimitedMode {
    /// Nothing keeps the app from reading the Steam Account's data.
    Off,
    /// No Steam Web API key is stored.
    NoKey,
    /// Steam refused the stored key at the last Sync.
    KeyRejected,
    /// Steam did not reveal the account's games at the last Sync: its game
    /// details are private.
    GameDetailsPrivate,
    /// Steam could not be asked or gave no usable answer at the last Sync.
    SteamUnreachable,
}

impl LimitedMode {
    /// The reason a Sync that ended with `outcome` leaves behind: `Some(Off)`
    /// after a successful one, `None` when it says nothing about Steam's
    /// answer for the Steam Account.
    pub(crate) fn after_sync(outcome: SyncOutcome) -> Option<Self> {
        match outcome {
            SyncOutcome::Synced { .. } => Some(Self::Off),
            SyncOutcome::KeyRejected => Some(Self::KeyRejected),
            SyncOutcome::GameDetailsPrivate => Some(Self::GameDetailsPrivate),
            SyncOutcome::SteamUnreachable => Some(Self::SteamUnreachable),
            SyncOutcome::NoKey | SyncOutcome::NoSteamAccount => None,
        }
    }

    /// The name the store keeps a reason under; `None` for `Off`, which is
    /// not kept.
    pub(crate) fn stored_name(self) -> Option<&'static str> {
        match self {
            Self::Off | Self::NoKey => None,
            Self::KeyRejected => Some("keyRejected"),
            Self::GameDetailsPrivate => Some("gameDetailsPrivate"),
            Self::SteamUnreachable => Some("steamUnreachable"),
        }
    }

    /// The reason kept under `name`; a name this version does not know
    /// reads as `Off`, so the next Sync decides.
    pub(crate) fn from_stored_name(name: &str) -> Self {
        [Self::KeyRejected, Self::GameDetailsPrivate, Self::SteamUnreachable]
            .into_iter()
            .find(|reason| reason.stored_name() == Some(name))
            .unwrap_or(Self::Off)
    }
}
