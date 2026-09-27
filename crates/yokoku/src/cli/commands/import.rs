use anyhow::Result;
use clap::{Parser, Subcommand};
use yokoku_config::Config;
use yokoku_domain::ImportId;

use crate::{app::App, cli::output::Paint};

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
