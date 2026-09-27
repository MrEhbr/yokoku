use std::path::{self, PathBuf};

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use tracing::debug;
use yokoku_config::{AddConfig, Config};
use yokoku_domain::{self as domain, title_with_year};
use yokoku_media::RootKind;

use crate::{
    app::App,
    cli::{
        commands::{ItemArgs, Kind},
        output::Paint,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MonitorPreset {
    /// Every episode, or the movie
    All,
    /// Episodes that have not aired yet
    Future,
    /// Episodes of the latest season
    LatestSeason,
    /// Nothing
    None,
}

#[derive(Parser)]
pub struct Args {
    #[command(flatten)]
    pub item: ItemArgs,

    /// What to monitor, overriding `add.monitor`; movies accept `all` or `none`
    #[arg(long)]
    pub monitor: Option<MonitorPreset>,

    /// Root folder to add the item to; it must be a root of the item's type
    #[arg(long)]
    pub root: PathBuf,

    /// Name of the item's folder in the root, e.g. an existing folder; defaults to the naming pattern
    #[arg(long)]
    pub folder: Option<String>,
}

impl Args {
    /// Command flags take precedence over the resolved configuration.
    fn apply_overrides(&self, config: &AddConfig) -> AddConfig {
        let mut resolved = config.clone();

        if let Some(monitor) = self.monitor {
            resolved.monitor = monitor.into();
        }

        resolved
    }
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let settings = args.apply_overrides(&config.add);
    debug!(?settings, "resolved command settings");

    let app = App::open(config).await?;
    let metadata = app.metadata()?;
    let root_kind = match args.item.kind {
        Kind::Series => RootKind::Series,
        Kind::Movie => RootKind::Movies,
    };
    let root = app.roots.get(root_kind, &path::absolute(&args.root)?).await?.path;
    match args.item.kind {
        Kind::Series => {
            let series = metadata.add_series(args.item.source, settings.monitor, root, args.folder).await?;
            success!(
                "Added series {} {} in {}",
                title_with_year(&series.title, series.year).bold(),
                series.source,
                series.folder.path().display().italic()
            )?;
        },
        Kind::Movie => {
            let monitored = match settings.monitor {
                domain::MonitorPreset::All => true,
                domain::MonitorPreset::None => false,
                domain::MonitorPreset::Future | domain::MonitorPreset::LatestSeason => {
                    bail!("movies can only be monitored with `all` or `none`")
                },
            };
            let movie = metadata.add_movie(args.item.source, monitored, root, args.folder).await?;
            success!(
                "Added movie {} {} in {}",
                title_with_year(&movie.title, movie.year).bold(),
                movie.source,
                movie.folder.path().display().italic()
            )?;
        },
    }

    app.deliver_events().await
}

impl From<MonitorPreset> for domain::MonitorPreset {
    fn from(monitor: MonitorPreset) -> Self {
        match monitor {
            MonitorPreset::All => Self::All,
            MonitorPreset::Future => Self::Future,
            MonitorPreset::LatestSeason => Self::LatestSeason,
            MonitorPreset::None => Self::None,
        }
    }
}
