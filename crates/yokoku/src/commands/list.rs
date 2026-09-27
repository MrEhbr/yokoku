use anyhow::Result;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tracing::debug;
use yokoku_domain::{MovieStatus, SeriesStatus};
use yokoku_library::{LibraryFilter, LibrarySort, LibraryStatus};

use crate::{
    app::App,
    commands::{Kind, title_with_year},
    config::Config,
    output::Paint,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct ListConfig {
    pub sort: Sort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Sort {
    #[default]
    Title,
    Added,
    NextRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Status {
    Continuing,
    OnBreak,
    Ended,
    Announced,
    InCinemas,
    Released,
}

#[derive(Parser)]
pub struct Args {
    /// Only items of this type
    #[arg(long)]
    pub kind: Option<Kind>,

    /// Only items with this status
    #[arg(long)]
    pub status: Option<Status>,

    /// Sort order, overriding `list.sort`
    #[arg(long)]
    pub sort: Option<Sort>,
}

impl Args {
    /// Command flags take precedence over the resolved configuration.
    fn apply_overrides(&self, config: &ListConfig) -> ListConfig {
        let mut resolved = config.clone();

        if let Some(sort) = self.sort {
            resolved.sort = sort;
        }

        resolved
    }
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let settings = args.apply_overrides(&config.list);
    debug!(?settings, "resolved command settings");

    let app = App::open(config).await?;
    let filter = LibraryFilter { kind: args.kind.map(Into::into), status: args.status.map(Into::into) };
    let entries = app.library.list(filter, settings.sort.into()).await?;

    if entries.is_empty() {
        hint!("No items.")?;
    }
    for entry in entries {
        say!(
            "{:<40} {:<6} {:<10} {:<5} {:<10} {}",
            title_with_year(&entry.title, entry.year).bold(),
            entry.id.kind(),
            entry.status.tone(),
            if entry.has_files { "files" } else { "-" },
            entry.next_release.map_or_else(|| "-".to_owned(), |date| date.to_string()),
            entry.source,
        )?;
    }

    Ok(())
}

impl From<Sort> for LibrarySort {
    fn from(sort: Sort) -> Self {
        match sort {
            Sort::Title => Self::Title,
            Sort::Added => Self::Added,
            Sort::NextRelease => Self::NextRelease,
        }
    }
}

impl From<Status> for LibraryStatus {
    fn from(status: Status) -> Self {
        match status {
            Status::Continuing => Self::Series(SeriesStatus::Continuing),
            Status::OnBreak => Self::Series(SeriesStatus::OnBreak),
            Status::Ended => Self::Series(SeriesStatus::Ended),
            Status::Announced => Self::Movie(MovieStatus::Announced),
            Status::InCinemas => Self::Movie(MovieStatus::InCinemas),
            Status::Released => Self::Movie(MovieStatus::Released),
        }
    }
}
