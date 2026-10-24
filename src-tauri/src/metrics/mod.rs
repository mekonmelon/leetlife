//! Latency instrumentation. Each named series keeps a bounded reservoir of the
//! most recent samples plus running count/sum/max, so percentiles are cheap and
//! memory is fixed regardless of uptime.
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

pub mod names {
    pub const IPC_SNAPSHOT: &str = "ipc.get_snapshot";
    pub const IPC_LOG_ATTEMPT: &str = "ipc.log_attempt";
    pub const IPC_PURCHASE: &str = "ipc.purchase_item";
    pub const IPC_STATS: &str = "ipc.get_stats";
    pub const UI_OPEN: &str = "ui.open";
    pub const UI_RENDER: &str = "ui.render";
    pub const DB_ATTEMPT_TX: &str = "db.attempt_tx";
    pub const DB_SNAPSHOT_LOAD: &str = "db.snapshot_load";
    pub const EVENT_DISPATCH: &str = "event.dispatch";
    pub const RECOMMEND: &str = "recommend.generate";
    pub const SCHEDULER_JOB: &str = "scheduler.job";
    pub const NOTIFY: &str = "notify.send";
    pub const CACHE_REFRESH: &str = "cache.refresh";
}

const RESERVOIR: usize = 2048;

#[derive(Default)]
struct Series {
    count: u64,
    sum_us: f64,
    max_us: f64,
    samples: VecDeque<f64>,
}

impl Series {
    fn record(&mut self, us: f64) {
        self.count += 1;
        self.sum_us += us;
        if us > self.max_us {
            self.max_us = us;
        }
        if self.samples.len() == RESERVOIR {
            self.samples.pop_front();
        }
        self.samples.push_back(us);
    }

    fn summary(&self, name: &str) -> MetricSummary {
        let mut sorted: Vec<f64> = self.samples.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pct = |p: f64| -> f64 {
            if sorted.is_empty() {
                return 0.0;
            }
            let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
            sorted[idx.min(sorted.len() - 1)]
        };
        MetricSummary {
            name: name.to_string(),
            count: self.count,
            mean_us: if self.count == 0 { 0.0 } else { self.sum_us / self.count as f64 },
            p50_us: pct(0.50),
            p95_us: pct(0.95),
            p99_us: pct(0.99),
            max_us: self.max_us,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricSummary {
    pub name: String,
    pub count: u64,
    pub mean_us: f64,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
}

#[derive(Default)]
pub struct Metrics {
    series: Mutex<HashMap<String, Series>>,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, name: &str, d: Duration) {
        self.record_us(name, d.as_secs_f64() * 1e6);
    }

    pub fn record_us(&self, name: &str, us: f64) {
        let mut g = self.series.lock();
        g.entry(name.to_string()).or_default().record(us);
    }

    /// Times a synchronous closure.
    pub fn time<T>(&self, name: &str, f: impl FnOnce() -> T) -> T {
        let t0 = Instant::now();
        let out = f();
        self.record(name, t0.elapsed());
        out
    }

    /// RAII timer: records on drop, so early returns and `?` are covered.
    pub fn timer(&self, name: &'static str) -> Timer<'_> {
        Timer { metrics: self, name, start: Instant::now() }
    }

    pub fn summary(&self) -> Vec<MetricSummary> {
        let g = self.series.lock();
        let mut out: Vec<MetricSummary> = g.iter().map(|(k, v)| v.summary(k)).collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    pub fn get(&self, name: &str) -> Option<MetricSummary> {
        self.series.lock().get(name).map(|s| s.summary(name))
    }
}

pub struct Timer<'a> {
    metrics: &'a Metrics,
    name: &'static str,
    start: Instant,
}

impl Drop for Timer<'_> {
    fn drop(&mut self) {
        self.metrics.record(self.name, self.start.elapsed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_are_ordered_and_bounded() {
        let m = Metrics::new();
        for i in 1..=1000 {
            m.record_us("x", i as f64);
        }
        let s = m.get("x").unwrap();
        assert_eq!(s.count, 1000);
        assert!((s.mean_us - 500.5).abs() < 1e-6);
        assert!(s.p50_us <= s.p95_us && s.p95_us <= s.p99_us && s.p99_us <= s.max_us);
        assert_eq!(s.max_us, 1000.0);
        assert!((s.p95_us - 950.0).abs() <= 1.0);
    }

    #[test]
    fn reservoir_is_bounded_but_count_is_not() {
        let m = Metrics::new();
        for _ in 0..(RESERVOIR * 3) {
            m.record_us("y", 1.0);
        }
        let s = m.get("y").unwrap();
        assert_eq!(s.count as usize, RESERVOIR * 3);
        assert_eq!(m.series.lock()["y"].samples.len(), RESERVOIR);
    }

    #[test]
    fn timer_records_on_drop() {
        let m = Metrics::new();
        {
            let _t = m.timer("z");
        }
        assert_eq!(m.get("z").unwrap().count, 1);
    }
}
