use anyhow::Result;
use clap::Parser;
use yokoku_config::Config;
use yokoku_domain::title_with_year;

use crate::{app::App, cli::output::Paint};

#[derive(Parser)]
pub struct Args {}

pub async fn run(config: &Config, _args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let missing = app.calendar.missing().await?;

    if missing.series.is_empty() && missing.movies.is_empty() {
        hint!("Nothing missing.")?;
    }
    for series in &missing.series {
        say!("{}  {}", title_with_year(&series.title, series.year).bold(), series.source)?;
        for episode in &series.episodes {
            say!("  {}  {}  {}", episode.reference, episode.air_date, episode.title)?;
        }
    }
    if !missing.movies.is_empty() {
        say!("{}", "Movies".bold())?;
    }
    for movie in &missing.movies {
        say!("  {}  {}", title_with_year(&movie.title, movie.year), movie.source)?;
    }

    Ok(())
}
