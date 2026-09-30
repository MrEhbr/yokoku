use anyhow::Result;
use clap::Parser;

use crate::app::App;

#[derive(Parser)]
pub struct Args {}

pub async fn run(app: &App, _args: Args) -> Result<()> {
    if app.roots.list().await?.is_empty() {
        hint!("No root folders; add one with `yokoku root add`.")?;
        return Ok(());
    }

    let report = app.scanner.scan().await?;
    app.deliver_events().await?;

    success!("Linked {} new files", report.found)?;
    if report.vanished > 0 {
        say!("Forgot {} files missing from disk", report.vanished)?;
    }
    if !report.needs_review.is_empty() {
        say!("{} folders need review; match their files on each item's page", report.needs_review.len())?;
    }
    Ok(())
}
