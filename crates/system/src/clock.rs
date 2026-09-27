use jiff::{Timestamp, Zoned, tz::TimeZone};
use serde::{Deserialize, Serialize};
use yokoku_domain::Clock;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ClockSettings {
    /// IANA time zone name, e.g. `Europe/Berlin`; the system zone when unset.
    #[serde(with = "jiff::fmt::serde::tz::optional")]
    pub timezone: Option<TimeZone>,
}

impl ClockSettings {
    pub fn time_zone(&self) -> TimeZone {
        self.timezone.clone().unwrap_or_else(TimeZone::system)
    }
}

#[derive(Debug, Clone)]
pub struct SystemClock {
    time_zone: TimeZone,
}

impl SystemClock {
    pub fn new(time_zone: TimeZone) -> Self {
        Self { time_zone }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Zoned {
        Timestamp::now().to_zoned(self.time_zone.clone())
    }
}
