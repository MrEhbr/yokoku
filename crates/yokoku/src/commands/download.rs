use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use yokoku_domain::{ExternalId, ItemId};
use yokoku_downloads::{Download, DownloadState, ports::TorrentSource};
use yokoku_library::Library;

use crate::{
    app::App,
    commands::{ItemArgs, Kind, title_with_year},
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
    let mut out = io::stdout();

    match args.command {
        Command::Test => {
            let version = app.downloads.test_connection().await?;
            writeln!(out, "Connected to {version}")?;
        },
        Command::Add(args) => {
            let item = match args.kind.zip(args.source) {
                Some((kind, source)) => Some(ItemArgs { kind, source }.resolve(&app.library).await?),
                None => None,
            };
            let download = app.downloads.add(&torrent_source(&args.torrent)?, item).await?;
            app.deliver_events().await?;
            match download.completed_at {
                Some(_) => writeln!(out, "Added {}; it is already complete", download.name)?,
                None => writeln!(out, "Added {}", download.name)?,
            }
        },
        Command::List => {
            let downloads = app.downloads.list().await?;
            if downloads.is_empty() {
                writeln!(out, "No downloads.")?;
            }
            for download in &downloads {
                let item = item_label(&app.library, download.item).await;
                writeln!(
                    out,
                    "{:<50} {:>3}%  {:<11}  {:<24}  {item}",
                    download.name,
                    download.percent_done(),
                    state_label(download),
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

async fn item_label(library: &Library, item: Option<ItemId>) -> String {
    match item {
        None => "-".into(),
        Some(ItemId::Series(id)) => library
            .series(id)
            .await
            .map_or_else(|_| "removed series".into(), |series| title_with_year(&series.title, series.year)),
        Some(ItemId::Movie(id)) => library
            .movie(id)
            .await
            .map_or_else(|_| "removed movie".into(), |movie| title_with_year(&movie.title, movie.year)),
    }
}

fn state_label(download: &Download) -> &'static str {
    if download.status.error.is_some() {
        return "error";
    }
    match download.status.state {
        DownloadState::Queued => "queued",
        DownloadState::Checking => "checking",
        DownloadState::Downloading => "downloading",
        DownloadState::Seeding => "seeding",
        DownloadState::Stopped if download.completed_at.is_some() => "finished",
        DownloadState::Stopped => "paused",
        DownloadState::Removed => "removed",
    }
}

/// Rate and time left while downloading; the error otherwise, if any.
fn progress_label(download: &Download) -> String {
    if let Some(error) = &download.status.error {
        return error.clone();
    }
    if download.status.state != DownloadState::Downloading {
        return String::new();
    }
    let rate = format!("{:.1} MB/s", download.status.download_rate as f64 / 1_000_000.0);
    match download.status.eta {
        Some(seconds) => format!("{rate}, {}h {:02}m left", seconds / 3600, seconds % 3600 / 60),
        None => rate,
    }
}
