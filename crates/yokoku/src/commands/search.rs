use std::io::{self, Write};

use anyhow::Result;
use clap::Parser;

use crate::{
    app::App,
    commands::{kind_label, title_with_year},
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
    let mut out = io::stdout();
    let hits = app.sync()?.search(&args.query.join(" ")).await?;

    if hits.is_empty() {
        writeln!(out, "No results.")?;
    }
    for hit in hits {
        let result = hit.result;
        writeln!(
            out,
            "{:<6} {:<50} {:<14} {}",
            kind_label(result.kind),
            title_with_year(&result.title, result.year),
            result.source,
            if hit.in_library { "in library" } else { "" },
        )?;
    }

    Ok(())
}
