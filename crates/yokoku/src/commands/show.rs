use std::io::{self, Write};

use anyhow::Result;
use clap::Parser;
use jiff::civil::Date;
use owo_colors::OwoColorize;
use yokoku_domain::{ItemId, Movie, Numbering, Series};
use yokoku_library::LibraryStatus;

use crate::{
    app::App,
    commands::{
        ItemArgs,
        label::{Label, monitored},
        title_with_year,
    },
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    #[command(flatten)]
    pub item: ItemArgs,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let today = app.library.today();
    let mut out = anstream::stdout();

    match args.item.resolve(&app.library).await? {
        ItemId::Series(id) => print_series(&mut out, &app.library.series(id).await?, today)?,
        ItemId::Movie(id) => print_movie(&mut out, &app.library.movie(id).await?, today)?,
    }

    Ok(())
}

fn print_series(out: &mut impl Write, series: &Series, today: Date) -> io::Result<()> {
    let numbering = match series.numbering {
        Numbering::Standard => "standard",
        Numbering::Absolute => "absolute",
    };
    writeln!(
        out,
        "{}  {}  {}  {}  numbering: {numbering}",
        title_with_year(&series.title, series.year).bold(),
        series.source,
        LibraryStatus::Series(series.status(today)).label(),
        monitored(series.monitored),
    )?;
    match series.next_episode(today) {
        Some((reference, episode)) => {
            writeln!(out, "Next      {reference}  {}  {}", date_label(episode.air_date), episode.title)?
        },
        None => writeln!(out, "Next      -")?,
    }
    match series.last_aired(today) {
        Some((reference, episode)) => writeln!(
            out,
            "Last      {reference}  {}  {}",
            date_label(episode.air_date),
            episode.file_status(today).label()
        )?,
        None => writeln!(out, "Last      -")?,
    }

    for season in &series.seasons {
        writeln!(out, "{}  {}", format!("Season {}", season.number).bold(), monitored(season.monitored))?;
        for episode in &season.episodes {
            writeln!(
                out,
                "  S{:02}E{:02}  {:<10}  {:<10}  {:<11}  {}",
                season.number,
                episode.number,
                date_label(episode.air_date),
                episode.file_status(today).label(),
                monitored(episode.monitored),
                episode.title,
            )?;
        }
    }
    Ok(())
}

fn print_movie(out: &mut impl Write, movie: &Movie, today: Date) -> io::Result<()> {
    writeln!(
        out,
        "{}  {}  {}  {}",
        title_with_year(&movie.title, movie.year).bold(),
        movie.source,
        LibraryStatus::Movie(movie.status(today)).label(),
        monitored(movie.monitored),
    )?;
    writeln!(out, "Cinema    {}", date_label(movie.releases.cinema))?;
    writeln!(out, "Digital   {}", date_label(movie.releases.digital))?;
    writeln!(out, "Physical  {}", date_label(movie.releases.physical))?;
    writeln!(out, "File      {}", movie.file_status(today).label())
}

fn date_label(date: Option<Date>) -> String {
    date.map_or_else(|| "-".to_owned(), |date| date.to_string())
}
