use std::io;

use anyhow::Result;
use clap::Parser;
use jiff::civil::Date;
use yokoku_domain::{ItemId, Movie, Series};
use yokoku_library::LibraryStatus;

use crate::{
    app::App,
    commands::{ItemArgs, title_with_year},
    config::Config,
    output::{Paint, Painted},
};

#[derive(Parser)]
pub struct Args {
    #[command(flatten)]
    pub item: ItemArgs,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let today = app.library.today();

    match args.item.resolve(&app.library).await? {
        ItemId::Series(id) => print_series(&app.library.series(id).await?, today)?,
        ItemId::Movie(id) => print_movie(&app.library.movie(id).await?, today)?,
    }

    Ok(())
}

fn print_series(series: &Series, today: Date) -> io::Result<()> {
    say!(
        "{}  {}  {}  {}  numbering: {}",
        title_with_year(&series.title, series.year).bold(),
        series.source,
        LibraryStatus::Series(series.status(today)).tone(),
        monitored(series.monitored),
        series.numbering,
    )?;
    match series.next_episode(today) {
        Some((reference, episode)) => {
            say!("Next      {reference}  {}  {}", date_label(episode.air_date), episode.title)?
        },
        None => say!("Next      -")?,
    }
    match series.last_aired(today) {
        Some((reference, episode)) => {
            say!("Last      {reference}  {}  {}", date_label(episode.air_date), episode.file_status(today).tone())?
        },
        None => say!("Last      -")?,
    }

    for season in &series.seasons {
        say!("{}  {}", format!("Season {}", season.number).bold(), monitored(season.monitored))?;
        for episode in &season.episodes {
            say!(
                "  S{:02}E{:02}  {:<10}  {:<10}  {:<11}  {}",
                season.number,
                episode.number,
                date_label(episode.air_date),
                episode.file_status(today).tone(),
                monitored(episode.monitored),
                episode.title,
            )?;
        }
    }
    Ok(())
}

fn print_movie(movie: &Movie, today: Date) -> io::Result<()> {
    say!(
        "{}  {}  {}  {}",
        title_with_year(&movie.title, movie.year).bold(),
        movie.source,
        LibraryStatus::Movie(movie.status(today)).tone(),
        monitored(movie.monitored),
    )?;
    say!("Cinema    {}", date_label(movie.releases.cinema))?;
    say!("Digital   {}", date_label(movie.releases.digital))?;
    say!("Physical  {}", date_label(movie.releases.physical))?;
    say!("File      {}", movie.file_status(today).tone())
}

fn date_label(date: Option<Date>) -> String {
    date.map_or_else(|| "-".to_owned(), |date| date.to_string())
}

fn monitored(monitored: bool) -> Painted<&'static str> {
    if monitored { "monitored" } else { "unmonitored" }.tone()
}
