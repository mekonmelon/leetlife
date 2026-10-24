//! Background scheduler. Jobs live in a min-heap keyed by `run_at`; the worker
//! sleeps until the earliest job (or until woken by a new push), then hands due
//! jobs to a `JobHandler`. Persistence of jobs is the engine's concern.
use crate::clock::Clock;
use crate::error::Result;
use crate::metrics::{names, Metrics};
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobKind {
    /// 08:00 — build today's plan (idempotent).
    GenerateDailyPlan,
    /// Shortly after the plan: "your character hasn't had breakfast".
    MorningReminder,
    /// Fires at a task's deadline; marks it missed if still pending.
    TaskDeadline { task_id: i64 },
    /// A nudge N minutes before a deadline.
    TaskWarning { task_id: i64 },
    /// 23:55 — overnight decay, streak bookkeeping, tomorrow's jobs.
    EndOfDay,
    /// A spaced-repetition review became due.
    ReviewDue { problem_id: i64 },
    /// Persist metrics summaries.
    FlushMetrics,
    /// Poll the configured activity provider.
    SyncActivity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: i64,
    pub kind: JobKind,
    pub run_at: i64,
}

/// Min-heap of jobs by `run_at`. O(log n) push/pop, O(1) peek; cancelled jobs are
/// tombstoned and skipped lazily when they surface.
#[derive(Default)]
pub struct JobQueue {
    heap: BinaryHeap<Reverse<(i64, i64)>>,
    jobs: HashMap<i64, ScheduledJob>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, job: ScheduledJob) {
        self.heap.push(Reverse((job.run_at, job.id)));
        self.jobs.insert(job.id, job);
    }

    pub fn cancel(&mut self, id: i64) -> bool {
        self.jobs.remove(&id).is_some()
    }

    pub fn cancel_where(&mut self, pred: impl Fn(&JobKind) -> bool) -> usize {
        let before = self.jobs.len();
        self.jobs.retain(|_, j| !pred(&j.kind));
        before - self.jobs.len()
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    pub fn contains_kind(&self, kind: &JobKind) -> bool {
        self.jobs.values().any(|j| &j.kind == kind)
    }

    /// Timestamp of the next live job.
    pub fn next_run_at(&mut self) -> Option<i64> {
        while let Some(Reverse((run_at, id))) = self.heap.peek().copied() {
            if self.jobs.contains_key(&id) {
                return Some(run_at);
            }
            self.heap.pop(); // tombstone
        }
        None
    }

    /// Removes and returns every job with `run_at <= now`, in time order.
    pub fn pop_due(&mut self, now: i64) -> Vec<ScheduledJob> {
        let mut due = Vec::new();
        while let Some(Reverse((run_at, id))) = self.heap.peek().copied() {
            if run_at > now {
                break;
            }
            self.heap.pop();
            if let Some(job) = self.jobs.remove(&id) {
                due.push(job);
            }
        }
        due
    }

    pub fn snapshot(&self) -> Vec<ScheduledJob> {
        let mut v: Vec<ScheduledJob> = self.jobs.values().cloned().collect();
        v.sort_by_key(|j| (j.run_at, j.id));
        v
    }
}

#[async_trait::async_trait]
pub trait JobHandler: Send + Sync + 'static {
    async fn handle_job(&self, job: ScheduledJob) -> Result<()>;
}

pub struct Scheduler {
    queue: Mutex<JobQueue>,
    wake: Notify,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self { queue: Mutex::new(JobQueue::new()), wake: Notify::new() }
    }

    pub fn schedule(&self, job: ScheduledJob) {
        self.queue.lock().push(job);
        self.wake.notify_one();
    }

    pub fn cancel(&self, id: i64) -> bool {
        let r = self.queue.lock().cancel(id);
        self.wake.notify_one();
        r
    }

    pub fn cancel_where(&self, pred: impl Fn(&JobKind) -> bool) -> usize {
        let n = self.queue.lock().cancel_where(pred);
        self.wake.notify_one();
        n
    }

    pub fn contains_kind(&self, kind: &JobKind) -> bool {
        self.queue.lock().contains_kind(kind)
    }

    /// Wakes the worker so it re-evaluates what is due. The worker also wakes
    /// on every schedule/cancel; this is for callers that moved the clock.
    pub fn wake(&self) {
        self.wake.notify_one();
    }

    pub fn pending(&self) -> Vec<ScheduledJob> {
        self.queue.lock().snapshot()
    }

    pub fn len(&self) -> usize {
        self.queue.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }

    /// Drains and runs whatever is due right now. Used by the worker and by tests.
    pub async fn run_due<H: JobHandler>(&self, handler: &H, now: i64, metrics: &Metrics) -> usize {
        let due = self.queue.lock().pop_due(now);
        let n = due.len();
        for job in due {
            let _t = metrics.timer(names::SCHEDULER_JOB);
            let kind = job.kind.clone();
            if let Err(e) = handler.handle_job(job).await {
                tracing::error!(?kind, error = %e, "scheduled job failed");
            }
        }
        n
    }

    pub async fn run<H: JobHandler>(self: Arc<Self>, handler: Arc<H>, clock: Arc<dyn Clock>, metrics: Arc<Metrics>, cancel: CancellationToken) {
        loop {
            let now = clock.now_ts();
            self.run_due(handler.as_ref(), now, &metrics).await;

            let wait = match self.queue.lock().next_run_at() {
                Some(next) => Duration::from_secs((next - clock.now_ts()).clamp(0, 3600) as u64),
                None => Duration::from_secs(3600),
            };
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                _ = self.wake.notified() => {}
                _ = tokio::time::sleep(wait) => {}
            }
        }
        tracing::info!("scheduler stopped");
    }
}

/// Unix timestamp of `date` at `hour:minute` local time.
pub fn at_local(date: NaiveDate, hour: u32, minute: u32) -> i64 {
    let naive = date.and_hms_opt(hour, minute, 0).expect("valid time");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .unwrap_or_else(|| Local.from_utc_datetime(&naive))
        .timestamp()
}

/// Next occurrence of `hour:minute` strictly after `now`.
pub fn next_daily(now: DateTime<Local>, hour: u32, minute: u32) -> i64 {
    let today = now.date_naive();
    let ts = at_local(today, hour, minute);
    if ts > now.timestamp() {
        ts
    } else {
        at_local(today.succ_opt().unwrap_or(today), hour, minute)
    }
}

pub fn weekday_name(date: NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn job(id: i64, run_at: i64) -> ScheduledJob {
        ScheduledJob { id, kind: JobKind::FlushMetrics, run_at }
    }

    #[test]
    fn heap_pops_in_time_order_and_skips_cancelled() {
        let mut q = JobQueue::new();
        q.push(job(1, 30));
        q.push(job(2, 10));
        q.push(job(3, 20));
        q.push(job(4, 40));
        assert!(q.cancel(3));
        assert_eq!(q.next_run_at(), Some(10));
        let due: Vec<i64> = q.pop_due(30).into_iter().map(|j| j.id).collect();
        assert_eq!(due, vec![2, 1]);
        assert_eq!(q.next_run_at(), Some(40));
        assert_eq!(q.len(), 1);
    }

    struct Counter(AtomicUsize);
    #[async_trait::async_trait]
    impl JobHandler for Counter {
        async fn handle_job(&self, _job: ScheduledJob) -> Result<()> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn run_due_only_runs_jobs_at_or_before_now() {
        let s = Scheduler::new();
        s.schedule(job(1, 100));
        s.schedule(job(2, 200));
        let h = Counter(AtomicUsize::new(0));
        let m = Metrics::new();
        assert_eq!(s.run_due(&h, 150, &m).await, 1);
        assert_eq!(h.0.load(Ordering::SeqCst), 1);
        assert_eq!(s.len(), 1);
        assert_eq!(m.get(names::SCHEDULER_JOB).unwrap().count, 1);
    }

    #[test]
    fn next_daily_rolls_to_tomorrow_after_the_time() {
        let now = Local.with_ymd_and_hms(2026, 9, 18, 9, 0, 0).unwrap();
        let eight = next_daily(now, 8, 0);
        let ten = next_daily(now, 10, 0);
        assert!(ten > now.timestamp() && ten - now.timestamp() == 3600);
        assert!(eight > now.timestamp() && eight - now.timestamp() == 23 * 3600);
    }
}
