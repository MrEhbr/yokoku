use std::str::FromStr;

use jiff::{Zoned, tz::TimeZone};
use jiff_cron::Schedule;
use serde::{Deserialize, Serialize};
use yokoku_core::jobs::Schedules;

use crate::config::Settings;

/// A cron schedule with seconds, such as `*/30 * * * * *` for every 30 seconds; written as its
/// expression.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Cron {
    expression: String,
    schedule: Schedule,
}

impl TryFrom<String> for Cron {
    type Error = InvalidSchedule;

    fn try_from(expression: String) -> Result<Self, Self::Error> {
        match Schedule::from_str(&expression) {
            Ok(schedule) => Ok(Self { expression, schedule }),
            Err(reason) => Err(InvalidSchedule { expression, reason }),
        }
    }
}

impl Cron {
    /// The next tick after now, in `time_zone`.
    fn next_tick(&self, time_zone: TimeZone) -> Option<Zoned> {
        self.schedule.upcoming(time_zone).next()
    }
}

impl From<Cron> for String {
    fn from(cron: Cron) -> Self {
        cron.expression
    }
}

impl PartialEq for Cron {
    fn eq(&self, other: &Self) -> bool {
        self.expression == other.expression
    }
}

impl Eq for Cron {}

/// Each job's schedule, under the job's name in snake case.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ScheduleSettings {
    /// Cron schedule with seconds for syncing downloads.
    pub sync_downloads: Cron,
    /// Cron schedule with seconds for syncing downloads while one is queued or downloading.
    pub sync_active_downloads: Cron,
    /// Cron schedule with seconds for carrying out approved imports.
    pub execute_imports: Cron,
    /// Cron schedule with seconds for checking whether Jellyfin should rescan.
    pub rescan_media_server: Cron,
    /// Cron schedule with seconds for syncing watched files from Jellyfin.
    pub sync_watched: Cron,
    /// Cron schedule with seconds for refreshing ratings.
    pub refresh_ratings: Cron,
    /// Cron schedule with seconds for refreshing the items due for it.
    pub refresh_metadata: Cron,
    /// Cron schedule with seconds for scanning root folders for outside changes.
    pub scan_library: Cron,
}

impl ScheduleSettings {
    /// The schedule of the job named `job`, such as `sync-watched` for `sync_watched`.
    pub fn of(&self, job: &str) -> Option<Cron> {
        let schedules = serde_json::to_value(self).ok()?;
        serde_json::from_value(schedules.get(job.replace('-', "_"))?.clone()).ok()
    }
}

impl Default for ScheduleSettings {
    fn default() -> Self {
        let cron = |expression: &str| Cron::try_from(expression.to_owned()).expect("the default schedules parse");
        Self {
            sync_downloads: cron("*/30 * * * * *"),
            sync_active_downloads: cron("*/5 * * * * *"),
            execute_imports: cron("*/5 * * * * *"),
            rescan_media_server: cron("*/10 * * * * *"),
            sync_watched: cron("0 */15 * * * *"),
            refresh_ratings: cron("0 0 6 * * *"),
            refresh_metadata: cron("0 0 */12 * * *"),
            scan_library: cron("0 0 5 * * *"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid schedule {expression:?}: {reason}")]
pub struct InvalidSchedule {
    expression: String,
    reason: jiff_cron::error::Error,
}

/// Reads `[serve]` and the time zone in effect on each call.
impl Schedules for Settings {
    fn next_tick(&self, job: &str) -> Option<Zoned> {
        let config = self.current();
        config.serve.of(job)?.next_tick(config.clock.time_zone())
    }
}

#[cfg(test)]
mod tests {
    use jiff::tz::TimeZone;

    use super::{Cron, ScheduleSettings};

    #[test]
    fn the_default_schedules_parse() {
        ScheduleSettings::default();
    }

    #[test]
    fn a_tick_falls_at_its_time_in_the_given_time_zone() {
        let berlin = TimeZone::get("Europe/Berlin").unwrap();
        let daily = Cron::try_from("0 0 5 * * *".to_owned()).unwrap();

        let tick = daily.next_tick(berlin.clone()).unwrap();

        assert_eq!((tick.hour(), tick.minute(), tick.time_zone()), (5, 0, &berlin));
    }

    #[test]
    fn a_job_finds_its_schedule_under_its_name_in_snake_case() {
        let schedules = ScheduleSettings::default();

        assert_eq!(schedules.of("sync-active-downloads"), Some(schedules.sync_active_downloads.clone()));
        assert_eq!(schedules.of("sync-everything"), None);
    }
}
