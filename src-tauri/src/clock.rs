//! Injectable clock so the engine, scheduler and tests agree on "now".
use chrono::{DateTime, Local, NaiveDate, TimeZone};
use parking_lot::Mutex;

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Local>;
    fn now_ts(&self) -> i64 {
        self.now().timestamp()
    }
    fn today(&self) -> String {
        date_key(&self.now())
    }
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Local> {
        Local::now()
    }
}

/// A clock that only moves when told to. Used by tests and simulations.
pub struct FixedClock(Mutex<DateTime<Local>>);
impl FixedClock {
    pub fn at(dt: DateTime<Local>) -> Self {
        Self(Mutex::new(dt))
    }
    pub fn at_ts(ts: i64) -> Self {
        Self::at(Local.timestamp_opt(ts, 0).single().expect("valid ts"))
    }
    pub fn set(&self, dt: DateTime<Local>) {
        *self.0.lock() = dt;
    }
    pub fn advance_secs(&self, secs: i64) {
        let mut g = self.0.lock();
        *g += chrono::Duration::seconds(secs);
    }
}
impl Clock for FixedClock {
    fn now(&self) -> DateTime<Local> {
        *self.0.lock()
    }
}

pub fn date_key(dt: &DateTime<Local>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

pub fn parse_date(key: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(key, "%Y-%m-%d").ok()
}

pub fn days_between(from: &str, to: &str) -> Option<i64> {
    Some((parse_date(to)? - parse_date(from)?).num_days())
}
