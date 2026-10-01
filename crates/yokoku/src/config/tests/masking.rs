use serde_json::json;

use super::keys;
use crate::config::Config;

const SECRET: &str = "a-long-secret-value";

#[test]
fn every_secret_is_shown_masked() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), SECRET).unwrap();

    let secrets: Vec<(String, String)> = keys()
        .into_iter()
        .filter_map(|key| {
            Config::editable(&key).ok()?;
            let stored = [(key.clone(), json!({ "file": file.path() }))];
            let shown = Config::load(None, &stored).ok()?.setting(&key).ok()?;
            Some((key, shown))
        })
        .collect();

    assert!(!secrets.is_empty());
    for (key, shown) in secrets {
        assert!(shown.contains('…') && !shown.contains(SECRET), "{key} = {shown}");
    }
}
