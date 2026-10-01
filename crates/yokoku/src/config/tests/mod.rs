mod fields;
mod load;
mod masking;
mod settings;

use serde_json::Value;

use crate::config::Config;

/// Every setting's dotted key, like `import.mode`.
fn keys() -> Vec<String> {
    fn leaves(value: &Value, prefix: &str) -> Vec<String> {
        match value.as_object() {
            Some(object) => object
                .iter()
                .flat_map(|(key, value)| {
                    let key = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };
                    leaves(value, &key)
                })
                .collect(),
            None => vec![prefix.to_owned()],
        }
    }
    leaves(&serde_json::to_value(Config::default()).unwrap(), "")
}
