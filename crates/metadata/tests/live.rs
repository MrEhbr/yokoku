use jiff::civil::date;
use yokoku_domain::{ExternalId, MediaKind};
use yokoku_library::ports::MetadataProvider;
use yokoku_metadata::TmdbClient;

fn client() -> TmdbClient {
    let token = std::env::var("APP__METADATA__TMDB__TOKEN").expect("APP__METADATA__TMDB__TOKEN is set");
    TmdbClient::new(token, "en-US", "US")
}

#[tokio::test]
#[ignore = "calls the real TMDB API; run with APP__METADATA__TMDB__TOKEN set"]
async fn real_tmdb_matches_the_recorded_shapes() {
    let client = client();

    let results = client.search("frieren").await.unwrap();
    assert!(results.iter().any(|r| r.kind == MediaKind::Series && r.source == ExternalId::Tmdb(209867)));

    let frieren = client.series(ExternalId::Tmdb(209867)).await.unwrap();
    assert_eq!(frieren.seasons[1].episodes[0].air_date, Some(date(2023, 9, 29)));
    assert!(frieren.alternate_titles.contains(&"Sousou no Frieren".to_owned()));

    let dune = client.movie(ExternalId::Tmdb(438631)).await.unwrap();
    assert_eq!(dune.releases.cinema, Some(date(2021, 10, 22)));
    assert!(dune.alternate_titles.contains(&"Dune: Part One".to_owned()));
}
