use std::io::Write;

use flate2::{Compression, write::GzEncoder};
use uuid::Uuid;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};
use yokoku_core::integrations::ports::{RatedItem, RatingsError, RatingsProvider};
use yokoku_domain::{ExternalId, ExternalIds, ItemId, Live, MovieId, Rating, RatingSource, SeriesId};
use yokoku_infra::ratings::{ImdbDataset, RatingsSettings};

const DATASET: &str = "tconst\taverageRating\tnumVotes
tt0000001\t5.7\t2231
tt0903747\t9.5\t2400000
tt1160419\t8.0\t950000
tt9999999\tnot-a-number\t10
tt22248376\t9.3
";

fn gzipped(text: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(text.as_bytes()).unwrap();
    encoder.finish().unwrap()
}

fn dataset(server: &MockServer, folder: &tempfile::TempDir) -> ImdbDataset {
    let settings = RatingsSettings { imdb_url: format!("{}/title.ratings.tsv.gz", server.uri()) };
    ImdbDataset::new(Live::fixed(settings), folder.path().join("ratings"))
}

fn series(n: u128, imdb_id: Option<&str>) -> RatedItem {
    RatedItem {
        item: ItemId::Series(SeriesId(Uuid::from_u128(n))),
        source: ExternalId::Tvdb(n as u64),
        external_ids: ExternalIds { imdb: imdb_id.map(|id| id.parse().unwrap()) },
    }
}

fn movie(n: u128, imdb_id: Option<&str>) -> RatedItem {
    RatedItem {
        item: ItemId::Movie(MovieId(Uuid::from_u128(n))),
        source: ExternalId::Tmdb(n as u64),
        external_ids: ExternalIds { imdb: imdb_id.map(|id| id.parse().unwrap()) },
    }
}

fn imdb(value: f32, votes: u32) -> Rating {
    Rating { source: RatingSource::Imdb, value, votes: Some(votes) }
}

async fn serving(body: Vec<u8>) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/title.ratings.tsv.gz"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body).insert_header("etag", "\"v1\""))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn items_get_the_ratings_of_their_imdb_ids() {
    let server = serving(gzipped(DATASET)).await;
    let folder = tempfile::tempdir().unwrap();
    let items = [
        series(1, Some("tt0903747")),
        movie(2, Some("tt1160419")),
        movie(3, Some("tt7777777")),
        movie(4, None),
        series(5, Some("tt9999999")),
        series(6, Some("tt22248376")),
    ];

    let ratings = dataset(&server, &folder).ratings(&items).await.unwrap();

    assert_eq!(ratings, [(items[0].item, imdb(9.5, 2_400_000)), (items[1].item, imdb(8.0, 950_000))]);
}

#[tokio::test]
async fn nothing_is_downloaded_while_no_item_has_an_imdb_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200)).expect(0).mount(&server).await;
    let folder = tempfile::tempdir().unwrap();

    let ratings = dataset(&server, &folder).ratings(&[movie(1, None)]).await.unwrap();

    assert_eq!(ratings, []);
}

#[tokio::test]
async fn an_unchanged_dataset_is_read_from_the_stored_copy() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(gzipped(DATASET)).insert_header("etag", "\"v1\""))
        .expect(1)
        .mount(&server)
        .await;
    let folder = tempfile::tempdir().unwrap();
    let items = [movie(1, Some("tt1160419"))];

    dataset(&server, &folder).ratings(&items).await.unwrap();
    let again = dataset(&server, &folder).ratings(&items).await.unwrap();

    assert_eq!(again, [(items[0].item, imdb(8.0, 950_000))]);
}

#[tokio::test]
async fn a_dataset_without_its_header_is_invalid() {
    let server = serving(gzipped("tt0903747\t9.5\t2400000\n")).await;
    let folder = tempfile::tempdir().unwrap();

    let error = dataset(&server, &folder).ratings(&[movie(1, Some("tt0903747"))]).await.unwrap_err();

    assert!(matches!(error, RatingsError::Invalid(_)), "{error:?}");
}

#[tokio::test]
async fn a_failed_download_leaves_ratings_unavailable() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(503)).mount(&server).await;
    let folder = tempfile::tempdir().unwrap();

    let error = dataset(&server, &folder).ratings(&[movie(1, Some("tt0903747"))]).await.unwrap_err();

    assert!(matches!(error, RatingsError::Unavailable(_)), "{error:?}");
}
