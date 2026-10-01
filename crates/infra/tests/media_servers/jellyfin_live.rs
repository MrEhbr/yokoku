use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use jiff::{Timestamp, civil::date};
use serde_json::{Value, json};
use tokio::time::sleep;
use yokoku_core::{
    integrations::{
        WatchSync,
        ports::{MediaServer, Played, PlayedItem, Watched, WatchedStore},
    },
    library::ports::{MovieRepo, SeriesRepo},
    media::{
        MediaFile,
        ports::{Changes, MediaRepo},
    },
};
use yokoku_domain::{
    EpisodeSpan, ExternalId, FileTarget, ItemFolder, Live, MediaFileId, MonitorPreset, Movie, Releases, Secret, Series,
    SeriesMetadata, SourceStatus,
};
use yokoku_infra::{
    db::Database,
    media_servers::{JellyfinClient, JellyfinSettings},
};
use yokoku_test_support::{
    metadata::{movie_metadata, series_metadata},
    services,
};

const WAIT: Duration = Duration::from_secs(60);
const SEVERANCE: ExternalId = ExternalId::Tvdb(371980);
const ARRIVAL: ExternalId = ExternalId::Tmdb(329865);

/// Jellyfin's API with the dev key, for what the client under test does not do.
struct Admin {
    http: reqwest::Client,
    url: String,
    authorization: String,
}

impl Admin {
    fn new() -> Self {
        let authorization = format!("MediaBrowser Token=\"{}\"", services::jellyfin_api_key());
        Self { http: reqwest::Client::new(), url: services::jellyfin_url(), authorization }
    }

    async fn send(&self, method: reqwest::Method, path: &str, body: Option<Value>) -> Value {
        let mut request =
            self.http.request(method, format!("{}{path}", self.url)).header("Authorization", &self.authorization);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.unwrap().error_for_status().unwrap();
        response.json().await.unwrap_or(Value::Null)
    }

    /// The new user's id; `jellyfin-setup` removes users named `test-…` when the services start.
    async fn create_user(&self, name: &str) -> String {
        let user = self.send(reqwest::Method::POST, "/Users/New", Some(json!({ "Name": name }))).await;
        user["Id"].as_str().unwrap().to_owned()
    }

    async fn refresh(&self) {
        self.send(reqwest::Method::POST, "/Library/Refresh", None).await;
    }

    /// Each of `paths`' item id, once Jellyfin lists every one with its episode numbers.
    async fn item_ids(&self, user: &str, paths: &[&Path]) -> Vec<String> {
        let start = Instant::now();
        loop {
            let query = format!("/Items?userId={user}&recursive=true&includeItemTypes=Episode,Movie&fields=Path");
            let items = self.send(reqwest::Method::GET, &query, None).await;
            let placed = |path: &&Path| {
                items["Items"].as_array().unwrap().iter().find(|item| {
                    item["Path"].as_str() == path.to_str()
                        && (item["Type"] == "Movie" || item["IndexNumber"].is_number())
                })
            };
            let found: Vec<String> =
                paths.iter().filter_map(placed).map(|item| item["Id"].as_str().unwrap().to_owned()).collect();
            if found.len() == paths.len() {
                return found;
            }
            assert!(start.elapsed() < WAIT, "Jellyfin did not list {paths:?}");
            sleep(Duration::from_millis(500)).await;
        }
    }

    async fn mark_played(&self, user: &str, item: &str) {
        self.send(reqwest::Method::POST, &format!("/UserPlayedItems/{item}?userId={user}"), None).await;
    }
}

/// Empty files in Jellyfin's test library folders, removed on drop.
struct TestMedia {
    folders: Vec<PathBuf>,
}

impl TestMedia {
    /// A series and a movie titled `title`, tagged with Severance's and Arrival's ids.
    fn new(title: &str) -> (Self, [PathBuf; 3]) {
        let root = services::jellyfin_media();
        let series = root.join(format!("tv/{title} (2022) [tvdbid-371980]"));
        let movie = root.join(format!("movies/{title} (2016) [tmdbid-329865]"));
        let files = [
            series.join(format!("Season 01/{title} (2022) - S01E01.mkv")),
            series.join(format!("Season 01/{title} (2022) - S01E02-E03.mkv")),
            movie.join(format!("{title} (2016).mkv")),
        ];
        for file in &files {
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"").unwrap();
        }
        (Self { folders: vec![series, movie] }, files)
    }
}

impl Drop for TestMedia {
    fn drop(&mut self) {
        for folder in &self.folders {
            _ = fs::remove_dir_all(folder);
        }
    }
}

fn client(user: &str) -> JellyfinClient {
    JellyfinClient::new(Live::fixed(JellyfinSettings {
        url: Some(services::jellyfin_url()),
        api_key: Some(Secret::new(services::jellyfin_api_key())),
        user: Some(user.into()),
    }))
}

/// "Severance" from TVDB and "Arrival" from TMDB, with library files `e01` at Jellyfin's path and
/// `e02_e03` and `arrival` elsewhere.
async fn library(jellyfin_e01: &Path) -> (Database, [MediaFile; 3]) {
    let db = Database::open_in_memory().await.unwrap();
    let aired = [Some(date(2022, 2, 18)); 3];
    let metadata = SeriesMetadata {
        source: SEVERANCE,
        ..series_metadata(371980, "Severance", SourceStatus::Returning, &[(1, &aired)])
    };
    let mut severance = Series::new(
        metadata,
        ItemFolder::new("/library/series".into(), "Severance (2022)".into()).unwrap(),
        MonitorPreset::All,
        date(2026, 10, 2),
        Timestamp::UNIX_EPOCH,
    );
    let mut arrival = Movie::new(
        movie_metadata(329865, "Arrival", Releases::default()),
        ItemFolder::new("/library/movies".into(), "Arrival (2016)".into()).unwrap(),
        true,
        Timestamp::UNIX_EPOCH,
    );
    SeriesRepo::save(&db, &mut severance).await.unwrap();
    MovieRepo::save(&db, &mut arrival).await.unwrap();
    let file = |path: &Path, target| MediaFile {
        id: MediaFileId::generate(),
        path: path.into(),
        size: 0,
        target,
        added_at: Timestamp::UNIX_EPOCH,
    };
    let episodes =
        |first, last| FileTarget::Episodes { series: severance.id, span: EpisodeSpan::new(1, first, last).unwrap() };
    let files = [
        file(jellyfin_e01, episodes(1, 1)),
        file(Path::new("/library/series/Severance (2022)/Season 01/S01E02-E03.mkv"), episodes(2, 3)),
        file(Path::new("/library/movies/Arrival (2016)/Arrival (2016).mkv"), FileTarget::Movie(arrival.id)),
    ];
    MediaRepo::save(&db, &Changes { added_files: files.to_vec(), ..Changes::default() }).await.unwrap();
    (db, files)
}

#[tokio::test]
#[ignore = "needs the services from `just services`"]
async fn played_items_become_watched_library_files() {
    let id = MediaFileId::generate().0.simple().to_string();
    let title = format!("Live {}", &id[id.len() - 8..]);
    let admin = Admin::new();
    let user = format!("test-{id}");
    let user_id = admin.create_user(&user).await;
    let (_media, [e01, e02_e03, movie]) = TestMedia::new(&title);
    admin.refresh().await;
    let items = admin.item_ids(&user_id, &[&e01, &e02_e03, &movie]).await;
    admin.mark_played(&user_id, &items[0]).await;
    admin.mark_played(&user_id, &items[2]).await;
    let client = Arc::new(client(&user));

    let mut played = client.played().await.unwrap();
    let (db, [library_e01, _, library_movie]) = library(&e01).await;
    let repo = Arc::new(db.clone());
    WatchSync::new(client, repo.clone(), repo.clone(), repo).sync().await.unwrap();

    played.sort_by(|a, b| a.path.cmp(&b.path));
    let paths: Vec<&Path> = played.iter().map(|played| played.path.as_path()).collect();
    assert_eq!(paths, [movie.as_path(), e01.as_path()]);
    assert!(played.iter().all(|played| played.at.is_some()), "{played:?}");
    let ids = |played: &Played| match &played.item {
        Some(PlayedItem::Movie(ids)) => ids.clone(),
        Some(PlayedItem::Episodes { series, span }) if *span == EpisodeSpan::new(1, 1, 1).unwrap() => series.clone(),
        other => panic!("unexpected {other:?}"),
    };
    assert!(ids(&played[0]).contains(&ARRIVAL), "{played:?}");
    assert!(ids(&played[1]).contains(&SEVERANCE), "{played:?}");
    let watched: Vec<MediaFileId> =
        db.watched().await.unwrap().into_iter().map(|watched: Watched| watched.file).collect();
    assert_eq!(watched.len(), 2, "{watched:?}");
    assert!(watched.contains(&library_e01.id) && watched.contains(&library_movie.id), "{watched:?}");
}
