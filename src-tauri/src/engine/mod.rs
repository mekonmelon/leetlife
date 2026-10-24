//! The engine: the only thing that mutates state. Every user action is one
//! SQLite transaction that (1) applies the game rules, (2) writes the resulting
//! events to the `events` outbox, and (3) only after commit publishes them on
//! the bus, updates the scheduler and refreshes the in-memory snapshot.
pub mod plan;

use crate::activity::{ActivityProvider, ExternalAttempt, LeetCodeProvider};
use crate::cache::{Snapshot, StateCache};
use crate::clock::{date_key, days_between, parse_date, Clock};
use crate::error::{Error, Result};
use crate::events::{AppEvent, EventBus, EventDispatcher};
use crate::game::{self, AvatarState, Reward};
use crate::metrics::{names, MetricSummary, Metrics};
use crate::models::*;
use crate::notifications::{run_notification_worker, Notifier};
use crate::recommendation::{self, spaced, Filter, Recommendation, TopicScore};
use crate::scheduler::{at_local, next_daily, weekday_name, JobHandler, JobKind, ScheduledJob, Scheduler};
use crate::storage::repo::{self, MetricHistoryRow, NewAttempt, NewProblem, NewTask};
use crate::storage::{seed, Db};
use chrono::{Local, NaiveDate, TimeZone, Timelike};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use sqlx::SqliteConnection;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub const SETTING_LEETCODE_USERNAME: &str = "leetcode_username";
pub const SETTING_LAST_DAY_ENDED: &str = "last_day_ended";
pub const SETTING_INGEST_PORT: &str = "ingest_port";
pub const DEFAULT_INGEST_PORT: u16 = 47_831;

const METRICS_FLUSH_SECS: i64 = 600;
const SYNC_SECS: i64 = 900;
const WARNING_LEAD_SECS: i64 = 30 * 60;
const MAX_CATCH_UP_DAYS: i64 = 30;

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub bus_capacity: usize,
    pub recommendation_limit: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self { bus_capacity: 256, recommendation_limit: 3 }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LogAttemptInput {
    pub problem_id: Option<i64>,
    pub slug: Option<String>,
    /// Used only when the slug is unknown and we need to create a custom problem.
    pub title: Option<String>,
    pub difficulty: Option<Difficulty>,
    pub topic: Option<String>,
    pub result: AttemptResult,
    pub duration_ms: Option<i64>,
    /// Unix seconds; defaults to now. Attempts dated before today never touch today's tasks.
    pub at: Option<i64>,
    pub source: Option<String>,
    pub external_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AttemptOutcome {
    pub attempt_id: i64,
    pub problem: ProblemMeta,
    pub reward: Reward,
    pub completed_tasks: Vec<DailyTask>,
    pub level_up: Option<i64>,
    pub streak: i64,
    pub is_review: bool,
    pub next_review_at: i64,
    pub achievements: Vec<String>,
    pub snapshot: Arc<Snapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncReport {
    pub provider: String,
    pub fetched: usize,
    pub imported: usize,
    pub error: Option<String>,
}

/// Side effects a transaction wants performed after it commits.
#[derive(Default)]
struct Effects {
    events: Vec<AppEvent>,
    jobs: Vec<ScheduledJob>,
    cancelled_jobs: Vec<i64>,
}

impl Effects {
    async fn event(&mut self, c: &mut SqliteConnection, ev: AppEvent, at: i64) -> Result<()> {
        repo::insert_event(c, ev.kind(), &serde_json::to_string(&ev)?, at).await?;
        self.events.push(ev);
        Ok(())
    }
    async fn schedule(&mut self, c: &mut SqliteConnection, kind: JobKind, run_at: i64) -> Result<()> {
        self.jobs.push(repo::insert_job(c, &kind, run_at).await?);
        Ok(())
    }
}

pub struct Engine {
    db: Db,
    clock: Arc<dyn Clock>,
    config: EngineConfig,
    bus: EventBus,
    dispatcher: Mutex<Option<EventDispatcher>>,
    scheduler: Arc<Scheduler>,
    cache: StateCache,
    metrics: Arc<Metrics>,
    /// SQLite has one writer; serialising writers here turns "database is
    /// locked" retries into plain queueing.
    write_lock: tokio::sync::Mutex<()>,
    last_reward: Mutex<Option<RewardSummary>>,
    provider: RwLock<Option<Arc<dyn ActivityProvider>>>,
}

impl Engine {
    pub async fn open(path: &Path, clock: Arc<dyn Clock>, config: EngineConfig) -> Result<Arc<Engine>> {
        Ok(Self::with_db(Db::open(path).await?, clock, config))
    }

    pub async fn in_memory(clock: Arc<dyn Clock>) -> Result<Arc<Engine>> {
        Ok(Self::with_db(Db::in_memory().await?, clock, EngineConfig::default()))
    }

    /// Reopens an already-open database with a fresh engine (a simulated restart).
    pub fn from_db(db: Db, clock: Arc<dyn Clock>, config: EngineConfig) -> Arc<Engine> {
        Self::with_db(db, clock, config)
    }

    fn with_db(db: Db, clock: Arc<dyn Clock>, config: EngineConfig) -> Arc<Engine> {
        let (bus, dispatcher) = EventBus::new(config.bus_capacity);
        Arc::new(Engine {
            db,
            clock,
            config,
            bus,
            dispatcher: Mutex::new(Some(dispatcher)),
            scheduler: Arc::new(Scheduler::new()),
            cache: StateCache::new(),
            metrics: Arc::new(Metrics::new()),
            write_lock: tokio::sync::Mutex::new(()),
            last_reward: Mutex::new(None),
            provider: RwLock::new(None),
        })
    }

    pub fn clock(&self) -> &dyn Clock {
        self.clock.as_ref()
    }
    pub fn metrics(&self) -> &Arc<Metrics> {
        &self.metrics
    }
    pub fn bus(&self) -> &EventBus {
        &self.bus
    }
    pub fn scheduler(&self) -> &Arc<Scheduler> {
        &self.scheduler
    }
    pub fn db(&self) -> &Db {
        &self.db
    }
    pub fn set_provider(&self, p: Option<Arc<dyn ActivityProvider>>) {
        *self.provider.write() = p;
    }

    // ------------------------------------------------------------ lifecycle

    /// Migrate, seed, catch up on days the app was closed, build today's plan,
    /// restore persisted jobs and warm the cache. Idempotent.
    pub async fn bootstrap(&self) -> Result<Arc<Snapshot>> {
        self.db.migrate().await?;
        let today = self.clock.today();
        let now = self.clock.now_ts();
        {
            let _w = self.write_lock.lock().await;
            let mut tx = self.db.begin().await?;
            seed::seed_all(&mut tx).await?;
            if repo::load_avatar(&mut tx).await?.is_none() {
                repo::save_avatar(&mut tx, &AvatarState::fresh(&today, now)).await?;
            }
            if repo::get_setting(&mut tx, SETTING_LAST_DAY_ENDED).await?.is_none() {
                let yesterday = parse_date(&today).and_then(|d| d.pred_opt()).map(|d| d.to_string()).unwrap_or_else(|| today.clone());
                repo::set_setting(&mut tx, SETTING_LAST_DAY_ENDED, &yesterday).await?;
            }
            tx.commit().await?;
        }
        self.catch_up_days().await?;
        self.generate_daily_plan(parse_date(&today).expect("today parses")).await?;
        self.restore_jobs().await?;
        self.configure_provider_from_settings().await?;
        self.refresh_snapshot().await
    }

    /// Runs `end_of_day` for every day between the last processed one and yesterday.
    async fn catch_up_days(&self) -> Result<()> {
        let today = parse_date(&self.clock.today()).expect("today parses");
        let last = {
            let mut c = self.db.conn().await?;
            repo::get_setting(&mut c, SETTING_LAST_DAY_ENDED).await?.and_then(|s| parse_date(&s))
        };
        let Some(mut d) = last.and_then(|d| d.succ_opt()) else { return Ok(()) };
        // Bound work after a long absence to the most recent month, rather
        // than taking multiple launches to ever catch up to the present.
        let earliest = today - chrono::Duration::days(MAX_CATCH_UP_DAYS);
        d = d.max(earliest);
        let mut n = 0;
        while d < today && n < MAX_CATCH_UP_DAYS {
            self.end_of_day(d).await?;
            d = d.succ_opt().expect("date");
            n += 1;
        }
        Ok(())
    }

    async fn restore_jobs(&self) -> Result<()> {
        let _w = self.write_lock.lock().await;
        let mut tx = self.db.begin().await?;
        let jobs = repo::pending_jobs(&mut tx).await?;
        let now = self.clock.now();
        let recurring = [
            (JobKind::GenerateDailyPlan, next_daily(now, 0, 0)),
            (JobKind::EndOfDay, next_daily(now, 0, 0)),
            (JobKind::FlushMetrics, now.timestamp() + METRICS_FLUSH_SECS),
            (JobKind::SyncActivity, now.timestamp() + SYNC_SECS),
        ];
        let mut new_jobs = Vec::new();
        for (kind, run_at) in recurring {
            if !jobs.iter().any(|j| j.kind == kind) {
                new_jobs.push(repo::insert_job(&mut tx, &kind, run_at).await?);
            }
        }
        tx.commit().await?;
        for j in jobs.into_iter().chain(new_jobs) {
            self.scheduler.schedule(j);
        }
        Ok(())
    }

    pub async fn configure_provider_from_settings(&self) -> Result<()> {
        let mut c = self.db.conn().await?;
        let username = repo::get_setting(&mut c, SETTING_LEETCODE_USERNAME).await?.filter(|s| !s.trim().is_empty());
        self.set_provider(username.map(|u| Arc::new(LeetCodeProvider::new(u.trim())) as Arc<dyn ActivityProvider>));
        Ok(())
    }

    /// Starts the async workers: event dispatcher, scheduler, notifier.
    /// Returns handles so the app can await them on shutdown.
    pub fn spawn_workers(self: &Arc<Self>, notifier: Arc<dyn Notifier>, cancel: CancellationToken) -> Vec<JoinHandle<()>> {
        let mut handles = Vec::new();
        let dispatcher = self.dispatcher.lock().take().expect("workers started twice");
        let notif_rx = self.bus.subscribe();
        handles.push(tokio::spawn(run_notification_worker(notif_rx, notifier, self.metrics.clone(), cancel.clone())));
        handles.push(tokio::spawn(dispatcher.run(self.metrics.clone(), cancel.clone())));
        handles.push(tokio::spawn(self.scheduler.clone().run(self.clone(), self.clock.clone(), self.metrics.clone(), cancel)));
        handles
    }

    /// Persist the metrics reservoir; called on the flush job and at shutdown.
    pub async fn flush_metrics(&self) -> Result<()> {
        let summary = self.metrics.summary();
        if summary.is_empty() {
            return Ok(());
        }
        let now = self.clock.now_ts();
        let _w = self.write_lock.lock().await;
        let mut tx = self.db.begin().await?;
        repo::insert_metrics(&mut tx, &summary, now).await?;
        repo::prune_finished_jobs(&mut tx, now - 7 * recommendation::DAY).await?;
        tx.commit().await?;
        Ok(())
    }

    // ------------------------------------------------------------ snapshot / reads

    /// Cached view for the menu-bar window: O(1) and never touches SQLite.
    pub async fn snapshot(&self) -> Result<Arc<Snapshot>> {
        match self.cache.get() {
            Some(s) => Ok(s),
            None => self.refresh_snapshot().await,
        }
    }

    pub async fn refresh_snapshot(&self) -> Result<Arc<Snapshot>> {
        let _t = self.metrics.timer(names::CACHE_REFRESH);
        let mut c = self.db.conn().await?;
        let snap = self.build_snapshot(&mut c).await?;
        Ok(self.cache.replace(snap))
    }

    async fn build_snapshot(&self, c: &mut SqliteConnection) -> Result<Snapshot> {
        let _t = self.metrics.timer(names::DB_SNAPSHOT_LOAD);
        let now = self.clock.now();
        let now_ts = now.timestamp();
        let today = date_key(&now);
        let avatar = repo::load_avatar(c).await?.unwrap_or_else(|| AvatarState::fresh(&today, now_ts));
        let tasks = repo::tasks_for_date(c, &today).await?;
        let inventory = repo::inventory(c).await?;
        let topics = repo::all_topics(c).await?;
        let recommendations = self.recommend_with(c, &Filter::default(), self.config.recommendation_limit).await?;
        let room_level = repo::room_level(c).await?;
        let due_reviews = repo::count_due_reviews(c, now_ts).await?;
        let (xp_into_level, xp_for_next_level) = avatar.xp_into_level();
        let mood = game::mood_line(&avatar, &tasks, now.hour());
        let home = repo::get_setting(c, crate::home::SETTING_HOME_LAYOUT).await?
            .map(|value| serde_json::from_str(&value)).transpose()?.unwrap_or_default();
        Ok(Snapshot {
            version: 0,
            generated_at: now_ts,
            weekday: weekday_name(now.date_naive()).to_string(),
            day_number: avatar.day_number(&today),
            date: today,
            avatar,
            mood,
            xp_into_level,
            xp_for_next_level,
            tasks,
            inventory,
            topics,
            recommendations,
            room_level,
            due_reviews,
            last_reward: self.last_reward.lock().clone(),
            home,
        })
    }

    async fn recommend_with(&self, c: &mut SqliteConnection, filter: &Filter, limit: usize) -> Result<Vec<Recommendation>> {
        let now = self.clock.now_ts();
        let problems = repo::all_problems(c).await?;
        let topics = repo::all_topics(c).await?;
        let reviews = repo::all_reviews(c).await?;
        let recent = repo::recent_attempt_counts(c, now - 7 * recommendation::DAY).await?;
        Ok(self.metrics.time(names::RECOMMEND, || recommendation::recommend(&problems, &topics, &reviews, &recent, now, filter, limit)))
    }

    pub async fn recommendations(&self, filter: Filter, limit: usize) -> Result<Vec<Recommendation>> {
        let mut c = self.db.conn().await?;
        self.recommend_with(&mut c, &filter, limit).await
    }

    pub async fn search_problems(&self, query: &str, limit: i64) -> Result<Vec<ProblemMeta>> {
        let mut c = self.db.conn().await?;
        repo::search_problems(&mut c, query, limit).await
    }

    pub async fn topics(&self) -> Result<Vec<String>> {
        let mut c = self.db.conn().await?;
        repo::catalogue_topics(&mut c).await
    }

    pub async fn shop(&self) -> Result<Vec<ShopItem>> {
        let mut c = self.db.conn().await?;
        let avatar = repo::load_avatar(&mut c).await?.unwrap_or_else(|| AvatarState::fresh(&self.clock.today(), self.clock.now_ts()));
        let owned: HashSet<i64> = repo::inventory(&mut c).await?.into_iter().map(|o| o.item.id).collect();
        Ok(repo::all_items(&mut c)
            .await?
            .into_iter()
            .map(|item| ShopItem {
                owned: owned.contains(&item.id),
                affordable: avatar.coins >= item.cost,
                unlocked: avatar.level >= item.min_level,
                item,
            })
            .collect())
    }

    pub async fn stats(&self) -> Result<Stats> {
        let _t = self.metrics.timer(names::IPC_STATS);
        let mut c = self.db.conn().await?;
        let now = self.clock.now();
        let today = now.date_naive();
        let from_date = today - chrono::Duration::days(13);
        let from_ts = at_local(from_date, 0, 0);
        let to_ts = at_local(today.succ_opt().unwrap_or(today), 0, 0);
        let mut buckets: Vec<DayCount> = (0..14)
            .map(|i| DayCount { date: (from_date + chrono::Duration::days(i)).to_string(), solved: 0, failed: 0 })
            .collect();
        for (ts, result) in repo::attempts_between(&mut c, from_ts, to_ts).await? {
            let d = Local.timestamp_opt(ts, 0).single().map(|dt| dt.date_naive()).unwrap_or(today);
            let idx = (d - from_date).num_days();
            if let Some(b) = usize::try_from(idx).ok().and_then(|i| buckets.get_mut(i)) {
                if result == "solved" {
                    b.solved += 1
                } else {
                    b.failed += 1
                }
            }
        }
        Ok(Stats {
            total_solved: repo::count_distinct_solved(&mut c).await?,
            total_attempts: repo::count_attempts(&mut c).await?,
            solved_by_difficulty: repo::solved_by_difficulty(&mut c).await?,
            last_14_days: buckets,
            topics: repo::all_topics(&mut c).await?,
            due_reviews: repo::due_reviews(&mut c, now.timestamp(), 10).await?,
            recent_events: repo::recent_events(&mut c, 25).await?,
            achievements: repo::all_achievements(&mut c).await?,
        })
    }

    pub fn metrics_summary(&self) -> Vec<MetricSummary> {
        self.metrics.summary()
    }

    pub async fn metric_history(&self, name: &str, limit: i64) -> Result<Vec<MetricHistoryRow>> {
        let mut c = self.db.conn().await?;
        repo::metric_history(&mut c, name, limit).await
    }

    pub async fn settings(&self) -> Result<HashMap<String, String>> {
        let mut c = self.db.conn().await?;
        repo::all_settings(&mut c).await
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        {
            let _w = self.write_lock.lock().await;
            let mut c = self.db.conn().await?;
            repo::set_setting(&mut c, key, value).await?;
        }
        if key == SETTING_LEETCODE_USERNAME {
            self.configure_provider_from_settings().await?;
        }
        Ok(())
    }

    pub async fn ingest_port(&self) -> u16 {
        let mut c = match self.db.conn().await {
            Ok(c) => c,
            Err(_) => return DEFAULT_INGEST_PORT,
        };
        repo::get_setting(&mut c, SETTING_INGEST_PORT).await.ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_INGEST_PORT)
    }

    pub async fn recent_attempts(&self, limit: i64) -> Result<Vec<AttemptRow>> {
        let mut c = self.db.conn().await?;
        repo::recent_attempts(&mut c, limit).await
    }

    // ------------------------------------------------------------ actions

    /// The main reducer: one attempt, one transaction.
    pub async fn log_attempt(&self, input: LogAttemptInput) -> Result<AttemptOutcome> {
        let _t = self.metrics.timer(names::IPC_LOG_ATTEMPT);
        // The Mac may have slept across midnight before the scheduler woke.
        if self.cache.get().map_or(true, |s| s.date != self.clock.today()) {
            self.catch_up_days().await?;
            self.generate_daily_plan(self.clock.now().date_naive()).await?;
        }
        let _w = self.write_lock.lock().await;
        let mut tx = self.db.begin().await?;
        let (applied, effects) = {
            let _tx_timer = self.metrics.timer(names::DB_ATTEMPT_TX);
            let r = self.apply_attempt(&mut tx, &input).await?;
            tx.commit().await?;
            r
        };
        *self.last_reward.lock() = Some(RewardSummary {
            headline: applied.headline.clone(),
            xp: applied.reward.xp,
            coins: applied.reward.coins,
            at: self.clock.now_ts(),
        });
        let snapshot = self.after_commit(effects).await?;
        Ok(AttemptOutcome {
            attempt_id: applied.attempt_id,
            problem: applied.problem,
            reward: applied.reward,
            completed_tasks: applied.completed_tasks,
            level_up: applied.level_up,
            streak: applied.streak,
            is_review: applied.is_review,
            next_review_at: applied.next_review_at,
            achievements: applied.achievements,
            snapshot,
        })
    }

    async fn apply_attempt(&self, c: &mut SqliteConnection, input: &LogAttemptInput) -> Result<(Applied, Effects)> {
        let now = self.clock.now_ts();
        let today = self.clock.today();
        let at = input.at.unwrap_or(now).min(now);
        let at_date = Local.timestamp_opt(at, 0).single().map(|d| date_key(&d)).unwrap_or_else(|| today.clone());
        let mut fx = Effects::default();

        let problem = self.resolve_problem(c, input).await?;
        let difficulty = problem.difficulty();
        let topic = problem.topic.clone();
        let source = input.source.as_deref().unwrap_or("manual");

        let attempt_id = repo::insert_attempt(
            c,
            &NewAttempt {
                problem_id: problem.id,
                difficulty,
                result: input.result,
                duration_ms: input.duration_ms,
                attempted_at: at,
                topic: &topic,
                source,
                external_id: input.external_id.as_deref(),
            },
        )
        .await?;

        let mut ts = repo::topic_score(c, &topic).await?.unwrap_or_else(|| TopicScore::new(&topic));
        recommendation::update_confidence(&mut ts, input.result, difficulty, at);
        repo::upsert_topic(c, &ts).await?;

        let prev = repo::review_state(c, problem.id).await?;
        let is_review = prev.is_some();
        let solved = input.result == AttemptResult::Solved;
        let next = spaced::next_review(prev.as_ref(), problem.id, solved, at);
        repo::upsert_review(c, &next).await?;

        let mut reward = game::reward_for_attempt(difficulty, input.result, is_review);
        let mut avatar = repo::load_avatar(c).await?.unwrap_or_else(|| AvatarState::fresh(&today, now));

        let base = if solved {
            AppEvent::ProblemSolved { attempt_id, problem_id: problem.id, slug: problem.slug.clone(), title: problem.title.clone(), difficulty, topic: topic.clone(), is_review }
        } else {
            AppEvent::ProblemFailed { attempt_id, problem_id: problem.id, slug: problem.slug.clone(), title: problem.title.clone(), difficulty, topic: topic.clone() }
        };
        fx.event(c, base, now).await?;
        if is_review {
            fx.event(c, AppEvent::ReviewCompleted { problem_id: problem.id, success: solved, next_due: next.due_at }, now).await?;
        }

        // One attempt advances at most one slot, the earliest pending one it
        // satisfies. Slots are ordered by deadline, so your first solve of the
        // day feeds breakfast rather than silently clearing the evening task.
        let mut completed = Vec::new();
        let mut fed = false;
        if at_date == today {
            let matched = repo::tasks_for_date(c, &today)
                .await?
                .into_iter()
                .find(|t| game::attempt_matches(t, problem.id, difficulty, &topic, input.result));
            if let Some(mut task) = matched {
                task.progress += 1;
                if task.progress >= task.target {
                    task.status = TaskStatus::Done;
                    task.completed_at = Some(now);
                    let meters = game::slot_reward(task.slot);
                    reward.add(&Reward { xp: task.reward_xp, coins: task.reward_coins, hunger: meters.hunger, energy: meters.energy, happiness: meters.happiness });
                    fx.cancelled_jobs.extend(repo::cancel_jobs_for_task(c, task.id).await?);
                    fx.event(c, AppEvent::DailyGoalCompleted { task_id: task.id, slot: task.slot, title: task.title.clone() }, now).await?;
                    fed |= task.slot == Slot::Breakfast;
                    completed.push(task.clone());
                }
                repo::update_task_progress(c, &task).await?;
            }
        }

        let level_up = avatar.apply(&reward, now);
        fx.event(c, AppEvent::RewardEarned { xp: reward.xp, coins: reward.coins, source: source.to_string() }, now).await?;
        if fed {
            fx.event(c, AppEvent::AvatarFed { hunger: avatar.hunger }, now).await?;
        }
        if solved && at_date == today {
            if let Some(streak) = avatar.touch_streak(&today) {
                fx.event(c, AppEvent::StreakUpdated { streak, best: avatar.best_streak }, now).await?;
            }
        }
        if let Some(level) = level_up {
            fx.event(c, AppEvent::LevelUp { level }, now).await?;
        }
        repo::save_avatar(c, &avatar).await?;

        let achievements = self.check_achievements(c, &avatar, &mut fx, now).await?;

        let headline = match (solved, is_review, completed.first()) {
            (true, _, Some(t)) => format!("{} done: {}", t.slot.label(), t.title),
            (true, true, None) => format!("Review passed: {}", problem.title),
            (true, false, None) => format!("Solved {}", problem.title),
            (false, _, _) => format!("Attempted {}", problem.title),
        };

        Ok((
            Applied {
                attempt_id,
                problem,
                reward,
                completed_tasks: completed,
                level_up,
                streak: avatar.streak,
                is_review,
                next_review_at: next.due_at,
                achievements,
                headline,
            },
            fx,
        ))
    }

    async fn resolve_problem(&self, c: &mut SqliteConnection, input: &LogAttemptInput) -> Result<ProblemMeta> {
        if let Some(id) = input.problem_id {
            return repo::problem_by_id(c, id).await?.ok_or_else(|| Error::Invalid(format!("unknown problem id {id}")));
        }
        let slug = input.slug.as_deref().map(str::trim).filter(|s| !s.is_empty()).ok_or_else(|| Error::Invalid("problem_id or slug is required".into()))?;
        if let Some(p) = repo::problem_by_slug(c, slug).await? {
            return Ok(p);
        }
        let difficulty = input.difficulty.ok_or_else(|| Error::Invalid(format!("'{slug}' is not in the catalogue; difficulty is required to add it")))?;
        let title = input.title.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or_else(|| titleize(slug));
        let topic = input.topic.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or("General");
        let id = repo::upsert_problem(c, &NewProblem { slug, title: &title, difficulty, topic, importance: 0.5, leetcode_id: None }).await?;
        repo::problem_by_id(c, id).await?.ok_or_else(|| Error::Invalid("failed to create problem".into()))
    }

    async fn check_achievements(&self, c: &mut SqliteConnection, avatar: &AvatarState, fx: &mut Effects, now: i64) -> Result<Vec<String>> {
        let solved = repo::count_solved_where(c, None).await?;
        let hards = repo::count_solved_where(c, Some(Difficulty::Hard)).await?;
        let perfect = repo::perfect_days(c).await?;
        let owned = repo::inventory(c).await?.len() as i64;
        let candidates: [(&str, &str, bool); 11] = [
            ("first_solve", "First blood", solved >= 1),
            ("solved_10", "Ten down", solved >= 10),
            ("solved_50", "Fifty problems", solved >= 50),
            ("solved_100", "Century", solved >= 100),
            ("first_hard", "Hard mode", hards >= 1),
            ("streak_7", "One week streak", avatar.streak >= 7),
            ("streak_30", "One month streak", avatar.streak >= 30),
            ("level_5", "Level 5", avatar.level >= 5),
            ("level_10", "Level 10", avatar.level >= 10),
            ("perfect_day", "Perfect day", perfect >= 1),
            ("decorator", "Interior decorator", owned >= 5),
        ];
        let mut unlocked = Vec::new();
        for (key, title, ok) in candidates {
            if ok && !repo::has_achievement(c, key).await? {
                repo::insert_achievement(c, key, title, now).await?;
                fx.event(c, AppEvent::AchievementUnlocked { key: key.into(), title: title.into() }, now).await?;
                unlocked.push(title.to_string());
            }
        }
        Ok(unlocked)
    }

    pub async fn purchase_item(&self, item_id: i64) -> Result<Arc<Snapshot>> {
        let _t = self.metrics.timer(names::IPC_PURCHASE);
        let _w = self.write_lock.lock().await;
        let now = self.clock.now_ts();
        let today = self.clock.today();
        let mut tx = self.db.begin().await?;
        let mut fx = Effects::default();
        let item = repo::item(&mut tx, item_id).await?.ok_or(Error::UnknownItem(item_id))?;
        if repo::owns_item(&mut tx, item_id).await? {
            return Err(Error::AlreadyOwned);
        }
        let mut avatar = repo::load_avatar(&mut tx).await?.unwrap_or_else(|| AvatarState::fresh(&today, now));
        if avatar.level < item.min_level {
            return Err(Error::LevelRequired(item.min_level));
        }
        if avatar.coins < item.cost {
            return Err(Error::InsufficientCoins { need: item.cost, have: avatar.coins });
        }
        avatar.apply(&Reward { coins: -item.cost, happiness: 5, ..Default::default() }, now);
        repo::save_avatar(&mut tx, &avatar).await?;
        repo::insert_inventory(&mut tx, item_id, now).await?;
        if let Some(level) = room_level_for(&item.asset_path) {
            let current = repo::room_level(&mut tx).await?;
            repo::set_room_level(&mut tx, current.max(level)).await?;
        }
        fx.event(&mut tx, AppEvent::ItemPurchased { item_id, name: item.name.clone(), cost: item.cost }, now).await?;
        self.check_achievements(&mut tx, &avatar, &mut fx, now).await?;
        tx.commit().await?;
        *self.last_reward.lock() = Some(RewardSummary { headline: format!("Bought {}", item.name), xp: 0, coins: -item.cost, at: now });
        self.after_commit(fx).await
    }

    pub async fn set_equipped(&self, item_id: i64, equipped: bool) -> Result<Arc<Snapshot>> {
        let _w = self.write_lock.lock().await;
        {
            let mut c = self.db.conn().await?;
            if !repo::set_equipped(&mut c, item_id, equipped).await? {
                return Err(Error::UnknownItem(item_id));
            }
        }
        self.refresh_snapshot().await
    }

    /// Decorating is an owned-item operation, committed with its equip state.
    pub async fn place_item(&self, item_id: i64, placement: crate::home::Placement) -> Result<Arc<Snapshot>> {
        placement.validate()?;
        let _w = self.write_lock.lock().await;
        let mut tx = self.db.begin().await?;
        if !repo::owns_item(&mut tx, item_id).await? {
            return Err(Error::Invalid("Find this treasure in the shop first".into()));
        }
        let item = repo::item(&mut tx, item_id).await?.ok_or(Error::UnknownItem(item_id))?;
        if matches!(item.category.as_str(), "wallpaper" | "outfit" | "room") {
            return Err(Error::Invalid("This treasure cannot be moved around the room".into()));
        }
        let mut home: crate::home::HomeLayout = repo::get_setting(&mut tx, crate::home::SETTING_HOME_LAYOUT).await?
            .map(|value| serde_json::from_str(&value)).transpose()?.unwrap_or_default();
        home.placements.insert(item_id, placement);
        repo::set_setting(&mut tx, crate::home::SETTING_HOME_LAYOUT, &serde_json::to_string(&home)?).await?;
        repo::set_equipped(&mut tx, item_id, true).await?;
        tx.commit().await?;
        self.refresh_snapshot().await
    }

    /// Garden writes and coin changes commit together. Unlocks are enforced here.
    pub async fn garden_action(&self, action: &str, plot: u8, species: Option<&str>) -> Result<Arc<Snapshot>> {
        use crate::home::{Flower, HomeLayout, FLOWER_GROW_SECONDS, GARDEN_LEVEL, SETTING_HOME_LAYOUT};
        if plot >= 6 { return Err(Error::Invalid("Choose one of the six flower beds".into())); }
        let _w = self.write_lock.lock().await;
        let now = self.clock.now_ts();
        let mut tx = self.db.begin().await?;
        let mut avatar = repo::load_avatar(&mut tx).await?.ok_or_else(|| Error::Invalid("Open your home first".into()))?;
        if avatar.level < GARDEN_LEVEL { return Err(Error::LevelRequired(GARDEN_LEVEL)); }
        let mut home: HomeLayout = repo::get_setting(&mut tx, SETTING_HOME_LAYOUT).await?
            .map(|value| serde_json::from_str(&value)).transpose()?.unwrap_or_default();
        match action {
            "plant" => {
                let species = species.filter(|s| ["daisy", "poppy", "moonflower"].contains(s))
                    .ok_or_else(|| Error::Invalid("Choose a flower from the seed pouch".into()))?;
                if home.garden.contains_key(&plot) { return Err(Error::Invalid("A flower is already growing here".into())); }
                if avatar.coins < 8 { return Err(Error::InsufficientCoins { need: 8, have: avatar.coins }); }
                avatar.apply(&Reward { coins: -8, ..Default::default() }, now);
                home.garden.insert(plot, Flower { species: species.into(), planted_at: now, watered_at: None });
            },
            "water" => {
                let flower = home.garden.get_mut(&plot).ok_or_else(|| Error::Invalid("Plant a seed here first".into()))?;
                if flower.watered_at.is_some() { return Err(Error::Invalid("Already watered. Let it grow a little".into())); }
                flower.watered_at = Some(now);
            },
            "harvest" => {
                let flower = home.garden.get(&plot).ok_or_else(|| Error::Invalid("Nothing is growing here yet".into()))?;
                if flower.watered_at.is_none() || now - flower.planted_at < FLOWER_GROW_SECONDS {
                    return Err(Error::Invalid("Water your flower and give it 30 minutes to bloom".into()));
                }
                home.garden.remove(&plot);
                avatar.apply(&Reward { coins: 12, happiness: 2, ..Default::default() }, now);
            },
            _ => return Err(Error::Invalid("Choose plant, water, or pick".into())),
        }
        repo::save_avatar(&mut tx, &avatar).await?;
        repo::set_setting(&mut tx, SETTING_HOME_LAYOUT, &serde_json::to_string(&home)?).await?;
        tx.commit().await?;
        self.refresh_snapshot().await
    }

    pub async fn set_wallpaper(&self, room: &str, asset: &str) -> Result<Arc<Snapshot>> {
        if !crate::home::ROOMS.contains(&room) {
            return Err(Error::Invalid("Choose a room in your cottage".into()));
        }
        let _w = self.write_lock.lock().await;
        let mut tx = self.db.begin().await?;
        if asset != "wallpapers/linen" && !repo::inventory(&mut tx).await?.iter().any(|owned| owned.item.category == "wallpaper" && owned.item.asset_path == asset) {
            return Err(Error::Invalid("Find this wallpaper in the shop first".into()));
        }
        let mut home: crate::home::HomeLayout = repo::get_setting(&mut tx, crate::home::SETTING_HOME_LAYOUT).await?
            .map(|value| serde_json::from_str(&value)).transpose()?.unwrap_or_default();
        home.wallpapers.insert(room.to_string(), asset.to_string());
        repo::set_setting(&mut tx, crate::home::SETTING_HOME_LAYOUT, &serde_json::to_string(&home)?).await?;
        tx.commit().await?;
        self.refresh_snapshot().await
    }

    /// Builds the plan for `date` if it doesn't exist yet. Returns the tasks either way.
    pub async fn generate_daily_plan(&self, date: NaiveDate) -> Result<Vec<DailyTask>> {
        let _w = self.write_lock.lock().await;
        let key = date.to_string();
        let now = self.clock.now_ts();
        let mut tx = self.db.begin().await?;
        let existing = repo::tasks_for_date(&mut tx, &key).await?;
        if !existing.is_empty() {
            return Ok(existing);
        }
        let topics = repo::all_topics(&mut tx).await?;
        let due = repo::due_reviews(&mut tx, at_local(date.succ_opt().unwrap_or(date), 0, 0), 5).await?;
        let suggested = self.recommend_with(&mut tx, &Filter::default(), 1).await?.into_iter().next().map(|r| r.problem.topic);
        let specs = plan::plan_day(&plan::PlanInput { date, topics: &topics, due_reviews: &due, suggested_topic: suggested.as_deref() });

        let mut fx = Effects::default();
        for spec in &specs {
            let deadline = spec.slot.deadline_hour().map(|h| at_local(date, h, 0));
            let id = repo::insert_task(
                &mut tx,
                &NewTask { date: &key, slot: spec.slot, kind: &spec.kind, title: &spec.title, target: spec.target, deadline, reward_xp: spec.reward_xp, reward_coins: spec.reward_coins },
            )
            .await?;
            if let Some(dl) = deadline {
                if dl > now {
                    fx.schedule(&mut tx, JobKind::TaskDeadline { task_id: id }, dl).await?;
                }
                if dl - WARNING_LEAD_SECS > now {
                    fx.schedule(&mut tx, JobKind::TaskWarning { task_id: id }, dl - WARNING_LEAD_SECS).await?;
                }
                if spec.slot == Slot::Breakfast && at_local(date, 9, 0) > now {
                    fx.schedule(&mut tx, JobKind::MorningReminder, at_local(date, 9, 0)).await?;
                }
            }
        }
        fx.event(&mut tx, AppEvent::DailyPlanCreated { date: key.clone(), task_count: specs.len() }, now).await?;
        let tasks = repo::tasks_for_date(&mut tx, &key).await?;
        tx.commit().await?;
        self.after_commit(fx).await?;
        Ok(tasks)
    }

    /// Closes out `date`: pending tasks become missed, the avatar sleeps, the
    /// streak is dropped if the grace day has passed.
    pub async fn end_of_day(&self, date: NaiveDate) -> Result<()> {
        let _w = self.write_lock.lock().await;
        let key = date.to_string();
        let now = self.clock.now_ts();
        let mut tx = self.db.begin().await?;
        if repo::get_setting(&mut tx, SETTING_LAST_DAY_ENDED).await?.as_deref().map_or(false, |d| d >= key.as_str()) {
            return Ok(());
        }
        let mut fx = Effects::default();
        let mut breakfast_done = false;
        for mut task in repo::tasks_for_date(&mut tx, &key).await? {
            breakfast_done |= task.slot == Slot::Breakfast && task.status == TaskStatus::Done;
            if task.status == TaskStatus::Pending {
                task.status = TaskStatus::Missed;
                repo::update_task_progress(&mut tx, &task).await?;
                fx.cancelled_jobs.extend(repo::cancel_jobs_for_task(&mut tx, task.id).await?);
            }
        }
        let solved = repo::solved_count_on(&mut tx, at_local(date, 0, 0), at_local(date.succ_opt().unwrap_or(date), 0, 0)).await?;
        let mut avatar = repo::load_avatar(&mut tx).await?.unwrap_or_else(|| AvatarState::fresh(&key, now));
        game::overnight(&mut avatar, breakfast_done, solved > 0, now);
        let gap = avatar.last_active_date.as_deref().and_then(|d| days_between(d, &key)).unwrap_or(i64::MAX);
        if avatar.streak > 0 && gap >= 2 {
            avatar.streak = 0;
            fx.event(&mut tx, AppEvent::StreakUpdated { streak: 0, best: avatar.best_streak }, now).await?;
        }
        repo::save_avatar(&mut tx, &avatar).await?;
        fx.event(&mut tx, AppEvent::DayEnded { date: key.clone(), solved, breakfast_done }, now).await?;
        repo::set_setting(&mut tx, SETTING_LAST_DAY_ENDED, &key).await?;
        tx.commit().await?;
        self.after_commit(fx).await?;
        Ok(())
    }

    async fn mark_task_missed(&self, task_id: i64) -> Result<()> {
        let _w = self.write_lock.lock().await;
        let now = self.clock.now_ts();
        let mut tx = self.db.begin().await?;
        let Some(mut task) = repo::task_by_id(&mut tx, task_id).await? else { return Ok(()) };
        if task.status != TaskStatus::Pending {
            return Ok(());
        }
        task.status = TaskStatus::Missed;
        repo::update_task_progress(&mut tx, &task).await?;
        let mut fx = Effects::default();
        fx.cancelled_jobs.extend(repo::cancel_jobs_for_task(&mut tx, task.id).await?);
        fx.event(&mut tx, AppEvent::TaskMissed { task_id, slot: task.slot, title: task.title.clone() }, now).await?;
        tx.commit().await?;
        self.after_commit(fx).await?;
        Ok(())
    }

    async fn warn_task(&self, task_id: i64) -> Result<()> {
        let mut c = self.db.conn().await?;
        let Some(task) = repo::task_by_id(&mut c, task_id).await? else { return Ok(()) };
        if task.status != TaskStatus::Pending {
            return Ok(());
        }
        let minutes_left = task.deadline.map(|d| ((d - self.clock.now_ts()) / 60).max(0)).unwrap_or(0);
        self.bus.publish(AppEvent::TaskDue { task_id, slot: task.slot, title: task.title, minutes_left }).await;
        Ok(())
    }

    /// Imports attempts from an external source, skipping ones already seen.
    pub async fn ingest_external(&self, attempts: Vec<ExternalAttempt>) -> Result<usize> {
        let mut imported = 0;
        for a in attempts {
            let dup = {
                let mut c = self.db.conn().await?;
                repo::attempt_exists_external(&mut c, &a.external_id).await?
            };
            if dup {
                continue;
            }
            self.log_attempt(LogAttemptInput {
                problem_id: None,
                slug: Some(a.slug),
                title: Some(a.title),
                difficulty: a.difficulty.or(Some(Difficulty::Medium)),
                topic: a.topic,
                result: a.result,
                duration_ms: a.duration_ms,
                at: Some(a.attempted_at),
                source: Some(a.source),
                external_id: Some(a.external_id),
            })
            .await?;
            imported += 1;
        }
        Ok(imported)
    }

    pub async fn sync_activity(&self) -> SyncReport {
        let Some(provider) = self.provider.read().clone() else {
            return SyncReport { provider: "manual".into(), fetched: 0, imported: 0, error: None };
        };
        let name = provider.name().to_string();
        let known: HashSet<String> = match self.db.conn().await {
            Ok(mut c) => repo::all_problems(&mut c).await.map(|ps| ps.into_iter().map(|p| p.slug).collect()).unwrap_or_default(),
            Err(_) => HashSet::new(),
        };
        match provider.fetch_recent(&known).await {
            Ok(attempts) => {
                let fetched = attempts.len();
                match self.ingest_external(attempts).await {
                    Ok(imported) => SyncReport { provider: name, fetched, imported, error: None },
                    Err(e) => SyncReport { provider: name, fetched, imported: 0, error: Some(e.to_string()) },
                }
            }
            Err(e) => SyncReport { provider: name, fetched: 0, imported: 0, error: Some(e.to_string()) },
        }
    }

    /// Everything that must happen only once the transaction is durable.
    async fn after_commit(&self, fx: Effects) -> Result<Arc<Snapshot>> {
        for id in fx.cancelled_jobs {
            self.scheduler.cancel(id);
        }
        for job in fx.jobs {
            self.scheduler.schedule(job);
        }
        let snapshot = self.refresh_snapshot().await?;
        self.bus.publish_all(fx.events).await;
        Ok(snapshot)
    }

    async fn reschedule(&self, kind: JobKind, run_at: i64) -> Result<()> {
        let _w = self.write_lock.lock().await;
        let mut c = self.db.conn().await?;
        if repo::has_pending_job(&mut c, &kind).await? {
            return Ok(());
        }
        let job = repo::insert_job(&mut c, &kind, run_at).await?;
        self.scheduler.schedule(job);
        Ok(())
    }

    async fn finish_job(&self, id: i64) -> Result<()> {
        let _w = self.write_lock.lock().await;
        let mut c = self.db.conn().await?;
        repo::mark_job(&mut c, id, "done").await
    }
}

struct Applied {
    attempt_id: i64,
    problem: ProblemMeta,
    reward: Reward,
    completed_tasks: Vec<DailyTask>,
    level_up: Option<i64>,
    streak: i64,
    is_review: bool,
    next_review_at: i64,
    achievements: Vec<String>,
    headline: String,
}

#[async_trait::async_trait]
impl JobHandler for Engine {
    async fn handle_job(&self, job: ScheduledJob) -> Result<()> {
        self.finish_job(job.id).await?;
        let now = self.clock.now();
        match job.kind {
            JobKind::GenerateDailyPlan => {
                self.catch_up_days().await?;
                self.generate_daily_plan(now.date_naive()).await?;
                self.reschedule(JobKind::GenerateDailyPlan, next_daily(now, 0, 0)).await
            }
            JobKind::MorningReminder => {
                let today = self.clock.today();
                let mut c = self.db.conn().await?;
                let tasks = repo::tasks_for_date(&mut c, &today).await?;
                drop(c);
                if let Some(t) = tasks.iter().find(|t| t.slot == Slot::Breakfast && t.status == TaskStatus::Pending) {
                    self.warn_task(t.id).await?;
                }
                Ok(())
            }
            JobKind::TaskWarning { task_id } => self.warn_task(task_id).await,
            JobKind::TaskDeadline { task_id } => self.mark_task_missed(task_id).await,
            JobKind::EndOfDay => {
                self.catch_up_days().await?;
                self.reschedule(JobKind::EndOfDay, next_daily(now, 0, 0)).await
            }
            JobKind::ReviewDue { .. } => {
                self.refresh_snapshot().await?;
                Ok(())
            }
            JobKind::FlushMetrics => {
                self.flush_metrics().await?;
                self.reschedule(JobKind::FlushMetrics, now.timestamp() + METRICS_FLUSH_SECS).await
            }
            JobKind::SyncActivity => {
                let report = self.sync_activity().await;
                if let Some(e) = report.error {
                    tracing::warn!(provider = %report.provider, error = %e, "activity sync failed");
                } else if report.imported > 0 {
                    tracing::info!(provider = %report.provider, imported = report.imported, "activity synced");
                }
                self.reschedule(JobKind::SyncActivity, now.timestamp() + SYNC_SECS).await
            }
        }
    }
}

fn room_level_for(asset_path: &str) -> Option<i64> {
    match asset_path {
        "rooms/studio" => Some(1),
        "rooms/loft" => Some(2),
        _ => None,
    }
}

fn titleize(slug: &str) -> String {
    slug.split('-')
        .filter(|s| !s.is_empty())
        .map(|w| {
            let mut cs = w.chars();
            match cs.next() {
                Some(f) => f.to_uppercase().collect::<String>() + cs.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titleize_slug() {
        assert_eq!(titleize("number-of-islands"), "Number Of Islands");
        assert_eq!(titleize("lru-cache"), "Lru Cache");
    }

    #[test]
    fn room_levels() {
        assert_eq!(room_level_for("rooms/loft"), Some(2));
        assert_eq!(room_level_for("furniture/desk"), None);
    }
}
