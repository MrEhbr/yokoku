use yokoku_core::downloads::ports::{Indexer, IndexerError, ReleaseQuery};
use yokoku_domain::{Live, Secret};
use yokoku_infra::indexers::{JackettClient, JackettSettings};
use yokoku_test_support::services;

fn client(api_key: &str) -> JackettClient {
    JackettClient::new(Live::fixed(JackettSettings {
        url: Some(services::jackett_url()),
        api_key: Some(Secret::new(api_key)),
    }))
}

#[tokio::test]
#[ignore = "needs the services from `just services`"]
async fn jackett_answers_its_caps_and_searches() {
    let client = client(&services::jackett_api_key());

    assert_eq!(client.version().await.unwrap(), "Jackett");
    let query = ReleaseQuery { text: "Dune".into(), kind: None, season: None, episode: None };
    client.search(&query).await.unwrap();
}

#[tokio::test]
#[ignore = "needs the services from `just services`"]
async fn jackett_refuses_a_wrong_api_key() {
    let error = client("wrong").version().await.unwrap_err();

    assert!(matches!(&error, IndexerError::Refused(reason) if reason == "Invalid API Key"), "{error}");
}
