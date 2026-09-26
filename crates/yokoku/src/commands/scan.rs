use std::io::{self, Write};

use anyhow::Result;
use clap::Parser;

use crate::{app::App, config::Config};

#[derive(Parser)]
pub struct Args {}

pub async fn run(config: &Config, _args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let mut out = io::stdout();
    if app.roots.list().await?.is_empty() {
        writeln!(out, "No root folders; add one with `yokoku root add`.")?;
        return Ok(());
    }

    let report = app.scanner.scan().await?;
    app.deliver_events().await?;

    writeln!(out, "Linked {} new files", report.found)?;
    if report.vanished > 0 {
        writeln!(out, "Forgot {} files missing from disk", report.vanished)?;
    }
    if !report.needs_review.is_empty() {
        writeln!(out, "{} folders need review; see `yokoku review list`", report.needs_review.len())?;
    }
    Ok(())
}
