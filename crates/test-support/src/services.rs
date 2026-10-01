//! The Jellyfin and Transmission that `just services` runs for the integration tests, as the dev shell
//! describes them.

use std::path::PathBuf;

/// The value of the dev shell's `name`; panics outside it.
pub fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is unset; run the integration tests in the dev shell"))
}

/// Where the services keep their state.
pub fn dir() -> PathBuf {
    env("YOKOKU_TEST_SERVICES_DIR").into()
}

pub fn jellyfin_url() -> String {
    env("YOKOKU_TEST_JELLYFIN_URL")
}

pub fn jellyfin_api_key() -> String {
    env("YOKOKU_TEST_JELLYFIN_API_KEY")
}

/// Jellyfin's TV and movie library folders for test files: `tv` and `movies`.
pub fn jellyfin_media() -> PathBuf {
    dir().join("jellyfin/test-media")
}

pub fn transmission_url() -> String {
    env("YOKOKU_TEST_TRANSMISSION_URL")
}

/// Where Transmission saves torrents.
pub fn transmission_downloads() -> PathBuf {
    dir().join("transmission/downloads")
}
