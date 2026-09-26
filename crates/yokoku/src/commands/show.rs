use std::io::{self, Write};

use anyhow::Result;
use clap::Parser;
use jiff::civil::Date;
use yokoku_domain::{Movie, Numbering, Series};
use yokoku_library::{ItemId, LibraryStatus};

use crate::{
    app::App,
    commands::{ItemArgs, file_status_label, status_label, title_with_year},
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
    let mut out = io::stdout();

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
        title_with_year(&series.title, series.year),
        series.source,
        status_label(LibraryStatus::Series(series.status(today))),
        monitored_label(series.monitored),
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
            file_status_label(episode.file_status(today))
        )?,
        None => writeln!(out, "Last      -")?,
    }

    for season in &series.seasons {
        writeln!(out, "Season {}  {}", season.number, monitored_label(season.monitored))?;
        for episode in &season.episodes {
            writeln!(
                out,
                "  S{:02}E{:02}  {:<10}  {:<10}  {:<11}  {}",
                season.number,
                episode.number,
                date_label(episode.air_date),
                file_status_label(episode.file_status(today)),
                monitored_label(episode.monitored),
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
        title_with_year(&movie.title, movie.year),
        movie.source,
        status_label(LibraryStatus::Movie(movie.status(today))),
        monitored_label(movie.monitored),
    )?;
    writeln!(out, "Cinema    {}", date_label(movie.releases.cinema))?;
    writeln!(out, "Digital   {}", date_label(movie.releases.digital))?;
    writeln!(out, "Physical  {}", date_label(movie.releases.physical))?;
    writeln!(out, "File      {}", file_status_label(movie.file_status(today)))
}

fn monitored_label(monitored: bool) -> &'static str {
    if monitored { "monitored" } else { "unmonitored" }
}

fn date_label(date: Option<Date>) -> String {
    date.map_or_else(|| "-".to_owned(), |date| date.to_string())
}
