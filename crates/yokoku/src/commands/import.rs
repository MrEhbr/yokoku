use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use yokoku_domain::ImportId;
use yokoku_media::ImportMode;

use crate::{app::App, config::Config, output::Paint};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct ImportConfig {
    pub mode: Mode,
}

/// How finished downloads reach the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Keep seeding; copy when the library is on another file system
    #[default]
    Hardlink,
    /// Keep seeding from a separate copy
    Copy,
    /// Take the files out of the download
    Move,
}

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List imports that are waiting, running or failed
    List,
    /// Carry out approved imports now
    Run,
    /// Try a failed import again
    Retry { import: ImportId },
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;

    match args.command {
        Command::List => {
            let imports = app.importer.list().await?;
            if imports.is_empty() {
                hint!("No imports waiting.")?;
            }
            for import in imports {
                say!(
                    "{}  {:<9}  {:>3} files  {}",
                    import.id,
                    import.status.tone(),
                    import.rows.len(),
                    import.source.display()
                )?;
                if let Some(error) = &import.error {
                    say!("      {}", error.red())?;
                }
            }
        },
        Command::Run => run_imports(&app).await?,
        Command::Retry { import } => {
            app.importer.retry(import).await?;
            run_imports(&app).await?;
        },
    }
    Ok(())
}

/// Carries out approved imports, delivers their events and reports each one.
pub async fn run_imports(app: &App) -> Result<()> {
    for import in app.importer.run_pending().await? {
        match &import.error {
            None => success!("Imported {}", import.source.display())?,
            Some(error) => failure!("Import of {} failed: {error}", import.source.display())?,
        }
    }
    app.deliver_events().await
}

impl From<Mode> for ImportMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Hardlink => Self::HardLink,
            Mode::Copy => Self::Copy,
            Mode::Move => Self::Move,
        }
    }
}
