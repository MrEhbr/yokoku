use jiff::{Timestamp, Zoned, tz::TimeZone};
use yokoku_domain::Clock;

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
