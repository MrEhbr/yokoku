use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use jiff::{Timestamp, civil::date};
use yokoku_core::{
    integrations::{
        WatchSync,
        ports::{MediaServer, MediaServerError, Played, PlayedItem, Watched, WatchedStore},
    },
    library::ports::{MovieRepo, SeriesRepo},
    media::{
        MediaFile,
        ports::{Changes, MediaRepo},
    },
};
use yokoku_domain::{
    EpisodeSpan, ExternalId, FileTarget, ItemFolder, MediaFileId, MonitorPreset, Movie, Releases, Series, SourceStatus,
};
use yokoku_infra::db::Database;
use yokoku_test_support::metadata::{movie_metadata, series_metadata};

const FRIEREN: ExternalId = ExternalId::Tmdb(209867);
const DUNE: ExternalId = ExternalId::Tmdb(438631);

/// Answers with `played`, or as unconfigured or unavailable.
struct FakeServer {
    played: Mutex<Result<Vec<Played>, MediaServerError>>,
}

#[async_trait]
impl MediaServer for FakeServer {
    async fn version(&self) -> Result<String, MediaServerError> {
        Ok("Jellyfin 10.10.7".into())
    }

    async fn refresh_library(&self) -> Result<(), MediaServerError> {
        Ok(())
    }

    async fn played(&self) -> Result<Vec<Played>, MediaServerError> {
        match &*self.played.lock().unwrap() {
            Ok(played) => Ok(played.clone()),
            Err(MediaServerError::NotConfigured) => Err(MediaServerError::NotConfigured),
            Err(_) => Err(MediaServerError::Unavailable("connection refused".into())),
        }
    }
}

/// "Frieren" with files `S01E01.mkv` and `S01E02-E03.mkv`, and "Dune" with `Dune.mkv`.
struct Setup {
    db: Database,
    server: Arc<FakeServer>,
    sync: WatchSync,
    e01: MediaFile,
    e02_e03: MediaFile,
    dune: MediaFile,
}

async fn setup() -> Setup {
    let db = Database::open_in_memory().await.unwrap();
    let now = Timestamp::UNIX_EPOCH;
    let aired = [Some(date(2023, 9, 29)); 3];
    let mut frieren = Series::new(
        series_metadata(209867, "Frieren", SourceStatus::Returning, &[(1, &aired)]),
        ItemFolder::new("/tv".into(), "Frieren (2023)".into()).unwrap(),
        MonitorPreset::All,
        date(2023, 9, 29),
        now,
    );
    let mut dune = Movie::new(
        movie_metadata(438631, "Dune", Releases::default()),
        ItemFolder::new("/movies".into(), "Dune (2021)".into()).unwrap(),
        true,
        now,
    );
    SeriesRepo::save(&db, &mut frieren).await.unwrap();
    MovieRepo::save(&db, &mut dune).await.unwrap();
    let file = |path: &str, target| MediaFile {
        id: MediaFileId::generate(),
        path: path.into(),
        size: 1,
        target,
        added_at: now,
    };
    let episodes =
        |first, last| FileTarget::Episodes { series: frieren.id, span: EpisodeSpan::new(1, first, last).unwrap() };
    let e01 = file("/tv/Frieren (2023)/S01E01.mkv", episodes(1, 1));
    let e02_e03 = file("/tv/Frieren (2023)/S01E02-E03.mkv", episodes(2, 3));
    let dune = file("/movies/Dune (2021)/Dune.mkv", FileTarget::Movie(dune.id));
    let added_files = vec![e01.clone(), e02_e03.clone(), dune.clone()];
    MediaRepo::save(&db, &Changes { added_files, ..Changes::default() }).await.unwrap();

    let server = Arc::new(FakeServer { played: Mutex::new(Ok(Vec::new())) });
    let repo = Arc::new(db.clone());
    let sync = WatchSync::new(server.clone(), repo.clone(), repo.clone(), repo);
    Setup { db, server, sync, e01, e02_e03, dune }
}

impl Setup {
    async fn sync(&self, played: Result<Vec<Played>, MediaServerError>) -> Vec<Watched> {
        *self.server.played.lock().unwrap() = played;
        _ = self.sync.sync().await;
        self.watched().await
    }

    /// In the order `e01`, `e02_e03`, `dune`.
    async fn watched(&self) -> Vec<Watched> {
        let order = [self.e01.id, self.e02_e03.id, self.dune.id];
        let mut watched = self.db.watched().await.unwrap();
        watched.sort_by_key(|watched| order.iter().position(|&id| id == watched.file));
        watched
    }
}

fn at(second: i64) -> Option<Timestamp> {
    Some(Timestamp::from_second(1_790_000_000 + second).unwrap())
}

fn played(path: &str, item: Option<PlayedItem>, at: Option<Timestamp>) -> Played {
    Played { path: PathBuf::from(path), item, at }
}

fn episodes(series: &[ExternalId], first: u16, last: u16) -> Option<PlayedItem> {
    Some(PlayedItem::Episodes { series: series.to_vec(), span: EpisodeSpan::new(1, first, last).unwrap() })
}

fn watched(file: &MediaFile, at: Option<Timestamp>) -> Watched {
    Watched { file: file.id, at }
}

#[tokio::test]
async fn a_file_played_at_its_library_path_is_watched() {
    let setup = setup().await;

    let watched = setup.sync(Ok(vec![played("/tv/Frieren (2023)/S01E01.mkv", None, at(1))])).await;

    assert_eq!(watched, [self::watched(&setup.e01, at(1))]);
}

#[tokio::test]
async fn files_elsewhere_are_matched_by_their_series_or_movie_id() {
    let setup = setup().await;
    let series = [ExternalId::Tvdb(424536), FRIEREN];

    let watched = setup
        .sync(Ok(vec![
            played("/data/tv/Frieren/S01E01.mkv", episodes(&series, 1, 1), at(1)),
            played("/data/movies/Dune.mkv", Some(PlayedItem::Movie(vec![DUNE])), None),
        ]))
        .await;

    assert_eq!(watched, [self::watched(&setup.e01, at(1)), self::watched(&setup.dune, None)]);
}

#[tokio::test]
async fn a_file_of_several_episodes_is_watched_once_each_is_played() {
    let setup = setup().await;
    let e02 = played("/data/S01E02.mkv", episodes(&[FRIEREN], 2, 2), at(5));
    let e03 = played("/data/S01E03.mkv", episodes(&[FRIEREN], 3, 3), at(3));

    let partly = setup.sync(Ok(vec![e02.clone()])).await;
    let fully = setup.sync(Ok(vec![e02, e03])).await;

    assert_eq!(partly, []);
    assert_eq!(fully, [watched(&setup.e02_e03, at(5))]);
}

#[tokio::test]
async fn a_played_file_of_several_episodes_marks_each_episode_file() {
    let setup = setup().await;

    let watched = setup.sync(Ok(vec![played("/data/S01E01-E03.mkv", episodes(&[FRIEREN], 1, 3), at(2))])).await;

    assert_eq!(watched, [self::watched(&setup.e01, at(2)), self::watched(&setup.e02_e03, at(2))]);
}

#[tokio::test]
async fn a_file_no_longer_played_is_no_longer_watched() {
    let setup = setup().await;
    setup.sync(Ok(vec![played("/movies/Dune (2021)/Dune.mkv", None, at(1))])).await;

    let watched = setup.sync(Ok(Vec::new())).await;

    assert_eq!(watched, []);
}

#[tokio::test]
async fn watched_files_stay_while_the_server_is_unconfigured_or_unavailable() {
    let setup = setup().await;
    let before = setup.sync(Ok(vec![played("/movies/Dune (2021)/Dune.mkv", None, at(1))])).await;

    let unconfigured = setup.sync(Err(MediaServerError::NotConfigured)).await;
    *setup.server.played.lock().unwrap() = Err(MediaServerError::Refused("down".into()));
    let failed = setup.sync.sync().await;

    assert_eq!(unconfigured, before);
    assert!(failed.is_err());
    assert_eq!(setup.watched().await, before);
}
