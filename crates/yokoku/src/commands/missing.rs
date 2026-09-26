use anyhow::Result;
use clap::Parser;

use crate::{app::App, commands::title_with_year, config::Config};

#[derive(Parser)]
pub struct Args {}

pub async fn run(config: &Config, _args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let missing = app.schedule.missing().await?;

    if missing.series.is_empty() && missing.movies.is_empty() {
        println!("Nothing missing.");
    }
    for series in &missing.series {
        println!("{}  {}", title_with_year(&series.title, series.year), series.source);
        for episode in &series.episodes {
            println!("  {}  {}  {}", episode.reference, episode.air_date, episode.title);
        }
    }
    if !missing.movies.is_empty() {
        println!("Movies");
    }
    for movie in &missing.movies {
        println!("  {}  {}", title_with_year(&movie.title, movie.year), movie.source);
    }

    Ok(())
}
