use std::fs;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use yokoku_domain::ExternalId;
use yokoku_downloads::{DownloadState, ports::TorrentSource};

use crate::{
    app::App,
    cli::{
        commands::{ItemArgs, Kind, import::run_imports},
        output::Paint,
    },
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

pub async fn run(app: &App, args: Args) -> Result<()> {
    match args.command {
        Command::Test => {
            let version = app.downloads.test_connection().await?;
            success!("Connected to {version}")?;
        },
        Command::Add(args) => {
            let item = match ItemArgs::optional(args.kind, args.source) {
                Some(item) => Some(item.resolve(&app.library).await?),
                None => None,
            };
            let download = app.downloads.add(&args.torrent()?, item).await?;
            app.deliver_events().await?;
            match download.completed_at {
                Some(_) => success!("Added {}; it is already complete", download.name)?,
                None => success!("Added {}", download.name)?,
            }
            run_imports(app).await?;
        },
        Command::List => {
            let downloads = app.downloads.list().await?;
            if downloads.is_empty() {
                hint!("No downloads.")?;
            }
            for download in &downloads {
                let item = match download.item {
                    Some(item) => app.title(item).await,
                    None => "-".into(),
                };
                let status = &download.status;
                let (state, progress) = match (&status.error, status.state) {
                    (Some(error), _) => ("error", error.clone()),
                    (None, DownloadState::Downloading) => {
                        let rate = format!("{:.1} MB/s", status.download_rate as f64 / 1_000_000.0);
                        let left = status.eta.map(|eta| format!(", {}h {:02}m left", eta / 3600, eta % 3600 / 60));
                        ("downloading", rate + &left.unwrap_or_default())
                    },
                    (None, DownloadState::Stopped) if download.completed_at.is_some() => ("finished", String::new()),
                    (None, DownloadState::Stopped) => ("paused", String::new()),
                    (None, state) => (state.as_str(), String::new()),
                };
                say!(
                    "{:<50} {:>3}%  {:<11}  {progress:<24}  {item}",
                    download.name,
                    download.percent_done(),
                    state.tone()
                )?;
            }
        },
        Command::Sync => {
            let report = app.downloads.sync().await?;
            app.deliver_events().await?;
            success!("Synced {} downloads; {} finished", report.synced, report.completed.len())?;
            if report.removed > 0 {
                say!("{} are no longer in Transmission", report.removed)?;
            }
            if report.picked_up > 0 {
                say!("Picked up {} torrents added in Transmission", report.picked_up)?;
            }
            if report.cleaned_up > 0 {
                say!("{} removed from Transmission after seeding", report.cleaned_up)?;
            }
            run_imports(app).await?;
            let waiting = app.reviewer.pending().await?.len();
            if waiting > 0 {
                caution!("{waiting} imports wait for review; see `yokoku review list`")?;
            }
        },
    }

    Ok(())
}

impl AddArgs {
    fn torrent(&self) -> Result<TorrentSource> {
        if self.torrent.starts_with("magnet:") {
            return Ok(TorrentSource::Magnet(self.torrent.clone()));
        }
        let bytes = fs::read(&self.torrent).with_context(|| format!("Failed to read {}", self.torrent))?;
        Ok(TorrentSource::File(bytes))
    }
}
