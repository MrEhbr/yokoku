use jiff::civil::date;
use yokoku_core::library::ports::MetadataProvider;
use yokoku_domain::{ExternalId, Live, MediaKind, Secret};
use yokoku_infra::metadata::{MetadataSettings, TmdbClient, TvdbClient};

fn client() -> TmdbClient {
    let token = std::env::var("YOKOKU__METADATA__TMDB__TOKEN").expect("YOKOKU__METADATA__TMDB__TOKEN is set");
    let mut settings = MetadataSettings::default();
    settings.tmdb.token = Some(Secret::new(token));
    TmdbClient::new(Live::fixed(settings))
}

#[tokio::test]
#[ignore = "calls the real TMDB API; run with YOKOKU__METADATA__TMDB__TOKEN set"]
async fn real_tmdb_matches_the_recorded_shapes() {
    let client = client();

    let results = client.search("frieren", None).await.unwrap();
    assert!(results.iter().any(|r| r.kind == MediaKind::Series && r.source == ExternalId::Tmdb(209867)));

    let frieren = client.series(ExternalId::Tmdb(209867)).await.unwrap();
    assert_eq!(frieren.seasons[1].episodes[0].air_date, Some(date(2023, 9, 29)));
    assert!(frieren.alternate_titles.contains(&"Sousou no Frieren".to_owned()));

    let dune = client.movie(ExternalId::Tmdb(438631)).await.unwrap();
    assert_eq!(dune.releases.cinema, Some(date(2021, 10, 22)));
    assert!(dune.alternate_titles.contains(&"Dune: Part One".to_owned()));
}

#[tokio::test]
#[ignore = "calls the real TVDB API; run with YOKOKU__METADATA__TVDB__API_KEY and YOKOKU__METADATA__TVDB__PIN set"]
async fn real_tvdb_matches_the_assumed_shapes() {
    let api_key = std::env::var("YOKOKU__METADATA__TVDB__API_KEY").expect("YOKOKU__METADATA__TVDB__API_KEY is set");
    let mut settings = MetadataSettings::default();
    settings.tvdb.api_key = Some(Secret::new(api_key));
    settings.tvdb.pin = std::env::var("YOKOKU__METADATA__TVDB__PIN").ok().map(Secret::new);
    let client = TvdbClient::new(Live::fixed(settings));

    let results = client.search("frieren", None).await.unwrap();
    assert!(results.iter().any(|r| r.kind == MediaKind::Series && r.source == ExternalId::Tvdb(424536)));

    let frieren = client.series(ExternalId::Tvdb(424536)).await.unwrap();
    assert_eq!(frieren.title, "Frieren: Beyond Journey's End");
    assert_eq!(frieren.seasons[1].episodes[0].air_date, Some(date(2023, 9, 29)));
}
