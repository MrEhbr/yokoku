use anyhow::Result;
use clap::Parser;
use yokoku_domain::title_with_year;

use crate::{app::App, cli::output::Paint};

#[derive(Parser)]
pub struct Args {
    /// Title to search for
    #[arg(required = true)]
    pub query: Vec<String>,
}

pub async fn run(app: &App, args: Args) -> Result<()> {
    let hits = app.metadata()?.search(&args.query.join(" "), None).await?;

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
            if hit.in_library.is_some() { "in library" } else { "" }.green(),
        )?;
    }

    Ok(())
}
