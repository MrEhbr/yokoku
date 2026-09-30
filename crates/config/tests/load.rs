use std::path::PathBuf;

use serde_json::json;
use tracing::Level;
use yokoku_config::{Config, LogFormat, LogOutput};

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
fn the_log_level_is_shown_in_lowercase() {
    assert_eq!(Config::default().setting("log.level").unwrap(), "\"info\"");
}
