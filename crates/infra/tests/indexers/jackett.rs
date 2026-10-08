use jiff::Timestamp;
use rstest::rstest;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param, query_param_is_missing},
};
use yokoku_core::downloads::ports::{Indexer, IndexerError, Release, ReleaseQuery, TorrentSource, Tracker};
use yokoku_domain::{Live, MediaKind, Secret, TrackerId, TrackerSet, Trackers};
use yokoku_infra::indexers::{JackettClient, JackettSettings};

const TORZNAB: &str = "/api/v2.0/indexers/all/results/torznab/api";
const KEY: &str = "dev-key";
const MAGNET: &str = "magnet:?xt=urn:btih:c9e15763f722f23e98a29decdfae341b98d53056";

fn client(server: &MockServer) -> JackettClient {
    JackettClient::new(Live::fixed(JackettSettings { url: Some(server.uri()), api_key: Some(Secret::new(KEY)) }))
}

/// A Torznab feed holding `items`, as Jackett's aggregate feed answers.
fn feed(items: &str) -> ResponseTemplate {
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <atom:link href="http://127.0.0.1:9117/" rel="self" type="application/rss+xml" />
    <title>AggregateSearch</title>
    <description>This feed includes all configured trackers</description>
    <link>http://127.0.0.1/</link>
    <category>search</category>
    {items}
  </channel>
</rss>"#
    );
    ResponseTemplate::new(200).set_body_raw(body, "application/rss+xml")
}

/// A RuTracker release as Jackett lists it, downloaded through `server`.
fn rutracker_item(server: &MockServer) -> String {
    let link =
        format!("{}/dl/rutracker/?jackett_apikey={KEY}&amp;path=Q2ZESjhP&amp;file=The+Walking+Dead", server.uri());
    format!(
        r#"<item>
      <title>The Walking Dead / S1E1-6 of 6 [2010, WEB-DL 1080p] MVO + Original + Sub</title>
      <guid>https://rutracker.org/forum/viewtopic.php?t=3301430</guid>
      <jackettindexer id="rutracker">RuTracker.org</jackettindexer>
      <type>semi-private</type>
      <comments>https://rutracker.org/forum/viewtopic.php?t=3301430</comments>
      <pubDate>Sat, 03 Oct 2026 12:30:00 +0300</pubDate>
      <size>12884901888</size>
      <grabs>500</grabs>
      <description />
      <link>{link}</link>
      <category>5000</category>
      <category>5040</category>
      <enclosure url="{link}" length="12884901888" type="application/x-bittorrent" />
      <torznab:attr name="category" value="5000" />
      <torznab:attr name="seeders" value="42" />
      <torznab:attr name="peers" value="50" />
      <torznab:attr name="downloadvolumefactor" value="1" />
    </item>"#
    )
}

async fn answer(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path(TORZNAB))
        .and(query_param("apikey", KEY))
        .respond_with(response)
        .mount(server)
        .await;
}

fn series(text: &str, season: Option<u16>, episode: Option<u16>) -> ReleaseQuery {
    ReleaseQuery { text: text.into(), kind: Some(MediaKind::Series), season, episode, trackers: Trackers::All }
}

#[tokio::test]
async fn a_release_comes_with_its_tracker_numbers_and_a_link_without_the_api_key() {
    let server = MockServer::start().await;
    answer(&server, feed(&rutracker_item(&server))).await;

    let releases = client(&server).search(&series("The Walking Dead", None, None)).await.unwrap();

    assert_eq!(
        releases.releases,
        [Release {
            title: "The Walking Dead / S1E1-6 of 6 [2010, WEB-DL 1080p] MVO + Original + Sub".into(),
            tracker: "RuTracker.org".into(),
            size: 12_884_901_888,
            seeders: Some(42),
            leechers: Some(8),
            grabs: Some(500),
            published: Some("2026-10-03T09:30:00Z".parse::<Timestamp>().unwrap()),
            link: format!("{}/dl/rutracker/?path=Q2ZESjhP&file=The+Walking+Dead", server.uri()),
            details: Some("https://rutracker.org/forum/viewtopic.php?t=3301430".into()),
        }]
    );
}

#[rstest]
#[case::episode(series("Frieren", Some(1), Some(5)), &[("t", "tvsearch"), ("season", "1"), ("ep", "5")], &["cat"])]
#[case::season(series("Frieren", Some(1), None), &[("t", "tvsearch"), ("season", "1")], &["ep"])]
#[case::episode_without_season(series("Frieren", None, Some(5)), &[("t", "tvsearch")], &["season", "ep"])]
#[case::movie(
    ReleaseQuery {
        text: "Dune".into(),
        kind: Some(MediaKind::Movie),
        season: Some(1),
        episode: None,
        trackers: Trackers::All,
    },
    &[("t", "movie")],
    &["season", "cat"],
)]
#[case::anything(
    ReleaseQuery { text: "Dune".into(), kind: None, season: None, episode: None, trackers: Trackers::All },
    &[("t", "search")],
    &["cat"],
)]
#[tokio::test]
async fn the_query_picks_the_torznab_search(
    #[case] query: ReleaseQuery,
    #[case] sent: &[(&str, &str)],
    #[case] left_out: &[&str],
) {
    let server = MockServer::start().await;
    let mut mock = Mock::given(path(TORZNAB)).and(query_param("q", query.text.as_str()));
    for (name, value) in sent {
        mock = mock.and(query_param(*name, *value));
    }
    for name in left_out {
        mock = mock.and(query_param_is_missing(*name));
    }
    mock.respond_with(feed("")).expect(1).mount(&server).await;

    let releases = client(&server).search(&query).await.unwrap();

    assert!(releases.releases.is_empty());
}

#[tokio::test]
async fn a_magnet_only_release_keeps_its_magnet_and_one_without_a_link_is_left_out() {
    let server = MockServer::start().await;
    let items = format!(
        r#"<item><title>Dune 2021 1080p</title><torznab:attr name="magneturl" value="{}" /></item>
           <item><title>No link</title></item>"#,
        MAGNET.replace('&', "&amp;")
    );
    answer(&server, feed(&items)).await;

    let releases = client(&server).search(&series("Dune", None, None)).await.unwrap();

    let found: Vec<(&str, &str, Option<u32>)> = releases
        .releases
        .iter()
        .map(|release| (release.title.as_str(), release.link.as_str(), release.seeders))
        .collect();
    assert_eq!(found, [("Dune 2021 1080p", MAGNET, None)]);
}

#[tokio::test]
async fn an_error_answer_is_refused_with_its_description() {
    let server = MockServer::start().await;
    let error = r#"<?xml version="1.0" encoding="UTF-8"?><error code="100" description="Invalid API Key" />"#;
    answer(&server, ResponseTemplate::new(200).set_body_raw(error, "application/xml")).await;

    let error = client(&server).search(&series("Dune", None, None)).await.unwrap_err();

    assert!(matches!(&error, IndexerError::Refused(reason) if reason == "Invalid API Key"), "{error}");
}

#[tokio::test]
async fn a_search_no_tracker_supports_says_so() {
    let server = MockServer::start().await;
    let error = r#"<?xml version="1.0" encoding="UTF-8"?><error code="201" description="all does not support the requested query. Please check the capabilities (t=caps) and make sure the search mode and parameters are supported." />"#;
    answer(&server, ResponseTemplate::new(200).set_body_raw(error, "application/xml")).await;

    let error = client(&server).search(&series("Dune", None, None)).await.unwrap_err();

    assert!(
        matches!(&error, IndexerError::Refused(reason) if reason == "no tracker added in Jackett supports TV searches"),
        "{error}"
    );
}

#[tokio::test]
async fn the_version_is_the_server_title_from_its_caps() {
    let server = MockServer::start().await;
    let caps = r#"<?xml version="1.0" encoding="UTF-8"?><caps><server title="Jackett" /><limits default="1000" max="1000" /></caps>"#;
    Mock::given(path(TORZNAB))
        .and(query_param("t", "caps"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(caps, "application/xml"))
        .mount(&server)
        .await;

    assert_eq!(client(&server).version().await.unwrap(), "Jackett");
}

#[tokio::test]
async fn nothing_is_searched_without_an_address() {
    let client = JackettClient::new(Live::fixed(JackettSettings::default()));

    let error = client.search(&series("Dune", None, None)).await.unwrap_err();

    assert!(matches!(error, IndexerError::NotConfigured), "{error}");
}

#[tokio::test]
async fn a_download_link_gets_the_api_key_back_and_gives_the_torrent_file() {
    let server = MockServer::start().await;
    let torrent = b"d8:announce3:url4:infod4:name4:Duneee".to_vec();
    Mock::given(path("/dl/rutracker/"))
        .and(query_param("path", "Q2ZESjhP"))
        .and(query_param("jackett_apikey", KEY))
        .respond_with(ResponseTemplate::new(200).set_body_raw(torrent.clone(), "application/x-bittorrent"))
        .mount(&server)
        .await;

    let fetched = client(&server).fetch(&format!("{}/dl/rutracker/?path=Q2ZESjhP", server.uri())).await.unwrap();

    assert_eq!(fetched, TorrentSource::File(torrent));
}

#[tokio::test]
async fn a_download_link_that_redirects_to_a_magnet_gives_the_magnet() {
    let server = MockServer::start().await;
    Mock::given(path("/dl/rutracker/"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", MAGNET))
        .mount(&server)
        .await;

    let fetched = client(&server).fetch(&format!("{}/dl/rutracker/?path=Q2ZESjhP", server.uri())).await.unwrap();

    assert_eq!(fetched, TorrentSource::Magnet(MAGNET.into()));
}

#[tokio::test]
async fn a_magnet_link_is_taken_as_it_is() {
    let client = JackettClient::new(Live::fixed(JackettSettings::default()));

    assert_eq!(client.fetch(MAGNET).await.unwrap(), TorrentSource::Magnet(MAGNET.into()));
}

#[tokio::test]
async fn a_link_elsewhere_than_jackett_is_not_fetched() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200)).expect(0).mount(&server).await;

    let error = client(&server).fetch("http://169.254.169.254/latest/meta-data").await.unwrap_err();

    assert!(matches!(error, IndexerError::Refused(_)), "{error}");
}

#[tokio::test]
async fn a_page_instead_of_a_torrent_is_refused() {
    let server = MockServer::start().await;
    Mock::given(path("/dl/rutracker/"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("<html>Login</html>", "text/html"))
        .mount(&server)
        .await;

    let error = client(&server).fetch(&format!("{}/dl/rutracker/?path=Q2ZESjhP", server.uri())).await.unwrap_err();

    assert!(matches!(error, IndexerError::Refused(_)), "{error}");
}

/// The Torznab feed of one tracker.
fn tracker_feed(tracker: &str) -> String {
    format!("/api/v2.0/indexers/{tracker}/results/torznab/api")
}

fn only(trackers: &[&str]) -> ReleaseQuery {
    let ids = trackers.iter().map(|tracker| tracker.parse::<TrackerId>().unwrap());
    ReleaseQuery { trackers: Trackers::Only(TrackerSet::new(ids).unwrap()), ..series("Frieren", None, None) }
}

fn titled(title: &str) -> String {
    format!(r#"<item><title>{title}</title><torznab:attr name="magneturl" value="{MAGNET}" /></item>"#)
}

#[tokio::test]
async fn the_configured_trackers_are_listed_with_their_names() {
    let server = MockServer::start().await;
    let listed = r#"<?xml version="1.0" encoding="UTF-8"?>
<indexers>
  <indexer id="anilibria" configured="true"><title>Anilibria</title><language>ru-RU</language><type>public</type></indexer>
  <indexer id="rutor" configured="true"><title>RuTor</title><language>ru-RU</language><type>public</type></indexer>
  <indexer id="1337x" configured="false"><title>1337x</title></indexer>
  <indexer id="bad id" configured="true"><title>Bad</title></indexer>
</indexers>"#;
    Mock::given(path(TORZNAB))
        .and(query_param("t", "indexers"))
        .and(query_param("configured", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(listed, "application/xml"))
        .mount(&server)
        .await;

    let trackers = client(&server).trackers().await.unwrap();

    assert_eq!(
        trackers,
        [
            Tracker { id: "anilibria".parse().unwrap(), name: "Anilibria".into() },
            Tracker { id: "rutor".parse().unwrap(), name: "RuTor".into() },
        ]
    );
}

#[tokio::test]
async fn chosen_trackers_are_searched_on_their_own_feeds_and_merged() {
    let server = MockServer::start().await;
    for (tracker, title) in [("rutor", "Frieren S01 RuTor"), ("anilibria", "Frieren S01 Anilibria")] {
        Mock::given(path(tracker_feed(tracker)))
            .and(query_param("apikey", KEY))
            .and(query_param("t", "tvsearch"))
            .and(query_param("q", "Frieren"))
            .respond_with(feed(&titled(title)))
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(path(TORZNAB)).respond_with(feed("")).expect(0).mount(&server).await;

    let releases = client(&server).search(&only(&["rutor", "anilibria"])).await.unwrap();

    let titles: Vec<&str> = releases.releases.iter().map(|release| release.title.as_str()).collect();
    assert_eq!(titles, ["Frieren S01 RuTor", "Frieren S01 Anilibria"]);
}

#[tokio::test]
async fn a_chosen_tracker_that_fails_leaves_the_others_releases() {
    let server = MockServer::start().await;
    Mock::given(path(tracker_feed("rutor"))).respond_with(feed(&titled("Frieren S01 RuTor"))).mount(&server).await;
    Mock::given(path(tracker_feed("anilibria"))).respond_with(ResponseTemplate::new(500)).mount(&server).await;

    let releases = client(&server).search(&only(&["rutor", "anilibria"])).await.unwrap();

    assert_eq!(releases.releases.len(), 1);
    assert_eq!(releases.warnings.len(), 1);
}

#[tokio::test]
async fn when_every_chosen_tracker_fails_the_search_fails() {
    let server = MockServer::start().await;
    let unsupported = r#"<?xml version="1.0" encoding="UTF-8"?><error code="201" description="anilibria does not support the requested query." />"#;
    Mock::given(path(tracker_feed("anilibria")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(unsupported, "application/xml"))
        .mount(&server)
        .await;

    let error = client(&server).search(&only(&["anilibria"])).await.unwrap_err();

    assert!(
        matches!(&error, IndexerError::Refused(reason) if reason == "anilibria doesn't support TV searches"),
        "{error}"
    );
}
