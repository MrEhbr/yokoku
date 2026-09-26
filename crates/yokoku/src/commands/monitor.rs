use anyhow::{Result, bail};
use clap::Parser;
use yokoku_domain::EpisodeRef;
use yokoku_library::ItemId;

use crate::{app::App, commands::ItemArgs, config::Config};

#[derive(Parser)]
pub struct Args {
    #[command(flatten)]
    pub item: ItemArgs,

    /// Only this season
    #[arg(long)]
    pub season: Option<u16>,

    /// Only this episode of `--season`
    #[arg(long, requires = "season")]
    pub episode: Option<u16>,

    /// Stop monitoring instead
    #[arg(long)]
    pub off: bool,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let monitored = !args.off;

    match (args.item.resolve(&app.library).await?, args.season, args.episode) {
        (ItemId::Series(id), None, _) => app.library.set_series_monitored(id, monitored).await?,
        (ItemId::Series(id), Some(season), None) => app.library.set_season_monitored(id, season, monitored).await?,
        (ItemId::Series(id), Some(season), Some(episode)) => {
            app.library.set_episode_monitored(id, EpisodeRef { season, episode }, monitored).await?
        },
        (ItemId::Movie(id), None, _) => app.library.set_movie_monitored(id, monitored).await?,
        (ItemId::Movie(_), Some(_), _) => bail!("--season and --episode apply to series only"),
    }

    Ok(())
}
