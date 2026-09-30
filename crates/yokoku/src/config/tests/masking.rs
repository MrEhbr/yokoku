use serde_json::{Value, json};

use crate::config::Config;

const SECRET: &str = "a-long-secret-value";

/// Every leaf's dotted key, e.g. `import.mode`.
fn leaves(value: &Value, prefix: &str, keys: &mut Vec<String>) {
    match value {
        Value::Object(table) => {
            for (name, value) in table {
                leaves(value, &format!("{prefix}{name}."), keys);
            }
        },
        _ => keys.push(prefix.trim_end_matches('.').to_owned()),
    }
}

#[test]
fn every_secret_is_shown_masked() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), SECRET).unwrap();
    let mut all = Vec::new();
    leaves(&serde_json::to_value(Config::default()).unwrap(), "", &mut all);

    let secrets: Vec<(String, String)> = all
        .into_iter()
        .filter_map(|key| {
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
