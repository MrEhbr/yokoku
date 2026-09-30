use std::sync::Mutex;

use jiff::{
    SignedDuration, Zoned,
    civil::{Date, date},
    tz::TimeZone,
};
use yokoku_domain::Clock;

/// The day tests run on.
pub const TODAY: Date = date(2026, 9, 26);

/// A clock standing still until moved with [`TestClock::advance`]; by default noon on [`TODAY`]
/// in UTC.
pub struct TestClock(Mutex<Zoned>);

impl TestClock {
    pub fn at(now: Zoned) -> Self {
        Self(Mutex::new(now))
    }

    pub fn advance(&self, by: SignedDuration) {
        let mut now = self.0.lock().unwrap();
        *now = now.checked_add(by).unwrap();
    }
}

impl Default for TestClock {
    fn default() -> Self {
        Self::at(TODAY.at(12, 0, 0, 0).to_zoned(TimeZone::UTC).unwrap())
    }
}

impl Clock for TestClock {
    fn now(&self) -> Zoned {
        self.0.lock().unwrap().clone()
    }
}
