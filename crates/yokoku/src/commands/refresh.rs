use anyhow::{Result, bail};
use clap::Parser;
use yokoku_domain::ExternalId;
use yokoku_library::ItemId;

use crate::{
    app::App,
    commands::{ItemArgs, Kind},
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
    let sync = app.sync()?;
    let item = args.kind.zip(args.source).map(|(kind, source)| ItemArgs { kind, source });

    let Some(item) = item else {
        let report = sync.refresh_all().await?;
        println!("Refreshed {} items", report.refreshed);
        for failure in &report.failures {
            let name = match failure.item {
                ItemId::Series(id) => app.library.series(id).await.map(|series| series.title),
                ItemId::Movie(id) => app.library.movie(id).await.map(|movie| movie.title),
            };
            println!("Failed {}: {}", name.unwrap_or_else(|_| "unknown item".into()), failure.error);
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
    println!("Refreshed {}", item.source);
    Ok(())
}
