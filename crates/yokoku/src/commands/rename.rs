use std::{io, path::Path};

use anyhow::{Result, bail};
use clap::Parser;
use yokoku_domain::{ExternalId, ItemId};
use yokoku_media::{Rename, RenameScope, SkipReason, Skipped};

use crate::{
    app::App,
    commands::{ItemArgs, Kind},
    config::Config,
    output::Paint,
};

#[derive(Parser)]
pub struct Args {
    /// Item type; the whole library is renamed when omitted
    #[arg(requires = "source")]
    pub kind: Option<Kind>,

    /// Source id, e.g. `tmdb:1396`
    #[arg(requires = "kind")]
    pub source: Option<ExternalId>,

    /// Move the files; without it the moves are only listed
    #[arg(long)]
    pub apply: bool,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let scope = match ItemArgs::optional(args.kind, args.source) {
        None => RenameScope::All,
        Some(item) => match item.resolve(&app.library).await? {
            ItemId::Series(id) => RenameScope::Series(id),
            ItemId::Movie(id) => RenameScope::Movie(id),
        },
    };

    if !args.apply {
        let plan = app.renamer.preview(scope).await?;
        if plan.renames.is_empty() {
            hint!("Nothing to rename.")?;
        }
        for rename in &plan.renames {
            print_rename(rename)?;
        }
        print_skipped(&plan.skipped)?;
        if !plan.renames.is_empty() {
            say!("Run with --apply to rename {} files.", plan.renames.len())?;
        }
        return Ok(());
    }

    let report = app.renamer.apply(scope).await?;
    app.deliver_events().await?;
    success!("Renamed {} files", report.renamed.len())?;
    print_skipped(&report.skipped)?;
    for failure in &report.failed {
        failure!("Failed {}: {}", failure.path.display(), failure.error)?;
    }
    if !report.failed.is_empty() {
        bail!("{} files could not be renamed", report.failed.len());
    }
    Ok(())
}

fn print_rename(rename: &Rename) -> io::Result<()> {
    let relative = |path: &Path| path.strip_prefix(&rename.root).unwrap_or(path).display().to_string();
    let moves = std::iter::once(&rename.video).chain(&rename.subtitles).filter(|step| step.from != step.to);
    for step in moves {
        say!("{}\n  {} {}", relative(&step.from), "->".dimmed(), relative(&step.to).green())?;
    }
    Ok(())
}

fn print_skipped(skipped: &[Skipped]) -> io::Result<()> {
    for skipped in skipped {
        let reason = match skipped.reason {
            SkipReason::OutsideRoots => "not in a root folder",
            SkipReason::NotInLibrary => "its item is no longer in the library",
            SkipReason::SharedTarget => "another file would get the same name",
        };
        caution!("Skipped {}: {reason}", skipped.path.display())?;
    }
    Ok(())
}
