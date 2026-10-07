use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    time::Duration,
};

use async_trait::async_trait;
use flate2::read::GzDecoder;
use reqwest::{StatusCode, header};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::debug;
use yokoku_core::integrations::ports::{RatedItem, RatingsError, RatingsProvider};
use yokoku_domain::{ItemId, Live, Rating, RatingSource};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT: Duration = Duration::from_secs(120);
const DATASET: &str = "title.ratings.tsv.gz";
const HEADER: &str = "tconst\taverageRating\tnumVotes";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RatingsSettings {
    /// IMDb's `title.ratings.tsv.gz` dataset.
    pub imdb_url: String,
}

impl Default for RatingsSettings {
    fn default() -> Self {
        Self { imdb_url: "https://datasets.imdbws.com/title.ratings.tsv.gz".into() }
    }
}

/// IMDb ratings from its daily dataset, downloaded into `folder` and fetched again only once it
/// changes.
pub struct ImdbDataset {
    http: reqwest::Client,
    settings: Live<RatingsSettings>,
    folder: PathBuf,
    /// Held while the dataset downloads.
    download: Mutex<()>,
}

impl ImdbDataset {
    pub fn new(settings: Live<RatingsSettings>, folder: PathBuf) -> Self {
        crate::tls::install_crypto_provider();
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TIMEOUT)
            .build()
            .expect("TLS backend initializes");
        Self { http, settings, folder, download: Mutex::new(()) }
    }

    /// The dataset's path once it is current: downloaded when changed since the stored copy.
    async fn current(&self) -> Result<PathBuf, RatingsError> {
        let _download = self.download.lock().await;
        let dataset = self.folder.join(DATASET);
        let etag_file = self.folder.join(format!("{DATASET}.etag"));
        let etag = match tokio::fs::try_exists(&dataset).await {
            Ok(true) => tokio::fs::read_to_string(&etag_file).await.ok(),
            _ => None,
        };

        let mut request = self.http.get(&self.settings.current().imdb_url);
        if let Some(etag) = &etag {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        let response = request.send().await.map_err(unavailable)?;
        match response.status() {
            StatusCode::NOT_MODIFIED if etag.is_some() => {
                debug!("IMDb dataset unchanged");
                return Ok(dataset);
            },
            status if !status.is_success() => {
                return Err(RatingsError::Unavailable(format!("IMDb answered {status}").into()));
            },
            _ => {},
        }
        let new_etag = response.headers().get(header::ETAG).and_then(|etag| etag.to_str().ok()).map(str::to_owned);
        let body = response.bytes().await.map_err(unavailable)?;

        tokio::fs::create_dir_all(&self.folder).await.map_err(unavailable)?;
        let partial = self.folder.join(format!("{DATASET}.partial"));
        tokio::fs::write(&partial, &body).await.map_err(unavailable)?;
        tokio::fs::rename(&partial, &dataset).await.map_err(unavailable)?;
        match new_etag {
            Some(etag) => tokio::fs::write(&etag_file, etag).await.map_err(unavailable)?,
            None => _ = tokio::fs::remove_file(&etag_file).await,
        }
        debug!(bytes = body.len(), "IMDb dataset downloaded");
        Ok(dataset)
    }
}

#[async_trait]
impl RatingsProvider for ImdbDataset {
    /// Looks items up by their IMDb id; asks nothing when none has one.
    async fn ratings(&self, items: &[RatedItem]) -> Result<Vec<(ItemId, Rating)>, RatingsError> {
        let mut wanted: HashMap<String, Vec<ItemId>> = HashMap::new();
        for rated in items {
            if let Some(imdb_id) = &rated.external_ids.imdb {
                wanted.entry(imdb_id.to_string()).or_default().push(rated.item);
            }
        }
        if wanted.is_empty() {
            return Ok(Vec::new());
        }
        let dataset = self.current().await?;
        tokio::task::spawn_blocking(move || read(&dataset, &wanted))
            .await
            .map_err(|error| RatingsError::Unavailable(error.into()))?
    }
}

/// The ratings of the `wanted` titles in the gzipped dataset at `path`; skips malformed lines.
fn read(path: &Path, wanted: &HashMap<String, Vec<ItemId>>) -> Result<Vec<(ItemId, Rating)>, RatingsError> {
    let file = File::open(path).map_err(unavailable)?;
    let mut lines = BufReader::new(GzDecoder::new(file)).lines();
    match lines.next() {
        Some(Ok(header)) if header == HEADER => {},
        Some(Err(error)) => return Err(RatingsError::Invalid(error.into())),
        _ => return Err(RatingsError::Invalid("the IMDb dataset does not start with its header".into())),
    }
    let mut ratings = Vec::new();
    for line in lines {
        let line = line.map_err(|error: io::Error| RatingsError::Invalid(error.into()))?;
        let mut fields = line.split('\t');
        let Some(items) = fields.next().and_then(|id| wanted.get(id)) else { continue };
        let (Some(Ok(value)), Some(Ok(votes))) =
            (fields.next().map(str::parse::<f32>), fields.next().map(str::parse::<u32>))
        else {
            continue;
        };
        let rating = Rating { source: RatingSource::Imdb, value, votes: Some(votes) };
        ratings.extend(items.iter().map(|&item| (item, rating)));
    }
    Ok(ratings)
}

fn unavailable(error: impl Error + Send + Sync + 'static) -> RatingsError {
    RatingsError::Unavailable(error.into())
}
