use rstest::rstest;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{basic_auth, body_partial_json, header},
};
use yokoku_domain::{Live, Secret};
use yokoku_downloads::{
    DownloadState,
    ports::{ClientError, DownloadClient, TorrentSource},
};
use yokoku_infra::download_clients::{TransmissionClient, TransmissionSettings};
use yokoku_test_support::transmission::{RPC, SESSION, answer, server, success};

const HASH: &str = "0638ffbb73b3f3ef1ba1fbbfa05a7e1db69610f6";

fn client(server: &MockServer) -> TransmissionClient {
    connect(TransmissionSettings { url: format!("{}{RPC}", server.uri()), ..TransmissionSettings::default() })
}

fn signed_in(server: &MockServer, password: &str) -> TransmissionClient {
    connect(TransmissionSettings {
        url: format!("{}{RPC}", server.uri()),
        username: Some("yokoku".into()),
        password: Some(Secret::new(password)),
    })
}

fn connect(settings: TransmissionSettings) -> TransmissionClient {
    TransmissionClient::new(Live::fixed(settings))
}

fn torrent(fields: Value) -> Value {
    let mut torrent = json!({
        "downloadDir": "/downloads",
        "error": 0,
        "errorString": "",
        "eta": -1,
        "hashString": HASH,
        "isFinished": false,
        "labels": ["yokoku"],
        "leftUntilDone": 0,
        "metadataPercentComplete": 1.0,
        "name": "Dune.2021.1080p.mkv",
        "rateDownload": 0,
        "sizeWhenDone": 3_000_000,
        "status": 6,
    });
    torrent.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
    torrent
}

#[tokio::test]
async fn takes_the_session_id_from_a_409_and_reports_the_version() {
    let server = server().await;
    answer(&server, "session-get", success(json!({ "rpc-version": 19, "version": "4.1.3 (0)" }))).await;

    let version = client(&server).version().await.unwrap();

    assert_eq!(version, "Transmission 4.1.3 (0)");
}

#[rstest]
#[case::magnet(TorrentSource::Magnet("magnet:?xt=urn:btih:abc".into()), json!({ "filename": "magnet:?xt=urn:btih:abc" }))]
#[case::file(TorrentSource::File(b"d4:infoe".to_vec()), json!({ "metainfo": "ZDQ6aW5mb2U=" }))]
#[tokio::test]
async fn adds_magnets_and_files_with_the_yokoku_label(#[case] source: TorrentSource, #[case] arguments: Value) {
    let server = server().await;
    let mut expected = json!({ "method": "torrent-add", "arguments": { "labels": ["yokoku"] } });
    expected["arguments"].as_object_mut().unwrap().extend(arguments.as_object().unwrap().clone());
    Mock::given(body_partial_json(expected))
        .and(header("X-Transmission-Session-Id", SESSION))
        .respond_with(success(
            json!({ "torrent-added": { "hashString": HASH.to_uppercase(), "id": 1, "name": "Dune" } }),
        ))
        .mount(&server)
        .await;

    let added = client(&server).add(&source).await.unwrap();

    assert_eq!((added.hash.as_str(), added.name.as_str()), (HASH, "Dune"));
}

#[tokio::test]
async fn a_duplicate_answers_with_the_existing_torrent() {
    let server = server().await;
    let duplicate = json!({ "torrent-duplicate": { "hashString": HASH, "id": 1, "name": "Dune" } });
    answer(&server, "torrent-add", success(duplicate)).await;

    let added = client(&server).add(&TorrentSource::File(vec![1])).await.unwrap();

    assert_eq!(added.hash, HASH);
}

#[tokio::test]
async fn a_refused_request_carries_transmissions_reason() {
    let server = server().await;
    answer(
        &server,
        "torrent-add",
        ResponseTemplate::new(200).set_body_json(json!({ "arguments": {}, "result": "unrecognized info" })),
    )
    .await;

    let error = client(&server).add(&TorrentSource::Magnet("not a magnet".into())).await.unwrap_err();

    assert!(matches!(error, ClientError::Refused(ref reason) if reason == "unrecognized info"), "{error}");
}

#[rstest]
#[case::seeding(json!({}), DownloadState::Seeding, true)]
#[case::queued_to_seed(json!({ "status": 5 }), DownloadState::Seeding, true)]
#[case::downloading(json!({ "status": 4, "leftUntilDone": 1_000_000, "rateDownload": 500, "eta": 2000 }), DownloadState::Downloading, false)]
#[case::queued(json!({ "status": 3, "leftUntilDone": 3_000_000 }), DownloadState::Queued, false)]
#[case::checking(json!({ "status": 2 }), DownloadState::Checking, false)]
#[case::stopped_but_done(json!({ "status": 0 }), DownloadState::Stopped, true)]
#[case::magnet_without_metadata(json!({ "status": 4, "sizeWhenDone": 0, "metadataPercentComplete": 0.0, "eta": -2 }), DownloadState::Downloading, false)]
#[tokio::test]
async fn reads_state_and_completion(#[case] fields: Value, #[case] state: DownloadState, #[case] complete: bool) {
    let server = server().await;
    answer(&server, "torrent-get", success(json!({ "torrents": [torrent(fields)] }))).await;

    let torrents = client(&server).torrents(&[HASH.into()]).await.unwrap();

    assert_eq!((torrents[0].status.state, torrents[0].complete), (state, complete));
}

#[tokio::test]
async fn reads_progress_rate_eta_folder_and_error() {
    let server = server().await;
    let fields = json!({
        "status": 4,
        "leftUntilDone": 1_000_000,
        "rateDownload": 500,
        "eta": 2000,
        "error": 3,
        "errorString": "No data found",
    });
    Mock::given(body_partial_json(json!({ "method": "torrent-get", "arguments": { "ids": [HASH] } })))
        .and(header("X-Transmission-Session-Id", SESSION))
        .respond_with(success(json!({ "torrents": [torrent(fields)] })))
        .mount(&server)
        .await;

    let torrents = client(&server).torrents(&[HASH.into()]).await.unwrap();

    let status = &torrents[0].status;
    assert_eq!((status.size, status.done, status.download_rate, status.eta), (3_000_000, 2_000_000, 500, Some(2000)));
    assert_eq!(status.download_dir, std::path::Path::new("/downloads"));
    assert_eq!(status.error.as_deref(), Some("No data found"));
    assert_eq!(torrents[0].name, "Dune.2021.1080p.mkv");
}

#[tokio::test]
async fn asking_for_no_torrents_sends_nothing() {
    let server = MockServer::start().await;

    let torrents = client(&server).torrents(&[]).await.unwrap();

    assert!(torrents.is_empty());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn sends_credentials_and_reports_rejected_ones() {
    let server = server().await;
    Mock::given(basic_auth("yokoku", "secret"))
        .and(header("X-Transmission-Session-Id", SESSION))
        .respond_with(success(json!({ "version": "4.1.3 (0)" })))
        .mount(&server)
        .await;
    answer(&server, "session-get", ResponseTemplate::new(401)).await;

    let accepted = signed_in(&server, "secret").version().await;
    let rejected = signed_in(&server, "wrong").version().await.unwrap_err();

    assert!(accepted.is_ok(), "{accepted:?}");
    assert!(matches!(rejected, ClientError::Refused(_)), "{rejected}");
}

#[tokio::test]
async fn a_server_error_is_unavailable() {
    let server = server().await;
    answer(&server, "session-get", ResponseTemplate::new(500)).await;

    let error = client(&server).version().await.unwrap_err();

    assert!(matches!(error, ClientError::Unavailable(_)), "{error}");
}

#[tokio::test]
async fn an_unreachable_client_is_unavailable() {
    let error = connect(TransmissionSettings {
        url: "http://127.0.0.1:9/transmission/rpc".into(),
        ..TransmissionSettings::default()
    })
    .version()
    .await
    .unwrap_err();

    assert!(matches!(error, ClientError::Unavailable(_)), "{error}");
}

#[tokio::test]
async fn lists_every_torrent_with_labels_and_finished_seeding() {
    let server = server().await;
    Mock::given(body_partial_json(json!({ "method": "torrent-get" })))
        .and(header("X-Transmission-Session-Id", SESSION))
        .and(|request: &Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            body["arguments"].get("ids").is_none()
        })
        .respond_with(success(
            json!({ "torrents": [torrent(json!({ "status": 0, "isFinished": true, "labels": ["tv", "anime"] }))] }),
        ))
        .mount(&server)
        .await;

    let torrents = client(&server).all_torrents().await.unwrap();

    assert_eq!(torrents.len(), 1);
    assert!(torrents[0].seeding_done);
    assert_eq!(torrents[0].labels, ["tv", "anime"]);
}

#[tokio::test]
async fn labels_are_empty_before_transmission_3() {
    let server = server().await;
    let mut old = torrent(json!({}));
    old.as_object_mut().unwrap().remove("labels");
    answer(&server, "torrent-get", success(json!({ "torrents": [old] }))).await;

    let torrents = client(&server).torrents(&[HASH.into()]).await.unwrap();

    assert!(torrents[0].labels.is_empty());
}

#[rstest]
#[case::with_data(true)]
#[case::without_data(false)]
#[tokio::test]
async fn removes_a_torrent_by_hash(#[case] delete_data: bool) {
    let server = server().await;
    Mock::given(body_partial_json(json!({
        "method": "torrent-remove",
        "arguments": { "ids": [HASH], "delete-local-data": delete_data }
    })))
    .and(header("X-Transmission-Session-Id", SESSION))
    .respond_with(success(json!({})))
    .expect(1)
    .mount(&server)
    .await;

    client(&server).remove(HASH, delete_data).await.unwrap();
}
