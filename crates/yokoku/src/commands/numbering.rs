use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use yokoku_domain::{ExternalId, Numbering as SeriesNumbering};

use crate::{app::App, config::Config};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Numbering {
    /// Season and episode numbers, e.g. S01E02
    Standard,
    /// One running episode number, fansub style
    Absolute,
}

#[derive(Parser)]
pub struct Args {
    /// Series source id, e.g. `tmdb:209867`
    pub source: ExternalId,

    pub numbering: Numbering,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let series = app
        .library
        .find_series(args.source)
        .await?
        .with_context(|| format!("series {} is not in the library", args.source))?;

    let numbering = match args.numbering {
        Numbering::Standard => SeriesNumbering::Standard,
        Numbering::Absolute => SeriesNumbering::Absolute,
    };
    app.library.set_numbering(series.id, numbering).await?;

    Ok(())
}
