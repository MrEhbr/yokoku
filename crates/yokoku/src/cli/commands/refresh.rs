use anyhow::{Result, bail};
use clap::Parser;
use yokoku_domain::{ExternalId, ItemId, ItemName};

use crate::{
    app::App,
    cli::commands::{ItemArgs, Kind},
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

pub async fn run(app: &App, args: Args) -> Result<()> {
    if app.settings.current().metadata.tmdb.token.is_none() {
        bail!("No TMDB token configured; set APP__METADATA__TMDB__TOKEN");
    }
    let metadata = &app.metadata;
    let item = ItemArgs::optional(args.kind, args.source);

    let Some(item) = item else {
        let report = metadata.refresh_all().await?;
        success!("Refreshed {} items", report.refreshed)?;
        for failure in &report.failures {
            let found = match failure.item {
                ItemId::Series(id) => app.library.series(id).await.map(|series| (series.title, series.year)),
                ItemId::Movie(id) => app.library.movie(id).await.map(|movie| (movie.title, movie.year)),
            };
            let name = found.map_or_else(
                |_| format!("removed {}", failure.item.kind()),
                |(title, year)| ItemName::new(&title, year).to_string(),
            );
            failure!("Failed {name}: {}", failure.error)?;
        }
        if !report.failures.is_empty() {
            bail!("{} items could not be refreshed", report.failures.len());
        }
        return Ok(());
    };

    match item.resolve(&app.library).await? {
        ItemId::Series(id) => metadata.refresh_series(id).await.map(drop)?,
        ItemId::Movie(id) => metadata.refresh_movie(id).await.map(drop)?,
    }
    success!("Refreshed {}", item.source)?;
    Ok(())
}
