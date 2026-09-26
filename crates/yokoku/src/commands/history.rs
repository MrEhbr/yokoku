use std::io::{self, Write};

use anyhow::Result;
use clap::Parser;
use yokoku_domain::ExternalId;
use yokoku_events::{DeleteReason, Event};

use crate::{
    app::App,
    commands::{ItemArgs, Kind},
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    /// Item type; the whole library's history when omitted
    #[arg(requires = "source")]
    pub kind: Option<Kind>,

    /// Source id, e.g. `tmdb:1396`
    #[arg(requires = "kind")]
    pub source: Option<ExternalId>,

    /// Most entries to show
    #[arg(long, short = 'n', default_value_t = 50)]
    pub limit: usize,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let item = match args.kind.zip(args.source) {
        Some((kind, source)) => Some(ItemArgs { kind, source }.resolve(&app.library).await?),
        None => None,
    };
    let time_zone = config.clock.time_zone()?;
    let mut out = io::stdout();

    let entries = app.history.page(item, None, args.limit).await?;
    if entries.is_empty() {
        writeln!(out, "No history.")?;
    }
    for recorded in entries {
        let at = recorded.occurred_at.to_zoned(time_zone.clone()).strftime("%Y-%m-%d %H:%M");
        let mut lines = describe(&recorded.event).into_iter();
        writeln!(out, "{at}  {}", lines.next().unwrap_or_default())?;
        for line in lines {
            writeln!(out, "                  {line}")?;
        }
    }
    Ok(())
}

/// One line, followed by one line per file where an event holds several.
fn describe(event: &Event) -> Vec<String> {
    let files = |verb: &str, files: &[yokoku_events::LinkedFile]| match files {
        [file] => vec![format!("{verb} {}", file.path.display())],
        files => std::iter::once(format!("{verb} {} files", files.len()))
            .chain(files.iter().map(|file| file.path.display().to_string()))
            .collect(),
    };
    match event {
        Event::SeriesAdded { title, .. } => vec![format!("Added series {title}")],
        Event::MovieAdded { title, .. } => vec![format!("Added movie {title}")],
        Event::SeriesRemoved { title, delete_files, .. } => vec![removed("series", title, *delete_files)],
        Event::MovieRemoved { title, delete_files, .. } => vec![removed("movie", title, *delete_files)],
        Event::FilesFound { files: found } => files("Found", found),
        Event::FilesImported { files: imported, .. } => files("Imported", imported),
        Event::FileDeleted { path, reason, recycled, .. } => {
            let verb = if *recycled { "Recycled" } else { "Deleted" };
            let reason = match reason {
                DeleteReason::External => "gone from disk",
                DeleteReason::Replaced => "replaced by an import",
                DeleteReason::User => "by request",
                DeleteReason::ItemRemoved => "its item was removed",
            };
            vec![format!("{verb} {} ({reason})", path.display())]
        },
        Event::FileRenamed { from, to, .. } => {
            vec![format!("Renamed {}", from.display()), format!("-> {}", to.display())]
        },
        Event::ImportNeedsReview { source, .. } => vec![format!("Import of {} needs review", source.display())],
        Event::ImportFailed { source, reason, .. } => vec![format!("Import of {} failed: {reason}", source.display())],
        Event::TorrentAdded { name, .. } => vec![format!("Added torrent {name}")],
        Event::DownloadCompleted { name, .. } => vec![format!("Finished downloading {name}")],
        Event::TorrentRemoved { name, .. } => vec![format!("Removed torrent {name} after seeding")],
    }
}

fn removed(kind: &str, title: &str, delete_files: bool) -> String {
    match delete_files {
        true => format!("Removed {kind} {title} and its files"),
        false => format!("Removed {kind} {title}"),
    }
}
