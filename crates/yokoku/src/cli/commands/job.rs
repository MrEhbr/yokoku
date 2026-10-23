use anyhow::{Result, anyhow, bail};
use clap::Parser;
use yokoku_core::jobs::Scheduler;

#[derive(Parser)]
pub struct Args {
    /// A scheduled job, such as sync-downloads
    pub job: String,
}

pub async fn run(scheduler: &Scheduler, args: Args) -> Result<()> {
    let Some(job) = scheduler.job(&args.job) else {
        let names: Vec<_> = scheduler.jobs().iter().map(|job| job.name()).collect();
        bail!("No job named {}; the jobs are {}", args.job, names.join(", "));
    };
    job.run().await.map_err(|error| anyhow!(error))?;
    success!("Ran {}", job.name())?;
    Ok(())
}
