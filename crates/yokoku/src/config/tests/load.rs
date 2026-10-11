use std::path::PathBuf;

use serde_json::json;
use tracing::Level;
use yokoku_core::media::RootKind;
use yokoku_infra::indexers::IndexerProvider;

use super::keys;
use crate::config::{Config, LogFormat, LogOutput, RootConfig};

#[test]
fn stored_settings_go_over_the_config_file_and_it_over_the_defaults() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        file.path(),
        "[log]\nlevel = \"debug\"\nformat = \"json\"\noutput = { file = \"/var/log/yokoku.log\" }\n\n\
         [import]\nmode = \"copy\"\n",
    )
    .unwrap();

    let config = Config::load(Some(file.path()), &[("import.mode".into(), json!("move"))]).unwrap();

    assert_eq!(config.log.level, Some(Level::DEBUG));
    assert_eq!(config.log.format, LogFormat::Json);
    assert_eq!(config.log.output, LogOutput::File(PathBuf::from("/var/log/yokoku.log")));
    assert_eq!(config.setting("import.mode").unwrap(), "\"move\"");
}

#[test]
fn the_sample_config_files_load_and_list_every_setting() {
    for name in ["app.toml", "docker.toml"] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config").join(name);
        Config::load(Some(&path), &[]).unwrap_or_else(|error| panic!("{name}: {error:#}"));

        let uncommented: String = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| match line.strip_prefix("# ") {
                Some(setting) if setting.split_once(" = ").is_some_and(|(key, _)| !key.contains(' ')) => setting,
                _ => line,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let listed = config::Config::builder()
            .add_source(config::File::from_str(&uncommented, config::FileFormat::Toml))
            .build()
            .unwrap();
        for key in keys() {
            assert!(listed.get::<config::Value>(&key).is_ok(), "{name} does not list {key}");
        }
    }
}

#[test]
fn the_log_level_is_shown_in_lowercase() {
    assert_eq!(Config::default().setting("log.level").unwrap(), "\"info\"");
}

#[test]
fn the_only_configured_indexer_is_selected_until_a_provider_is_set() {
    let mut config = Config::default();
    config.prowlarr.url = Some("http://localhost:9696".into());
    assert_eq!(config.active_indexer(), IndexerProvider::Prowlarr);
    assert_eq!(config.setting("indexer.provider").unwrap(), "\"prowlarr\"");

    config.jackett.url = Some("http://localhost:9117".into());
    assert_eq!(config.active_indexer(), IndexerProvider::Jackett);

    config.indexer.provider = Some(IndexerProvider::Prowlarr);
    assert_eq!(config.active_indexer(), IndexerProvider::Prowlarr);
}

#[test]
fn selected_indexer_loads_from_stored_settings() {
    let legacy = Config::load(None, &[("prowlarr.url".into(), json!("http://localhost:9696"))]).unwrap();
    assert_eq!(legacy.active_indexer(), IndexerProvider::Prowlarr);

    let config = Config::load(None, &[("indexer.provider".into(), json!("prowlarr"))]).unwrap();
    assert_eq!(config.active_indexer(), IndexerProvider::Prowlarr);
    assert!(Config::load(None, &[("indexer.provider".into(), json!("other"))]).is_err());
}

#[test]
fn root_folders_load_from_the_config_file_with_optional_names() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        file.path(),
        "[[roots]]\nkind = \"series\"\npath = \"/media/library/Anime\"\n\n\
         [[roots]]\nkind = \"movies\"\npath = \"/media/library/AnimeMovies\"\nname = \"Anime movies\"\n",
    )
    .unwrap();

    let config = Config::load(Some(file.path()), &[]).unwrap();

    assert_eq!(
        config.roots,
        [
            RootConfig { kind: RootKind::Series, path: "/media/library/Anime".into(), name: None },
            RootConfig {
                kind: RootKind::Movies,
                path: "/media/library/AnimeMovies".into(),
                name: Some("Anime movies".into())
            },
        ]
    );
}

#[test]
fn a_root_folder_with_an_unknown_key_is_refused() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "[[roots]]\nkind = \"series\"\npath = \"/media/anime\"\nlabel = \"Anime\"\n").unwrap();

    assert!(Config::load(Some(file.path()), &[]).is_err());
}
