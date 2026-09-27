use std::path::{self, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use yokoku_config::Config;

use crate::app::App;

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Add a folder that holds series or movies
    Add { kind: RootKind, path: PathBuf },
    /// List root folders
    List,
    /// Remove a root folder; refused while series or movies belong to it
    Remove { path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RootKind {
    Series,
    Movies,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;

    match args.command {
        Command::Add { kind, path } => {
            let root = app.roots.add(kind.into(), &path::absolute(path)?).await?;
            success!("Added {} root {}", root.kind, root.path.display())?;
        },
        Command::List => {
            let roots = app.roots.list().await?;
            if roots.is_empty() {
                hint!("No root folders; add one with `yokoku root add`.")?;
            }
            for root in roots {
                say!("{:<7} {}", root.kind, root.path.display())?;
            }
        },
        Command::Remove { path } => {
            let path = path::absolute(path)?;
            app.roots.remove(&path).await?;
            success!("Removed root {}", path.display())?;
        },
    }

    Ok(())
}

impl From<RootKind> for yokoku_media::RootKind {
    fn from(kind: RootKind) -> Self {
        match kind {
            RootKind::Series => Self::Series,
            RootKind::Movies => Self::Movies,
        }
    }
}
