pub mod completions;
pub mod files;
pub mod jellyfin;
pub mod job;
pub mod refresh;
pub mod root;
pub mod scan;
pub mod settings;

use anyhow::{Context, Result};
use clap::ValueEnum;
use yokoku_core::library::Library;
use yokoku_domain::{ExternalId, ItemId, MediaKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Kind {
    Series,
    Movie,
}

/// Identifies a library item by type and source id.
#[derive(Debug, clap::Args)]
pub struct ItemArgs {
    /// Item type
    pub kind: Kind,

    /// Source id, e.g. `tmdb:1396`
    pub source: ExternalId,
}

impl ItemArgs {
    /// `None` unless both `kind` and `source` are given.
    pub fn optional(kind: Option<Kind>, source: Option<ExternalId>) -> Option<Self> {
        kind.zip(source).map(|(kind, source)| Self { kind, source })
    }

    pub async fn resolve(&self, library: &Library) -> Result<ItemId> {
        let id = match self.kind {
            Kind::Series => library.find_series(self.source).await?.map(|series| ItemId::Series(series.id)),
            Kind::Movie => library.find_movie(self.source).await?.map(|movie| ItemId::Movie(movie.id)),
        };
        id.with_context(|| format!("{} {} is not in the library", MediaKind::from(self.kind), self.source))
    }
}

impl From<Kind> for MediaKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Series => Self::Series,
            Kind::Movie => Self::Movie,
        }
    }
}
