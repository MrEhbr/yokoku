use std::io::Write;

use anyhow::Result;
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use yokoku_detect::Conflict;
use yokoku_domain::{EpisodeSpan, FileTarget, ImportId, ItemId};
use yokoku_library::Library;
use yokoku_media::{Approval, ReviewRow};

use crate::{
    app::App,
    commands::{ItemArgs, import::run_imports, item_title, label::Label},
    config::Config,
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
    let mut out = anstream::stdout();

    match args.command {
        Command::List => {
            let pending = app.review.pending().await?;
            if pending.is_empty() {
                writeln!(out, "Nothing to review.")?;
            }
            for import in pending {
                writeln!(out, "{}  {:>3} files  {}", import.id, import.rows.len(), import.source.display())?;
            }
        },
        Command::Show { import } => {
            let review = app.review.get(import).await?;
            writeln!(out, "{}", review.source.display())?;
            for (number, row) in (1..).zip(&review.rows) {
                let file = row.row.path.strip_prefix(&review.source).unwrap_or(&row.row.path);
                let target = target_label(&app.library, row).await;
                writeln!(out, "{number:>3}  {:<50}  {target:<40}  {}", file.display(), details(row))?;
            }
        },
        Command::Match(args) => {
            let target = args.item.file_target(&app.library, args.episodes).await?;
            app.review.match_row(args.import, args.row, target).await?;
            writeln!(out, "Matched row {}", args.row)?;
        },
        Command::Skip { import, row } => {
            app.review.skip_row(import, row).await?;
            writeln!(out, "Skipped row {row}")?;
        },
        Command::Replace { import, row } => {
            app.review.replace_row(import, row).await?;
            writeln!(out, "Row {row} replaces the library file")?;
        },
        Command::Approve { import } => match app.review.approve(import).await? {
            Approval::Linked(files) => {
                app.deliver_events().await?;
                writeln!(out, "Linked {} files", files.len())?;
            },
            Approval::Queued => run_imports(&app, &mut out).await?,
        },
    }

    Ok(())
}

async fn target_label(library: &Library, row: &ReviewRow) -> String {
    if row.row.skipped {
        return "skipped".into();
    }
    match row.row.target {
        None => "-".into(),
        Some(FileTarget::Episodes { series, span }) => {
            format!("{} {span}", item_title(library, ItemId::Series(series)).await)
        },
        Some(FileTarget::Movie(movie)) => item_title(library, ItemId::Movie(movie)).await,
    }
}

fn details(row: &ReviewRow) -> String {
    if row.row.skipped {
        return String::new();
    }
    let conflicts = row.conflicts.iter().map(|conflict| match conflict {
        Conflict::SharedTarget => "same as another row".yellow().to_string(),
        Conflict::AlreadyHasFile => "already has a file".yellow().to_string(),
    });
    let replaces = row.row.replace.then(|| "replaces the library file".to_owned());
    [row.row.confidence.label().to_string()].into_iter().chain(replaces).chain(conflicts).collect::<Vec<_>>().join(", ")
}
