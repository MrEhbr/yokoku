#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown {kind} {value:?}")]
pub struct ParseEnumError {
    kind: &'static str,
    value: String,
}

impl ParseEnumError {
    pub fn new(kind: &'static str, value: &str) -> Self {
        Self { kind, value: value.to_owned() }
    }
}

/// Implements `as_str`, `FromStr` and `Display` for a fieldless enum from one variant-to-name table;
/// `Display` shows the name with spaces for underscores.
#[macro_export]
macro_rules! string_enum {
    ($type:ty, $kind:literal { $($variant:ident => $name:literal),+ $(,)? }) => {
        impl $type {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }
        }

        impl ::std::str::FromStr for $type {
            type Err = $crate::ParseEnumError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($name => Ok(Self::$variant),)+
                    _ => Err($crate::ParseEnumError::new($kind, value)),
                }
            }
        }

        impl ::std::fmt::Display for $type {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.pad(&self.as_str().replace('_', " "))
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{Confidence, SeriesStatus};

    #[rstest]
    #[case::one_word(format!("{}", Confidence::Guess), "guess")]
    #[case::underscores_become_spaces(format!("{}", SeriesStatus::OnBreak), "on break")]
    #[case::padded(format!("{:<10}|", SeriesStatus::OnBreak), "on break  |")]
    fn displays_the_name(#[case] shown: String, #[case] expected: &str) {
        assert_eq!(shown, expected);
    }
}
