use std::path::PathBuf;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

/// How every secret serializes.
pub const REDACTED: &str = "<redacted>";

/// A secret value, given inline or read from a file when deserialized.
///
/// In TOML:
///   `key = "literal-value"`
///   `key = { file = "/path" }`   the file's content, without trailing whitespace
#[derive(Debug, Clone)]
pub struct Secret(SecretString);

#[derive(Deserialize)]
#[serde(untagged)]
enum Source {
    Inline(String),
    File { file: PathBuf },
}

impl Secret {
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = match Source::deserialize(deserializer)? {
            Source::Inline(value) => value,
            Source::File { file } => std::fs::read_to_string(&file)
                .map_err(|error| D::Error::custom(format!("failed to read secret file {}: {error}", file.display())))?
                .trim_end()
                .to_owned(),
        };
        Ok(Self(value.into()))
    }
}

impl Serialize for Secret {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(REDACTED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize, Serialize)]
    struct Holder {
        secret: Secret,
    }

    fn holder(toml: &str) -> Result<Holder, toml::de::Error> {
        toml::from_str(toml)
    }

    #[test]
    fn deserializes_an_inline_string() {
        assert_eq!(holder(r#"secret = "hello""#).unwrap().secret.expose(), "hello");
    }

    #[test]
    fn reads_a_file_without_trailing_whitespace() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "p4ssw0rd\n\n").unwrap();

        let holder = holder(&format!("secret = {{ file = {:?} }}", file.path().display().to_string())).unwrap();

        assert_eq!(holder.secret.expose(), "p4ssw0rd");
    }

    #[test]
    fn a_missing_file_fails() {
        let error = holder(r#"secret = { file = "/nonexistent/secret/file" }"#).err().unwrap();

        assert!(error.to_string().contains("failed to read secret file /nonexistent/secret/file"), "{error}");
    }

    #[test]
    fn never_shows_the_value() {
        let holder = holder(r#"secret = "hello""#).unwrap();

        assert_eq!(serde_json::to_value(&holder).unwrap()["secret"], REDACTED);
        assert!(!format!("{:?}", holder.secret).contains("hello"));
    }
}
