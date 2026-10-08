use reqwest::Url;
use serde::{Deserialize, Serialize};
use yokoku_domain::{Secret, TrackerId};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TorznabSettings {
    #[serde(default)]
    pub feeds: Vec<TorznabFeed>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TorznabFeed {
    pub id: TrackerId,
    pub name: String,
    /// Full Torznab API endpoint, ending in `/api` for most providers.
    pub url: String,
    pub api_key: Option<Secret>,
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn enabled() -> bool {
    true
}

impl TorznabFeed {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Give the feed a name".into());
        }
        let url = Url::parse(&self.url).map_err(|_| "Give a full Torznab API URL".to_owned())?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("Use an HTTP(S) Torznab API URL without credentials or query parameters".into());
        }
        Ok(())
    }
}
