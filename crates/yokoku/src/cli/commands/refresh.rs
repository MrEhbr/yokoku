use anyhow::{Result, bail};
use clap::Parser;
use yokoku_domain::{ExternalId, ItemId};

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
    let metadata = app.metadata()?;
    let item = ItemArgs::optional(args.kind, args.source);

    let Some(item) = item else {
        let report = metadata.refresh_all().await?;
        success!("Refreshed {} items", report.refreshed)?;
        for failure in &report.failures {
            let name = app.title(failure.item).await;
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
