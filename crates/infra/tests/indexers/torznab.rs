use std::sync::Arc;

use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};
use yokoku_core::downloads::ports::{Indexer, ReleaseQuery, TorrentSource};
use yokoku_domain::{Live, MediaKind, Secret, TrackerSet, Trackers};
use yokoku_infra::{
    db::Database,
    indexers::{CombinedIndexer, JackettSettings, TorznabClient, TorznabFeed, TorznabSettings},
};

const KEY: &str = "private-key";

fn feed(server: &MockServer, id: &str) -> TorznabFeed {
    TorznabFeed {
        id: id.parse().unwrap(),
        name: "Example".into(),
        url: format!("{}/api", server.uri()),
        api_key: Some(Secret::new(KEY)),
        enabled: true,
    }
}

fn query() -> ReleaseQuery {
    ReleaseQuery {
        text: "Dune".into(),
        kind: Some(MediaKind::Movie),
        season: None,
        episode: None,
        trackers: Trackers::All,
    }
}

async fn answer(server: &MockServer, torrent: &str) {
    Mock::given(method("GET")).and(path("/api")).and(query_param("t", "caps"))
        .and(query_param("apikey", KEY))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"<caps><server title="Example"/><searching><search available="yes" supportedParams="q"/><movie-search available="yes" supportedParams="q"/></searching></caps>"#,
            "application/xml"))
        .mount(server).await;
    Mock::given(method("GET")).and(path("/api")).and(query_param("t", "movie"))
        .and(query_param("q", "Dune")).and(query_param("apikey", KEY))
        .respond_with(ResponseTemplate::new(200).set_body_raw(format!(
            r#"<rss xmlns:torznab="http://torznab.com/schemas/2015/feed"><channel><item><title>Dune 2021</title><enclosure url="{torrent}" type="application/x-bittorrent"/><torznab:attr name="seeders" value="12"/></item></channel></rss>"#), "application/xml"))
        .mount(server).await;
}

#[tokio::test]
async fn direct_feed_searches_and_fetches_a_torrent() {
    let server = MockServer::start().await;
    let torrent = format!("{}/download?passkey=abc", server.uri());
    answer(&server, &torrent.replace('&', "&amp;")).await;
    Mock::given(path("/download"))
        .and(query_param("passkey", "abc"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(b"d4:infod4:name4:Dunee".to_vec(), "application/x-bittorrent"),
        )
        .mount(&server)
        .await;
    let client = TorznabClient::new();

    assert_eq!(client.test(&feed(&server, "example")).await.unwrap(), "Example");
    let found = client.search(&feed(&server, "example"), &query()).await.unwrap();
    assert_eq!(
        (found[0].title.as_str(), found[0].tracker.as_str(), found[0].seeders),
        ("Dune 2021", "Example", Some(12))
    );
    assert_eq!(client.fetch(&found[0].link).await.unwrap(), TorrentSource::File(b"d4:infod4:name4:Dunee".to_vec()));
}

#[tokio::test]
async fn configured_and_stored_feeds_are_combined_with_partial_failures() {
    let server = MockServer::start().await;
    answer(&server, &format!("{}/download", server.uri())).await;
    let db = Arc::new(Database::open_in_memory().await.unwrap());
    db.save_torznab_feed(&feed(&server, "stored")).await.unwrap();
    let bad = TorznabFeed { url: "http://127.0.0.1:9/api".into(), ..feed(&server, "bad") };
    let indexer = CombinedIndexer::new(
        Live::fixed(JackettSettings::default()),
        Live::fixed(TorznabSettings { feeds: vec![bad] }),
        db.clone(),
    );

    let sources = indexer.trackers().await.unwrap();
    let found = indexer.search(&query()).await.unwrap();

    assert_eq!(sources.len(), 2);
    assert_eq!(found.releases.len(), 1);
    assert_eq!(found.warnings.len(), 1);
    assert!(found.releases[0].link.starts_with("d:stored:"));
    assert_eq!(db.torznab_feeds().await.unwrap().len(), 1);

    let selected = ReleaseQuery {
        trackers: Trackers::Only(TrackerSet::new(["torznab_stored".parse().unwrap()]).unwrap()),
        ..query()
    };
    let found = indexer.search(&selected).await.unwrap();
    assert_eq!(found.releases.len(), 1);
    assert!(found.warnings.is_empty());
}

#[tokio::test]
async fn configured_feed_id_cannot_duplicate_a_stored_feed() {
    let server = MockServer::start().await;
    let db = Arc::new(Database::open_in_memory().await.unwrap());
    db.save_torznab_feed(&feed(&server, "shared")).await.unwrap();
    let indexer = CombinedIndexer::new(
        Live::fixed(JackettSettings::default()),
        Live::fixed(TorznabSettings { feeds: vec![feed(&server, "shared")] }),
        db,
    );

    let error = indexer.feeds().await.unwrap_err();

    assert!(error.to_string().contains("duplicate Torznab feed id: shared"));
}
