use anyhow::{Result, anyhow};
use clap::Parser;

use crate::{app::App, jobs::Job};

#[derive(Parser)]
pub struct Args {
    pub job: Job,
}

pub async fn run(app: &App, args: Args) -> Result<()> {
    args.job.run(app).await.map_err(|error| anyhow!(error))?;
    success!("Ran {}", args.job.name())?;
    Ok(())
}
