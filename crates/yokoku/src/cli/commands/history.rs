use anyhow::Result;
use clap::Parser;
use yokoku_domain::ExternalId;

use crate::{
    app::App,
    cli::{
        commands::{ItemArgs, Kind},
        output::Paint,
    },
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    /// Item type; the whole library's history when omitted
    #[arg(requires = "source")]
    pub kind: Option<Kind>,

    /// Source id, e.g. `tmdb:1396`
    #[arg(requires = "kind")]
    pub source: Option<ExternalId>,

    /// Most entries to show
    #[arg(long, short = 'n', default_value_t = 50)]
    pub limit: usize,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let item = match ItemArgs::optional(args.kind, args.source) {
        Some(item) => Some(item.resolve(&app.library).await?),
        None => None,
    };
    let time_zone = config.clock.time_zone()?;

    let entries = app.history.page(item, None, args.limit).await?;
    if entries.is_empty() {
        hint!("No history.")?;
    }
    for recorded in entries {
        let at = recorded.occurred_at.to_zoned(time_zone.clone()).strftime("%Y-%m-%d %H:%M");
        let description = recorded.event.to_string();
        let mut lines = description.lines();
        say!("{}  {}", at.dimmed(), lines.next().unwrap_or_default())?;
        for line in lines {
            say!("                  {line}")?;
        }
    }
    Ok(())
}
