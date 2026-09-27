pub mod add;
pub mod calendar;
pub mod delete;
pub mod download;
pub mod files;
pub mod history;
pub mod import;
pub mod jellyfin;
pub mod list;
pub mod missing;
pub mod monitor;
pub mod numbering;
pub mod refresh;
pub mod remove;
pub mod rename;
pub mod review;
pub mod root;
pub mod scan;
pub mod search;
pub mod serve;
pub mod settings;
pub mod show;

use std::io::{self, Write};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use yokoku_domain::{EpisodeSpan, ExternalId, FileTarget, ItemId, MediaKind};
use yokoku_library::Library;
use yokoku_media::MediaFile;

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

    /// `episodes` of a series, or a movie; fails when `episodes` does not fit the item type.
    pub async fn file_target(&self, library: &Library, episodes: Option<EpisodeSpan>) -> Result<FileTarget> {
        match (self.resolve(library).await?, episodes) {
            (ItemId::Series(series), Some(span)) => Ok(FileTarget::Episodes { series, span }),
            (ItemId::Series(_), None) => bail!("A series needs episodes, e.g. S01E02"),
            (ItemId::Movie(movie), None) => Ok(FileTarget::Movie(movie)),
            (ItemId::Movie(_), Some(_)) => bail!("A movie takes no episodes"),
        }
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

/// Lists `files` and asks on stdin to delete them; fails unless the answer is `y` or `yes`.
pub fn confirm_deletion(files: &[MediaFile], yes: bool) -> Result<()> {
    if yes || files.is_empty() {
        return Ok(());
    }
    for file in files {
        say!("  {}", file.path.display())?;
    }
    let noun = if files.len() == 1 { "file" } else { "files" };
    let mut out = io::stdout();
    write!(out, "Delete {} {noun}? [y/N] ", files.len())?;
    out.flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    match answer.trim().to_lowercase().as_str() {
        "y" | "yes" => Ok(()),
        _ => bail!("Nothing was deleted"),
    }
}

pub fn title_with_year(title: &str, year: Option<i16>) -> String {
    match year {
        Some(year) => format!("{title} ({year})"),
        None => title.to_owned(),
    }
}
