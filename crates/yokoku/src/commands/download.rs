use std::{fs, io::Write, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use yokoku_domain::ExternalId;
use yokoku_downloads::{Download, DownloadState, ports::TorrentSource};

use crate::{
    app::App,
    commands::{ItemArgs, Kind, import::run_imports, item_title, label::Label},
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Check that Transmission can be reached
    Test,
    /// Add a torrent, for a library item or for detection to work out
    Add(AddArgs),
    /// List downloads as of the last sync
    List,
    /// Fetch progress from Transmission and record finished downloads
    Sync,
}

#[derive(clap::Args)]
pub struct AddArgs {
    /// A magnet link or the path of a .torrent file
    pub torrent: String,

    /// Item type the torrent is for
    #[arg(requires = "source")]
    pub kind: Option<Kind>,

    /// Source id, e.g. `tmdb:1396`
    #[arg(requires = "kind")]
    pub source: Option<ExternalId>,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let mut out = anstream::stdout();

    match args.command {
        Command::Test => {
            let version = app.downloads.test_connection().await?;
            writeln!(out, "Connected to {version}")?;
        },
        Command::Add(args) => {
            let item = match ItemArgs::optional(args.kind, args.source) {
                Some(item) => Some(item.resolve(&app.library).await?),
                None => None,
            };
            let download = app.downloads.add(&torrent_source(&args.torrent)?, item).await?;
            app.deliver_events().await?;
            match download.completed_at {
                Some(_) => writeln!(out, "Added {}; it is already complete", download.name)?,
                None => writeln!(out, "Added {}", download.name)?,
            }
            run_imports(&app, &mut out).await?;
        },
        Command::List => {
            let downloads = app.downloads.list().await?;
            if downloads.is_empty() {
                writeln!(out, "No downloads.")?;
            }
            for download in &downloads {
                let item = match download.item {
                    Some(item) => item_title(&app.library, item).await,
                    None => "-".into(),
                };
                writeln!(
                    out,
                    "{:<50} {:>3}%  {:<11}  {:<24}  {item}",
                    download.name,
                    download.percent_done(),
                    download.label(),
                    progress_label(download)
                )?;
            }
        },
        Command::Sync => {
            let report = app.downloads.sync().await?;
            app.deliver_events().await?;
            writeln!(out, "Synced {} downloads; {} finished", report.synced, report.completed.len())?;
            if report.removed > 0 {
                writeln!(out, "{} are no longer in Transmission", report.removed)?;
            }
            if report.picked_up > 0 {
                writeln!(out, "Picked up {} torrents added in Transmission", report.picked_up)?;
            }
            if report.cleaned_up > 0 {
                writeln!(out, "{} removed from Transmission after seeding", report.cleaned_up)?;
            }
            run_imports(&app, &mut out).await?;
            let waiting = app.review.pending().await?.len();
            if waiting > 0 {
                writeln!(out, "{waiting} imports wait for review; see `yokoku review list`")?;
            }
        },
    }

    Ok(())
}

fn torrent_source(torrent: &str) -> Result<TorrentSource> {
    if torrent.starts_with("magnet:") {
        return Ok(TorrentSource::Magnet(torrent.to_owned()));
    }
    let path = PathBuf::from(torrent);
    let bytes = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(TorrentSource::File(bytes))
}

/// Rate and time left while downloading; the error otherwise, if any.
fn progress_label(download: &Download) -> String {
    match (&download.status.error, download.status.state) {
        (Some(error), _) => error.clone(),
        (None, DownloadState::Downloading) => {
            let rate = format!("{:.1} MB/s", download.status.download_rate as f64 / 1_000_000.0);
            match download.status.eta {
                Some(seconds) => format!("{rate}, {}h {:02}m left", seconds / 3600, seconds % 3600 / 60),
                None => rate,
            }
        },
        (None, _) => String::new(),
    }
}
