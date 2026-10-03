use std::path::{self, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::app::App;

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Add a folder that holds series or movies
    Add {
        kind: RootKind,
        path: PathBuf,
        /// Shown instead of the path; the folder's name by default
        #[arg(long)]
        name: Option<String>,
    },
    /// List root folders, those from the config file included
    List,
    /// Remove a root folder; refused while series or movies belong to it, and for one in the config file
    Remove { path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RootKind {
    Series,
    Movies,
}

pub async fn run(app: &App, args: Args) -> Result<()> {
    match args.command {
        Command::Add { kind, path, name } => {
            let named = name.is_some();
            let root = app.roots.add(kind.into(), &path::absolute(path)?, name).await?;
            if named {
                success!("Added {} root {} as {}", root.kind, root.path.display(), root.name)?;
            } else {
                success!("Added {} root {}", root.kind, root.path.display())?;
            }
        },
        Command::List => {
            let roots = app.roots.list().await?;
            if roots.is_empty() {
                hint!("No root folders; add one with `yokoku root add`.")?;
            }
            for root in roots {
                let origin = if root.configured { "  (config file)" } else { "" };
                say!("{:<7} {}  {}{origin}", root.kind, root.path.display(), root.name)?;
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

impl From<RootKind> for yokoku_core::media::RootKind {
    fn from(kind: RootKind) -> Self {
        match kind {
            RootKind::Series => Self::Series,
            RootKind::Movies => Self::Movies,
        }
    }
}
