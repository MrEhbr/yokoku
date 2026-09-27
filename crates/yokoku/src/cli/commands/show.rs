use anyhow::Result;
use clap::Parser;
use yokoku_domain::ItemId;
use yokoku_library::LibraryStatus;

use crate::{
    app::App,
    cli::{
        commands::{ItemArgs, title_with_year},
        output::{Paint, Painted},
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

    match args.item.resolve(&app.library).await? {
        ItemId::Series(id) => {
            let series = app.library.series(id).await?;
            say!(
                "{}  {}  {}  {}  numbering: {}",
                title_with_year(&series.title, series.year).bold(),
                series.source,
                LibraryStatus::Series(series.status(today)).tone(),
                monitored(series.monitored),
                series.numbering,
            )?;
            match series.next_episode(today) {
                Some((reference, episode)) => say!(
                    "Next      {reference}  {}  {}",
                    episode.air_date.map_or("-".into(), |date| date.to_string()),
                    episode.title
                )?,
                None => say!("Next      -")?,
            }
            match series.last_aired(today) {
                Some((reference, episode)) => say!(
                    "Last      {reference}  {}  {}",
                    episode.air_date.map_or("-".into(), |date| date.to_string()),
                    episode.file_status(today).tone()
                )?,
                None => say!("Last      -")?,
            }
            for season in &series.seasons {
                say!("{}  {}", format!("Season {}", season.number).bold(), monitored(season.monitored))?;
                for episode in &season.episodes {
                    say!(
                        "  S{:02}E{:02}  {:<10}  {:<10}  {:<11}  {}",
                        season.number,
                        episode.number,
                        episode.air_date.map_or("-".into(), |date| date.to_string()),
                        episode.file_status(today).tone(),
                        monitored(episode.monitored),
                        episode.title,
                    )?;
                }
            }
        },
        ItemId::Movie(id) => {
            let movie = app.library.movie(id).await?;
            say!(
                "{}  {}  {}  {}",
                title_with_year(&movie.title, movie.year).bold(),
                movie.source,
                LibraryStatus::Movie(movie.status(today)).tone(),
                monitored(movie.monitored),
            )?;
            let releases = &movie.releases;
            for (kind, date) in
                [("Cinema", releases.cinema), ("Digital", releases.digital), ("Physical", releases.physical)]
            {
                say!("{kind:<10}{}", date.map_or("-".into(), |date| date.to_string()))?;
            }
            say!("File      {}", movie.file_status(today).tone())?;
        },
    }
    Ok(())
}

fn monitored(monitored: bool) -> Painted<&'static str> {
    if monitored { "monitored" } else { "unmonitored" }.tone()
}
