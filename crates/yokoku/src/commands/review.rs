use anyhow::Result;
use clap::{Parser, Subcommand};
use yokoku_domain::{EpisodeSpan, FileTarget, ImportId, ItemId};
use yokoku_media::Approval;

use crate::{
    app::App,
    commands::{ItemArgs, import::run_imports},
    config::Config,
    output::Paint,
};

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List imports waiting for review
    List,
    /// Show the files of an import and what they were matched to
    Show { import: ImportId },
    /// Match a file to episodes of a series or to a movie
    Match(MatchArgs),
    /// Skip a file; it is left where it is and not offered again
    Skip { import: ImportId, row: usize },
    /// Let a downloaded file replace the library file of its episode or movie
    Replace { import: ImportId, row: usize },
    /// Link every matched file and finish the import
    Approve { import: ImportId },
}

#[derive(clap::Args)]
pub struct MatchArgs {
    pub import: ImportId,
    /// Row number, as shown by `review show`
    pub row: usize,
    #[command(flatten)]
    pub item: ItemArgs,
    /// Episodes the file holds, e.g. `S01E02` or `S01E01-E03`; series only
    pub episodes: Option<EpisodeSpan>,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;

    match args.command {
        Command::List => {
            let pending = app.reviewer.pending().await?;
            if pending.is_empty() {
                hint!("Nothing to review.")?;
            }
            for import in pending {
                say!("{}  {:>3} files  {}", import.id, import.rows.len(), import.source.display())?;
            }
        },
        Command::Show { import } => {
            let review = app.reviewer.get(import).await?;
            say!("{}", review.source.display())?;
            for (number, row) in (1..).zip(&review.rows) {
                let file = row.row.path.strip_prefix(&review.source).unwrap_or(&row.row.path);
                if row.row.skipped {
                    say!("{number:>3}  {:<50}  skipped", file.display())?;
                    continue;
                }
                let target = match row.row.target {
                    None => "-".into(),
                    Some(FileTarget::Episodes { series, span }) => {
                        format!("{} {span}", app.title(ItemId::Series(series)).await)
                    },
                    Some(FileTarget::Movie(movie)) => app.title(ItemId::Movie(movie)).await,
                };
                let replaces = row.row.replace.then(|| "replaces the library file".to_owned());
                let conflicts = row.conflicts.iter().map(|conflict| conflict.yellow().to_string());
                let details: Vec<String> =
                    [row.row.confidence.tone().to_string()].into_iter().chain(replaces).chain(conflicts).collect();
                say!("{number:>3}  {:<50}  {target:<40}  {}", file.display(), details.join(", "))?;
            }
        },
        Command::Match(args) => {
            let target = args.item.file_target(&app.library, args.episodes).await?;
            app.reviewer.match_row(args.import, args.row, target).await?;
            success!("Matched row {}", args.row)?;
        },
        Command::Skip { import, row } => {
            app.reviewer.skip_row(import, row).await?;
            say!("Skipped row {row}")?;
        },
        Command::Replace { import, row } => {
            app.reviewer.replace_row(import, row).await?;
            say!("Row {row} replaces the library file")?;
        },
        Command::Approve { import } => match app.reviewer.approve(import).await? {
            Approval::Linked(files) => {
                app.deliver_events().await?;
                success!("Linked {} files", files.len())?;
            },
            Approval::Queued => run_imports(&app).await?,
        },
    }

    Ok(())
}
