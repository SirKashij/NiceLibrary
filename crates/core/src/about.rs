//! About NiceLibrary: which version runs, under which license, and where the
//! project page and the license are. The interface carries the privacy
//! notice, the Valve trademark notice and the non-affiliation statement.

use serde::{Deserialize, Serialize};

/// What the About dialog tells of the app itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct About {
    /// The version of the app that runs, such as `0.1.0`.
    pub version: String,
    /// The SPDX identifier of the app's license.
    pub license: String,
}

/// A page the About dialog links to, opened in the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AboutPage {
    /// The project page on GitHub.
    Project,
    /// The license text in the repository.
    License,
}

impl AboutPage {
    pub(crate) fn url(self) -> &'static str {
        match self {
            Self::Project => "https://github.com/SirKashij/NiceLibrary",
            Self::License => "https://github.com/SirKashij/NiceLibrary/blob/main/LICENSE",
        }
    }
}

/// The app's version and license, as its manifest states them; core and
/// shell share the workspace's version.
pub(crate) fn about() -> About {
    About { version: env!("CARGO_PKG_VERSION").to_owned(), license: env!("CARGO_PKG_LICENSE").to_owned() }
}
