use anyhow::Result;
use clap::Parser;
use jiff::{ToSpan, civil::Date};
use tracing::debug;
use yokoku_config::{CalendarConfig, Config};
use yokoku_library::{CalendarRelease, month_of, week_of};

use crate::{app::App, cli::output::Paint};

#[derive(Parser)]
pub struct Args {
    /// Show the whole month instead of the week
    #[arg(long, conflicts_with = "days")]
    pub month: bool,

    /// Show this many days from the date instead of the week, overriding `calendar.days`
    #[arg(long, short = 'd')]
    pub days: Option<u16>,

    /// A day in the period to show, e.g. `2026-10-01`; today when omitted
    #[arg(long)]
    pub date: Option<Date>,
}

impl Args {
    /// Command flags take precedence over the resolved configuration.
    fn apply_overrides(&self, config: &CalendarConfig) -> CalendarConfig {
        let mut resolved = config.clone();

        if let Some(days) = self.days {
            resolved.days = Some(days);
        }

        resolved
    }
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let settings = args.apply_overrides(&config.calendar);
    debug!(?settings, "resolved command settings");

    let app = App::open(config).await?;
    let day = args.date.unwrap_or_else(|| app.calendar.today());
    let (from, to) = match settings.days {
        _ if args.month => month_of(day),
        Some(days) => (day, day + i64::from(days).days()),
        None => week_of(day),
    };
    let entries = app.calendar.entries(from, to).await?;

    say!("{from} to {to}")?;
    if entries.is_empty() {
        hint!("Nothing scheduled.")?;
    }
    let mut current = None;
    for entry in &entries {
        if current != Some(entry.date) {
            say!("{}", entry.date.strftime("%Y-%m-%d %a").bold())?;
            current = Some(entry.date);
        }
        let release = match &entry.release {
            CalendarRelease::Episode { reference, title } => format!("{reference}  {title}"),
            CalendarRelease::Movie(kind) => format!("{kind} release"),
        };
        say!("  {:<30} {:<40} {}", entry.title, release, entry.status.tone())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::flag_wins(Some(3), Some(3))]
    #[case::config_without_a_flag(None, Some(30))]
    fn flags_override_the_configuration(#[case] days: Option<u16>, #[case] expected: Option<u16>) {
        let args = Args { month: false, days, date: None };

        assert_eq!(args.apply_overrides(&CalendarConfig { days: Some(30) }).days, expected);
    }
}
