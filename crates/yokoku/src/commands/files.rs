use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use yokoku_media::{MediaError, ports::ProbeError};

use crate::{app::App, commands::ItemArgs, config::Config, output::Paint};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct FilesConfig {
    /// `ffprobe` on the `PATH`, or a path to it.
    pub ffprobe: PathBuf,
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self { ffprobe: PathBuf::from("ffprobe") }
    }
}

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show the files of a series or movie with their resolution and languages
    Show(ItemArgs),
    /// Read the details of every file not read yet, with ffprobe
    Probe,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;

    match args.command {
        Command::Show(item) => {
            let item = item.resolve(&app.library).await?;
            let files = app.prober.details(item).await?;
            if files.is_empty() {
                hint!("No files.")?;
            }
            for details in &files {
                say!("{}", details.file.path.display().bold())?;
                let size = format!("{:.1} GB", details.file.size as f64 / 1_000_000_000.0);
                match &details.info {
                    None => say!("  {size}, not probed yet; run `yokoku files probe`")?,
                    Some(info) => {
                        let minutes = info.duration.map(|duration| duration.as_secs() / 60);
                        let duration = minutes.map(|minutes| format!("{}h {:02}m", minutes / 60, minutes % 60));
                        let video = info.video.as_ref().map(ToString::to_string);
                        let summary: Vec<String> = [Some(size), duration, video].into_iter().flatten().collect();
                        say!("  {}", summary.join(", "))?;
                        if !info.audio.is_empty() {
                            let audio: Vec<String> = info.audio.iter().map(ToString::to_string).collect();
                            say!("  Audio      {}", audio.join(", "))?;
                        }
                    },
                }
                let inside: Vec<String> =
                    details.info.iter().flat_map(|info| &info.subtitles).map(ToString::to_string).collect();
                let beside: Vec<String> = details.subtitle_files.iter().map(ToString::to_string).collect();
                let parts: Vec<String> = [(inside, "in the file"), (beside, "beside it")]
                    .into_iter()
                    .filter(|(languages, _)| !languages.is_empty())
                    .map(|(languages, place)| format!("{} {place}", languages.join(", ")))
                    .collect();
                if !parts.is_empty() {
                    say!("  Subtitles  {}", parts.join("; "))?;
                }
            }
        },
        Command::Probe => {
            let report = match app.prober.probe_missing().await {
                Err(MediaError::Probe(ProbeError::Missing)) => {
                    let program = config.files.ffprobe.display();
                    bail!("{program} is not installed; set [files] ffprobe to its path");
                },
                result => result.context("Failed to probe files")?,
            };
            success!("Probed {} files", report.probed)?;
            for (path, reason) in &report.failed {
                failure!("Could not probe {}: {reason}", path.display())?;
            }
        },
    }
    Ok(())
}
