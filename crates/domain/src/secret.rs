use std::path::PathBuf;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

/// Characters `masked` shows at each end.
const SHOWN: usize = 4;

/// A secret value, given inline or read from a file when deserialized.
///
/// In TOML:
///   `key = "literal-value"`
///   `key = { file = "/path" }`   the file's content, without trailing whitespace
///
/// It serializes as its value, so a saved secret loads again; `masked` is how it is shown, and `Debug`
/// shows nothing of it.
#[derive(Debug, Clone)]
pub struct Secret(SecretString);

#[derive(Deserialize)]
#[serde(untagged)]
enum Source {
    Inline(String),
    File { file: PathBuf },
}

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().into())
    }

    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }

    /// The first and last four characters, `abcd…wxyz`; for eight or fewer, `…` and the last quarter.
    pub fn masked(&self) -> String {
        let chars: Vec<char> = self.expose().chars().collect();
        let (head, tail) = if chars.len() > 2 * SHOWN { (SHOWN, SHOWN) } else { (0, chars.len() / 4) };
        let (start, end) = (&chars[..head], &chars[chars.len() - tail..]);
        format!("{}…{}", String::from_iter(start), String::from_iter(end))
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
        serializer.serialize_str(self.expose())
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

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
    fn serializes_as_its_value_so_it_loads_again() {
        let saved = toml::to_string(&holder(r#"secret = "hello""#).unwrap()).unwrap();

        assert_eq!(holder(&saved).unwrap().secret.expose(), "hello");
    }

    #[rstest]
    #[case::long("eyJhbGciOiJIUzI1NiJ9", "eyJh…NiJ9")]
    #[case::nine("123456789", "1234…6789")]
    #[case::eight("12345678", "…78")]
    #[case::pin("1234", "…4")]
    #[case::tiny("abc", "…")]
    #[case::multibyte("пароль-секрет", "паро…крет")]
    fn masks_all_but_its_ends(#[case] value: &str, #[case] masked: &str) {
        assert_eq!(Secret::new(value).masked(), masked);
    }

    #[test]
    fn debug_shows_nothing_of_it() {
        assert!(!format!("{:?}", Secret::new("hello")).contains("hel"));
    }
}
