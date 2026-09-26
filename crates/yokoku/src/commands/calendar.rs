use std::io::{self, Write};

use anyhow::Result;
use clap::Parser;
use jiff::civil::Date;
use yokoku_library::{CalendarEntry, CalendarRelease, month_of, week_of};

use crate::{
    app::App,
    commands::{file_status_label, release_label},
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    /// Show the whole month instead of the week
    #[arg(long)]
    pub month: bool,

    /// A day in the period to show, e.g. `2026-10-01`; today when omitted
    #[arg(long)]
    pub date: Option<Date>,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;
    let day = args.date.unwrap_or_else(|| app.schedule.today());
    let (from, to) = if args.month { month_of(day) } else { week_of(day) };

    let mut out = io::stdout();
    writeln!(out, "{from} to {to}")?;
    print_calendar(&mut out, &app.schedule.calendar(from, to).await?)?;

    Ok(())
}

/// Entries grouped under their date.
pub fn print_calendar(out: &mut impl Write, entries: &[CalendarEntry]) -> io::Result<()> {
    if entries.is_empty() {
        writeln!(out, "Nothing scheduled.")?;
    }
    let mut current = None;
    for entry in entries {
        if current != Some(entry.date) {
            writeln!(out, "{}", entry.date.strftime("%Y-%m-%d %a"))?;
            current = Some(entry.date);
        }
        let release = match &entry.release {
            CalendarRelease::Episode { reference, title } => format!("{reference}  {title}"),
            CalendarRelease::Movie(kind) => release_label(*kind).to_owned(),
        };
        writeln!(out, "  {:<30} {:<40} {}", entry.title, release, file_status_label(entry.status))?;
    }
    Ok(())
}
