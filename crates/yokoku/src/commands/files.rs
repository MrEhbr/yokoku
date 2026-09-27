use std::{
    io::{self, Write},
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use yokoku_domain::SubtitleTags;
use yokoku_media::{FileDetails, MediaError, MediaInfo, ports::ProbeError};

use crate::{app::App, commands::ItemArgs, config::Config};

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
    let mut out = anstream::stdout();

    match args.command {
        Command::Show(item) => {
            let item = item.resolve(&app.library).await?;
            let details = app.prober.details(item).await?;
            if details.is_empty() {
                writeln!(out, "No files.")?;
            }
            for file in &details {
                write_details(&mut out, file)?;
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
            writeln!(out, "Probed {} files", report.probed)?;
            for (path, reason) in &report.failed {
                writeln!(out, "{}", format!("Could not probe {}: {reason}", path.display()).red())?;
            }
        },
    }
    Ok(())
}

fn write_details(out: &mut impl Write, details: &FileDetails) -> io::Result<()> {
    writeln!(out, "{}", details.file.path.display().bold())?;
    let size = format!("{:.1} GB", details.file.size as f64 / 1_000_000_000.0);
    let Some(info) = &details.info else {
        writeln!(out, "  {size}, not probed yet; run `yokoku files probe`")?;
        return write_subtitles(out, None, &details.subtitle_files);
    };
    let mut summary = vec![size];
    summary.extend(info.duration.map(duration));
    summary.extend(info.video.as_ref().map(|video| format!("{}x{} {}", video.width, video.height, video.codec)));
    writeln!(out, "  {}", summary.join(", "))?;
    if !info.audio.is_empty() {
        let audio: Vec<String> = info
            .audio
            .iter()
            .map(|audio| {
                format!("{} {} {}", language(audio.language.as_deref()), audio.codec, channels(audio.channels))
            })
            .collect();
        writeln!(out, "  Audio      {}", audio.join(", "))?;
    }
    write_subtitles(out, Some(info), &details.subtitle_files)
}

fn write_subtitles(out: &mut impl Write, info: Option<&MediaInfo>, files: &[SubtitleTags]) -> io::Result<()> {
    let inside: Vec<String> = info
        .into_iter()
        .flat_map(|info| &info.subtitles)
        .map(|subtitle| tagged(language(subtitle.language.as_deref()), false, subtitle.forced))
        .collect();
    let beside: Vec<String> = files
        .iter()
        .map(|tags| tagged(tags.language.as_deref().unwrap_or("unknown").to_owned(), tags.sdh, tags.forced))
        .collect();
    let parts: Vec<String> = [(inside, "in the file"), (beside, "beside it")]
        .into_iter()
        .filter(|(languages, _)| !languages.is_empty())
        .map(|(languages, place)| format!("{} {place}", languages.join(", ")))
        .collect();
    if !parts.is_empty() {
        writeln!(out, "  Subtitles  {}", parts.join("; "))?;
    }
    Ok(())
}

fn language(code: Option<&str>) -> String {
    code.unwrap_or("unknown").to_owned()
}

fn tagged(language: String, sdh: bool, forced: bool) -> String {
    match (sdh, forced) {
        (false, false) => language,
        (true, false) => format!("{language} (SDH)"),
        (false, true) => format!("{language} (forced)"),
        (true, true) => format!("{language} (SDH, forced)"),
    }
}

fn channels(count: u16) -> String {
    match count {
        1 => "mono".into(),
        2 => "stereo".into(),
        6 => "5.1".into(),
        8 => "7.1".into(),
        count => format!("{count} channels"),
    }
}

fn duration(duration: Duration) -> String {
    let minutes = duration.as_secs() / 60;
    format!("{}h {:02}m", minutes / 60, minutes % 60)
}
