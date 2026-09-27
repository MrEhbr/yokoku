use std::{
    io::Write,
    path::{self, PathBuf},
};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use yokoku_media::RootKind;

use crate::{app::App, commands::label::Label, config::Config};

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Add a folder that holds series or movies
    Add { kind: Kind, path: PathBuf },
    /// List root folders
    List,
    /// Remove a root folder; refused while series or movies belong to it
    Remove { path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Kind {
    Series,
    Movies,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let mut out = anstream::stdout();

    match args.command {
        Command::Add { kind, path } => {
            let root = app.roots.add(kind.into(), &path::absolute(path)?).await?;
            writeln!(out, "Added {} root {}", root.kind.label(), root.path.display())?;
        },
        Command::List => {
            let roots = app.roots.list().await?;
            if roots.is_empty() {
                writeln!(out, "No root folders; add one with `yokoku root add`.")?;
            }
            for root in roots {
                writeln!(out, "{:<7} {}", root.kind.label(), root.path.display())?;
            }
        },
        Command::Remove { path } => {
            let path = path::absolute(path)?;
            app.roots.remove(&path).await?;
            writeln!(out, "Removed root {}", path.display())?;
        },
    }

    Ok(())
}

impl From<Kind> for RootKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Series => Self::Series,
            Kind::Movies => Self::Movies,
        }
    }
}
