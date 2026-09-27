use std::io::Write;

use anyhow::Result;
use clap::Parser;
use owo_colors::OwoColorize;

use crate::{app::App, commands::title_with_year, config::Config};

#[derive(Parser)]
pub struct Args {}

pub async fn run(config: &Config, _args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let mut out = anstream::stdout();
    let missing = app.schedule.missing().await?;

    if missing.series.is_empty() && missing.movies.is_empty() {
        writeln!(out, "Nothing missing.")?;
    }
    for series in &missing.series {
        writeln!(out, "{}  {}", title_with_year(&series.title, series.year).bold(), series.source)?;
        for episode in &series.episodes {
            writeln!(out, "  {}  {}  {}", episode.reference, episode.air_date, episode.title)?;
        }
    }
    if !missing.movies.is_empty() {
        writeln!(out, "{}", "Movies".bold())?;
    }
    for movie in &missing.movies {
        writeln!(out, "  {}  {}", title_with_year(&movie.title, movie.year), movie.source)?;
    }

    Ok(())
}
