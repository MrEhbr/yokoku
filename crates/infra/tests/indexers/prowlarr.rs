use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};
use yokoku_core::downloads::ports::{Indexer, ReleaseQuery, TorrentSource};
use yokoku_domain::{Live, MediaKind, Secret, TrackerSet, Trackers};
use yokoku_infra::indexers::{IndexerProvider, JackettSettings, ProwlarrClient, ProwlarrSettings, SelectedIndexer};

const KEY: &str = "prowlarr-key";

fn client(server: &MockServer) -> ProwlarrClient {
    ProwlarrClient::new(Live::fixed(ProwlarrSettings { url: Some(server.uri()), api_key: Some(Secret::new(KEY)) }))
}

fn query(trackers: Trackers) -> ReleaseQuery {
    ReleaseQuery { text: "Dune".into(), kind: Some(MediaKind::Movie), season: None, episode: None, trackers }
}

async fn indexers(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/api/v1/indexer"))
        .and(header("X-Api-Key", KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"id": 1, "name": "Rutor", "enable": true, "supportsSearch": true, "protocol": "torrent"},
            {"id": 2, "name": "Disabled", "enable": false, "supportsSearch": true, "protocol": "torrent"},
            {"id": 3, "name": "Usenet", "enable": true, "supportsSearch": true, "protocol": "usenet"}
        ])))
        .mount(server)
        .await;
}

async fn search_answer(server: &MockServer, link: &str) {
    Mock::given(method("GET"))
        .and(path("/api/v1/indexer/1/newznab"))
        .and(header("X-Api-Key", KEY))
        .and(query_param("t", "caps"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"<caps><server title="Rutor"/><searching><search available="yes" supportedParams="q"/><movie-search available="yes" supportedParams="q"/></searching></caps>"#,
            "application/xml",
        ))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/indexer/1/newznab"))
        .and(header("X-Api-Key", KEY))
        .and(query_param("t", "movie"))
        .and(query_param("q", "Dune"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            format!(
                r#"<rss xmlns:torznab="http://torznab.com/schemas/2015/feed"><channel><item><title>Dune 2021</title><enclosure url="{}" type="application/x-bittorrent"/><torznab:attr name="seeders" value="12"/></item></channel></rss>"#,
                link.replace('&', "&amp;")
            ),
            "application/xml",
        ))
        .mount(server)
        .await;
}

#[tokio::test]
async fn searches_enabled_torrent_indexers_and_fetches_via_prowlarr() {
    let server = MockServer::start().await;
    indexers(&server).await;
    let link = format!("{}/api/v1/indexer/1/download?link=test&file=Dune", server.uri());
    search_answer(&server, &link).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/indexer/1/download"))
        .and(header("X-Api-Key", KEY))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(b"d4:infod4:name4:Dunee".to_vec(), "application/x-bittorrent"),
        )
        .mount(&server)
        .await;

    let client = client(&server);
    assert_eq!(client.version().await.unwrap(), "Prowlarr");
    assert_eq!(client.trackers().await.unwrap().len(), 1);
    let found = client.search(&query(Trackers::All)).await.unwrap();
    assert_eq!(
        (found.releases[0].title.as_str(), found.releases[0].tracker.as_str(), found.releases[0].seeders),
        ("Dune 2021", "Rutor", Some(12))
    );
    assert_eq!(
        client.fetch(&found.releases[0].link).await.unwrap(),
        TorrentSource::File(b"d4:infod4:name4:Dunee".to_vec())
    );
}

#[tokio::test]
async fn selected_indexer_searches_only_prowlarr_when_both_are_configured() {
    let server = MockServer::start().await;
    indexers(&server).await;
    search_answer(&server, &format!("{}/api/v1/indexer/1/download?link=test&file=Dune", server.uri())).await;
    let indexer = SelectedIndexer::new(
        Live::fixed(JackettSettings { url: Some(server.uri()), api_key: Some(Secret::new("jackett-key")) }),
        Live::fixed(ProwlarrSettings { url: Some(server.uri()), api_key: Some(Secret::new(KEY)) }),
        Live::fixed(IndexerProvider::Prowlarr),
    );

    let trackers = indexer.trackers().await.unwrap();
    assert_eq!(trackers[0].id.as_str(), "prowlarr_1");
    let all = indexer.search(&query(Trackers::All)).await.unwrap();
    assert_eq!(all.releases.len(), 1);
    assert!(all.warnings.is_empty());
    assert_eq!(all.releases[0].tracker, "Rutor · Prowlarr");
    let selected = Trackers::Only(TrackerSet::new(["prowlarr_1".parse().unwrap()]).unwrap());
    let found = indexer.search(&query(selected)).await.unwrap();
    assert_eq!(found.releases.len(), 1);
    assert!(found.warnings.is_empty());
    assert!(found.releases[0].link.starts_with("p:"));
    let wrong = Trackers::Only(TrackerSet::new(["jackett_rutor".parse().unwrap()]).unwrap());
    assert!(indexer.search(&query(wrong)).await.is_err());
}

#[tokio::test]
async fn series_search_sends_season_to_indexer_that_supports_it() {
    let server = MockServer::start().await;
    indexers(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/indexer/1/newznab"))
        .and(query_param("t", "caps"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"<caps><searching><tv-search available="yes" supportedParams="q,season,ep"/></searching></caps>"#,
            "application/xml",
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/indexer/1/newznab"))
        .and(query_param("t", "tvsearch"))
        .and(query_param("q", "Show"))
        .and(query_param("season", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"<rss><channel><item><title>Show S02</title><link>magnet:?xt=urn:btih:123</link></item></channel></rss>"#,
            "application/xml",
        ))
        .mount(&server)
        .await;

    let query = ReleaseQuery {
        text: "Show".into(),
        kind: Some(MediaKind::Series),
        season: Some(2),
        episode: None,
        trackers: Trackers::All,
    };
    let found = client(&server).search(&query).await.unwrap();
    assert_eq!(found.releases[0].title, "Show S02");
}

#[tokio::test]
async fn rejects_download_links_outside_configured_prowlarr() {
    let server = MockServer::start().await;
    let error =
        client(&server).fetch("http://example.com/api/v1/indexer/1/download?link=test&file=Dune").await.unwrap_err();
    assert!(error.to_string().contains("does not lead to Prowlarr"));
}

#[tokio::test]
async fn rejects_external_download_redirects() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/indexer/1/download"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "https://tracker.example/download"))
        .mount(&server)
        .await;

    let link = format!("{}/api/v1/indexer/1/download?link=test&file=Dune", server.uri());
    let error = client(&server).fetch(&link).await.unwrap_err();
    assert!(error.to_string().contains("turn off Redirect"));
}
