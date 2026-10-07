//! One series or movie with its description, episodes or releases, and files.

use dioxus::prelude::*;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use yokoku_domain::{ImportId, MovieId, SeriesId};

use super::{FileStatus, Status};
#[cfg(feature = "server")]
use crate::api::{Dep, Library, Prober, Ratings, Reviewer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Numbering {
    Standard,
    Absolute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Release {
    Cinema,
    Digital,
    Physical,
}

/// Image URLs; `None` for an image the item doesn't have.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Images {
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub logo: Option<String>,
}

/// What the item is about; empty until its metadata is refreshed.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Description {
    pub overview: String,
    pub genres: Vec<String>,
    /// Minutes: a movie's length, or a series' usual episode length.
    pub runtime: Option<u16>,
}

/// An item's rating at one source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemRating {
    /// `IMDb`.
    pub source: String,
    /// On the source's scale, like `7.8` of 10.
    pub value: f32,
    pub votes: Option<u32>,
    /// The item's page at the source.
    pub url: Option<String>,
}

/// Files a scan found in the item's folder that wait to be matched; when several scans left
/// imports, `import` is the first, and the next one opens once it is done.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Unrecognised {
    pub files: usize,
    /// The scan's import that holds them.
    pub import: ImportId,
}

/// A library file and what a probe read from it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    /// Bytes.
    pub size: u64,
    /// `None` until the file is probed.
    pub streams: Option<Streams>,
    /// Subtitle files beside the video, like `en (SDH)`.
    pub subtitle_files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Streams {
    pub minutes: Option<u64>,
    pub video: Option<Video>,
    /// Like `eng aac 5.1`, in file order.
    pub audio: Vec<String>,
    /// Subtitles inside the file, like `eng (forced)`.
    pub subtitles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Video {
    pub width: u32,
    pub height: u32,
    pub codec: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeriesDetail {
    pub id: SeriesId,
    pub title: String,
    pub original_title: String,
    pub year: Option<i16>,
    /// `tmdb:209867`.
    pub source: String,
    /// The item's page at its metadata source.
    pub source_url: String,
    /// Today in the server's time zone.
    pub today: Date,
    pub status: Status,
    pub numbering: Numbering,
    pub monitored: bool,
    pub images: Images,
    pub description: Description,
    pub ratings: Vec<ItemRating>,
    pub next: Option<EpisodeRow>,
    pub last: Option<EpisodeRow>,
    pub seasons: Vec<SeasonDetail>,
    /// Files a scan found in the series' folder that wait to be matched.
    pub unrecognised: Option<Unrecognised>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeasonDetail {
    pub number: u16,
    pub monitored: bool,
    pub episodes: Vec<EpisodeRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EpisodeRow {
    pub season: u16,
    pub number: u16,
    pub title: String,
    /// Empty when the source has none.
    pub overview: String,
    pub air_date: Option<Date>,
    pub file: FileStatus,
    /// The episode's file, when it has one.
    pub file_info: Option<FileInfo>,
    /// Set when the Jellyfin user has played the episode's file.
    pub watched: Option<Watched>,
    pub monitored: bool,
}

/// A file the Jellyfin user has played.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watched {
    /// When it was last played, in the server's time zone; `None` when Jellyfin gives no date.
    pub on: Option<Date>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MovieDetail {
    pub id: MovieId,
    pub title: String,
    pub original_title: String,
    pub year: Option<i16>,
    /// `tmdb:438631`.
    pub source: String,
    /// The item's page at its metadata source.
    pub source_url: String,
    /// Today in the server's time zone.
    pub today: Date,
    pub status: Status,
    pub monitored: bool,
    pub images: Images,
    pub description: Description,
    pub ratings: Vec<ItemRating>,
    /// Every release kind, in order, with its date when known.
    pub releases: Vec<(Release, Option<Date>)>,
    pub file: FileStatus,
    /// The movie's file, when it has one.
    pub file_info: Option<FileInfo>,
    /// Set when the Jellyfin user has played the movie's file.
    pub watched: Option<Watched>,
    /// Files a scan found in the movie's folder that wait to be matched.
    pub unrecognised: Option<Unrecognised>,
}

impl Release {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cinema => "Cinema",
            Self::Digital => "Digital",
            Self::Physical => "Physical",
        }
    }
}

/// An item's library files and their size, subtitles left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiskUsage {
    pub files: usize,
    /// Bytes.
    pub size: u64,
}

impl SeriesDetail {
    /// A file holding several episodes counts once.
    pub fn disk_usage(&self) -> DiskUsage {
        let files: std::collections::HashMap<&str, u64> = self
            .seasons
            .iter()
            .flat_map(|season| &season.episodes)
            .filter_map(|episode| episode.file_info.as_ref().map(|info| (info.path.as_str(), info.size)))
            .collect();
        DiskUsage { files: files.len(), size: files.values().sum() }
    }

    /// Episodes the Jellyfin user has played, and episodes downloaded.
    pub fn watch_progress(&self) -> (usize, usize) {
        let episodes = self.seasons.iter().flat_map(|season| &season.episodes);
        let downloaded = episodes.clone().filter(|episode| episode.file == FileStatus::Downloaded).count();
        (episodes.filter(|episode| episode.watched.is_some()).count(), downloaded)
    }
}

impl SeasonDetail {
    /// `Season 1`, or `Specials` for season 0.
    pub fn name(&self) -> String {
        match self.number {
            0 => "Specials".to_owned(),
            number => format!("Season {number}"),
        }
    }
}

/// `None` when the library has no such series.
#[get("/api/series/{id}", library: Dep<Library>, prober: Dep<Prober>, reviewer: Dep<Reviewer>, ratings: Dep<Ratings>)]
pub async fn series(id: SeriesId) -> Result<Option<SeriesDetail>, ServerFnError> {
    server::series(&library, &prober, &reviewer, &ratings, id).await
}

/// `None` when the library has no such movie.
#[get("/api/movies/{id}", library: Dep<Library>, prober: Dep<Prober>, reviewer: Dep<Reviewer>, ratings: Dep<Ratings>)]
pub async fn movie(id: MovieId) -> Result<Option<MovieDetail>, ServerFnError> {
    server::movie(&library, &prober, &reviewer, &ratings, id).await
}

#[cfg(feature = "server")]
mod server {
    use std::collections::HashMap;

    use dioxus::{logger::tracing::error, prelude::*};
    use jiff::civil::Date;
    use yokoku_core::{
        library::{LibraryError, LibraryStatus, artwork_name},
        media::{FileDetails, MediaInfo},
    };
    use yokoku_domain::{
        Artwork, ArtworkKind, Episode, EpisodeRef, ExternalId, ImdbId, ItemFolder, ItemId, MediaFileId, MediaKind,
        Movie, MovieId, Rating, RatingSource, ReleaseKind, Series, SeriesId,
    };

    use super::{
        Description, EpisodeRow, FileInfo, Images, ItemRating, Library, MovieDetail, Numbering, Prober, Ratings,
        Release, Reviewer, SeasonDetail, SeriesDetail, Streams, Unrecognised, Video, Watched,
    };
    use crate::api::artwork;

    type Files = HashMap<MediaFileId, FileInfo>;
    type WatchedFiles = HashMap<MediaFileId, Watched>;

    pub(super) async fn series(
        library: &Library,
        prober: &Prober,
        reviewer: &Reviewer,
        ratings: &Ratings,
        id: SeriesId,
    ) -> Result<Option<SeriesDetail>, ServerFnError> {
        let today = library.today();
        let series = match library.series(id).await {
            Ok(series) => series,
            Err(LibraryError::SeriesNotFound(_)) => return Ok(None),
            Err(error) => {
                error!(%error, series = %id, "loading the series failed");
                return Err(ServerFnError::new("The series could not be loaded"));
            },
        };
        let files = files(prober, ItemId::Series(id)).await;
        let watched = watched(library).await;
        let unrecognised = unrecognised(reviewer, &series.folder).await;
        let ratings = item_ratings(ratings, ItemId::Series(id), series.external_ids.imdb.as_ref()).await;
        Ok(Some(SeriesDetail::new(&series, &files, &watched, ratings, unrecognised, today)))
    }

    pub(super) async fn movie(
        library: &Library,
        prober: &Prober,
        reviewer: &Reviewer,
        ratings: &Ratings,
        id: MovieId,
    ) -> Result<Option<MovieDetail>, ServerFnError> {
        let today = library.today();
        let movie = match library.movie(id).await {
            Ok(movie) => movie,
            Err(LibraryError::MovieNotFound(_)) => return Ok(None),
            Err(error) => {
                error!(%error, movie = %id, "loading the movie failed");
                return Err(ServerFnError::new("The movie could not be loaded"));
            },
        };
        let files = files(prober, ItemId::Movie(id)).await;
        let watched = watched(library).await;
        let unrecognised = unrecognised(reviewer, &movie.folder).await;
        let ratings = item_ratings(ratings, ItemId::Movie(id), movie.external_ids.imdb.as_ref()).await;
        Ok(Some(MovieDetail::new(&movie, &files, &watched, ratings, unrecognised, today)))
    }

    /// Files of scans of `folder` waiting for review; `None` when there are none or imports
    /// cannot be read.
    async fn unrecognised(reviewer: &Reviewer, folder: &ItemFolder) -> Option<Unrecognised> {
        let imports = reviewer.pending().await.unwrap_or_else(|error| {
            error!(%error, folder = %folder.path().display(), "reading the imports failed");
            Vec::new()
        });
        let scans: Vec<_> =
            imports.iter().filter(|import| import.download.is_none() && import.source == folder.path()).collect();
        let files = scans.iter().map(|import| import.rows.len()).sum();
        let import = scans.first()?.id;
        (files > 0).then_some(Unrecognised { files, import })
    }

    /// The item's files by id; empty when they cannot be read.
    async fn files(prober: &Prober, item: ItemId) -> Files {
        match prober.details(item).await {
            Ok(details) => details.into_iter().map(|details| (details.file.id, FileInfo::from(details))).collect(),
            Err(error) => {
                error!(%error, ?item, "reading the item's files failed");
                Files::new()
            },
        }
    }

    /// The item's stored ratings; empty when they cannot be read.
    async fn item_ratings(ratings: &Ratings, item: ItemId, imdb_id: Option<&ImdbId>) -> Vec<ItemRating> {
        match ratings.of(item).await {
            Ok(stored) => stored.into_iter().map(|rating| ItemRating::new(rating, imdb_id)).collect(),
            Err(error) => {
                error!(%error, ?item, "reading the item's ratings failed");
                Vec::new()
            },
        }
    }

    /// Played library files; empty when they cannot be read.
    async fn watched(library: &Library) -> WatchedFiles {
        match library.watched_files().await {
            Ok(watched) => {
                watched.into_iter().map(|(file, at)| (file, Watched { on: at.map(|at| library.date_of(at)) })).collect()
            },
            Err(error) => {
                error!(%error, "reading the watched files failed");
                WatchedFiles::new()
            },
        }
    }

    impl SeriesDetail {
        fn new(
            series: &Series,
            files: &Files,
            watched: &WatchedFiles,
            ratings: Vec<ItemRating>,
            unrecognised: Option<Unrecognised>,
            today: Date,
        ) -> Self {
            let row = |(reference, episode)| EpisodeRow::new(reference, episode, files, watched, today);
            Self {
                id: series.id,
                title: series.title.clone(),
                original_title: series.original_title.clone(),
                year: series.year,
                source: series.source.to_string(),
                source_url: source_url(series.source, MediaKind::Series),
                today,
                status: LibraryStatus::Series(series.status(today)).into(),
                numbering: series.numbering.into(),
                monitored: series.monitored,
                images: Images::new(ItemId::Series(series.id), &series.artwork),
                description: series.description.clone().into(),
                ratings,
                next: series.next_episode(today).map(row),
                last: series.last_aired(today).map(row),
                unrecognised,
                seasons: series
                    .seasons
                    .iter()
                    .map(|season| SeasonDetail {
                        number: season.number,
                        monitored: season.monitored,
                        episodes: season
                            .episodes
                            .iter()
                            .map(|episode| {
                                row((EpisodeRef { season: season.number, episode: episode.number }, episode))
                            })
                            .collect(),
                    })
                    .collect(),
            }
        }
    }

    impl EpisodeRow {
        fn new(reference: EpisodeRef, episode: &Episode, files: &Files, watched: &WatchedFiles, today: Date) -> Self {
            Self {
                season: reference.season,
                number: reference.episode,
                title: episode.title.clone(),
                overview: episode.overview.clone(),
                air_date: episode.air_date,
                file: episode.file_status(today).into(),
                file_info: episode.file.and_then(|file| files.get(&file)).cloned(),
                watched: episode.file.and_then(|file| watched.get(&file)).copied(),
                monitored: episode.monitored,
            }
        }
    }

    impl MovieDetail {
        fn new(
            movie: &Movie,
            files: &Files,
            watched: &WatchedFiles,
            ratings: Vec<ItemRating>,
            unrecognised: Option<Unrecognised>,
            today: Date,
        ) -> Self {
            let releases = &movie.releases;
            Self {
                id: movie.id,
                title: movie.title.clone(),
                original_title: movie.original_title.clone(),
                year: movie.year,
                source: movie.source.to_string(),
                source_url: source_url(movie.source, MediaKind::Movie),
                today,
                status: LibraryStatus::Movie(movie.status(today)).into(),
                monitored: movie.monitored,
                images: Images::new(ItemId::Movie(movie.id), &movie.artwork),
                description: movie.description.clone().into(),
                ratings,
                releases: vec![
                    (Release::Cinema, releases.cinema),
                    (Release::Digital, releases.digital),
                    (Release::Physical, releases.physical),
                ],
                file: movie.file_status(today).into(),
                file_info: movie.file.and_then(|file| files.get(&file)).cloned(),
                watched: movie.file.and_then(|file| watched.get(&file)).copied(),
                unrecognised,
            }
        }
    }

    impl Images {
        pub(crate) fn new(item: ItemId, artwork: &Artwork) -> Self {
            let url = |kind| artwork.get(kind).and_then(artwork_name).map(|name| artwork::url(item, kind, name));
            Self {
                poster: url(ArtworkKind::Poster),
                backdrop: url(ArtworkKind::Backdrop),
                logo: url(ArtworkKind::Logo),
            }
        }
    }

    fn source_url(source: ExternalId, kind: MediaKind) -> String {
        match (source, kind) {
            (ExternalId::Tmdb(id), MediaKind::Series) => format!("https://www.themoviedb.org/tv/{id}"),
            (ExternalId::Tmdb(id), MediaKind::Movie) => format!("https://www.themoviedb.org/movie/{id}"),
            (ExternalId::Tvdb(id), MediaKind::Series) => format!("https://thetvdb.com/dereferrer/series/{id}"),
            (ExternalId::Tvdb(id), MediaKind::Movie) => format!("https://thetvdb.com/dereferrer/movie/{id}"),
        }
    }

    impl ItemRating {
        fn new(rating: Rating, imdb_id: Option<&ImdbId>) -> Self {
            match rating.source {
                RatingSource::Imdb => Self {
                    source: "IMDb".to_owned(),
                    value: rating.value,
                    votes: rating.votes,
                    url: imdb_id.map(|id| format!("https://www.imdb.com/title/{id}/")),
                },
            }
        }
    }

    impl From<yokoku_domain::Description> for Description {
        fn from(description: yokoku_domain::Description) -> Self {
            Self { overview: description.overview, genres: description.genres, runtime: description.runtime }
        }
    }

    impl From<FileDetails> for FileInfo {
        fn from(details: FileDetails) -> Self {
            Self {
                path: details.file.path.display().to_string(),
                size: details.file.size,
                streams: details.info.map(Streams::from),
                subtitle_files: details.subtitle_files.iter().map(ToString::to_string).collect(),
            }
        }
    }

    impl From<MediaInfo> for Streams {
        fn from(info: MediaInfo) -> Self {
            Self {
                minutes: info.duration.map(|duration| duration.as_secs() / 60),
                video: info.video.map(|video| Video { width: video.width, height: video.height, codec: video.codec }),
                audio: info.audio.iter().map(ToString::to_string).collect(),
                subtitles: info.subtitles.iter().map(ToString::to_string).collect(),
            }
        }
    }

    impl From<yokoku_domain::Numbering> for Numbering {
        fn from(numbering: yokoku_domain::Numbering) -> Self {
            match numbering {
                yokoku_domain::Numbering::Standard => Self::Standard,
                yokoku_domain::Numbering::Absolute => Self::Absolute,
            }
        }
    }

    impl From<ReleaseKind> for Release {
        fn from(kind: ReleaseKind) -> Self {
            match kind {
                ReleaseKind::Cinema => Self::Cinema,
                ReleaseKind::Digital => Self::Digital,
                ReleaseKind::Physical => Self::Physical,
            }
        }
    }
}
