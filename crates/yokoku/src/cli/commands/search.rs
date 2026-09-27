use anyhow::Result;
use clap::Parser;

use crate::{
    app::App,
    cli::{commands::title_with_year, output::Paint},
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    /// Title to search for
    #[arg(required = true)]
    pub query: Vec<String>,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let hits = app.metadata()?.search(&args.query.join(" ")).await?;

    if hits.is_empty() {
        hint!("No results.")?;
    }
    for hit in hits {
        let result = hit.result;
        say!(
            "{:<6} {:<50} {:<14} {}",
            result.kind,
            title_with_year(&result.title, result.year).bold(),
            result.source,
            if hit.in_library { "in library" } else { "" }.green(),
        )?;
    }

    Ok(())
}
