use std::io::Write;

use anyhow::Result;
use clap::Parser;
use yokoku_domain::EpisodeSpan;

use crate::{
    app::App,
    commands::{ItemArgs, confirm_deletion},
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    #[command(flatten)]
    pub item: ItemArgs,
    /// Episodes whose file to delete, e.g. `S01E02`; series only
    pub episodes: Option<EpisodeSpan>,
    /// Delete without asking
    #[arg(short, long)]
    pub yes: bool,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let target = args.item.file_target(&app.library, args.episodes).await?;

    confirm_deletion(&app.deleter.files_of(target).await?, args.yes)?;
    let deleted = app.deleter.delete(target).await?;
    app.deliver_events().await?;
    let mut out = anstream::stdout();
    for file in deleted {
        writeln!(out, "Deleted {}", file.path.display())?;
    }
    Ok(())
}
