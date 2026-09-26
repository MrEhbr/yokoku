use std::io::{self, Write};

use anyhow::{Result, bail};
use clap::Parser;
use yokoku_domain::{EpisodeSpan, ExternalId, FileTarget, ItemId};

use crate::{
    app::App,
    commands::{ItemArgs, Kind},
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    pub kind: Kind,
    /// Source id, e.g. `tmdb:1396`
    pub source: ExternalId,
    /// Episodes whose file to delete, e.g. `S01E02`; series only
    pub episodes: Option<EpisodeSpan>,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let target = match (ItemArgs { kind: args.kind, source: args.source }.resolve(&app.library).await?, args.episodes) {
        (ItemId::Series(series), Some(span)) => FileTarget::Episodes { series, span },
        (ItemId::Series(_), None) => bail!("Name the episodes whose file to delete, e.g. S01E02"),
        (ItemId::Movie(movie), None) => FileTarget::Movie(movie),
        (ItemId::Movie(_), Some(_)) => bail!("A movie takes no episodes"),
    };

    let deleted = app.deleter.delete(target).await?;
    app.deliver_events().await?;
    let verb = if config.recycle.folder.is_some() { "Recycled" } else { "Deleted" };
    let mut out = io::stdout();
    for file in deleted {
        writeln!(out, "{verb} {}", file.path.display())?;
    }
    Ok(())
}
