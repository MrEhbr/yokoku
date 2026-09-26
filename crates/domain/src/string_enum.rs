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

/// Implements `as_str` and `FromStr` for a fieldless enum from one variant-to-name table.
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
    };
}
