use std::io::Write;

use anyhow::{Result, bail};
use clap::Parser;
use owo_colors::OwoColorize;
use yokoku_domain::{ExternalId, ItemId};

use crate::{
    app::App,
    commands::{ItemArgs, Kind, item_title},
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    /// Item type; the whole library is refreshed when omitted
    #[arg(requires = "source")]
    pub kind: Option<Kind>,

    /// Source id, e.g. `tmdb:1396`
    #[arg(requires = "kind")]
    pub source: Option<ExternalId>,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let mut out = anstream::stdout();
    let sync = app.sync()?;
    let item = ItemArgs::optional(args.kind, args.source);

    let Some(item) = item else {
        let report = sync.refresh_all().await?;
        writeln!(out, "Refreshed {} items", report.refreshed)?;
        for failure in &report.failures {
            let name = item_title(&app.library, failure.item).await;
            writeln!(out, "{}", format!("Failed {name}: {}", failure.error).red())?;
        }
        if !report.failures.is_empty() {
            bail!("{} items could not be refreshed", report.failures.len());
        }
        return Ok(());
    };

    match item.resolve(&app.library).await? {
        ItemId::Series(id) => sync.refresh_series(id).await.map(drop)?,
        ItemId::Movie(id) => sync.refresh_movie(id).await.map(drop)?,
    }
    writeln!(out, "Refreshed {}", item.source)?;
    Ok(())
}
