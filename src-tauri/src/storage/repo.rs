//! Every SQL statement in LeetLife lives here.
use crate::error::Result;
use crate::game::AvatarState;
use crate::metrics::MetricSummary;
use crate::models::*;
use crate::recommendation::{ReviewState, TopicScore};
use crate::scheduler::{JobKind, ScheduledJob};
use sqlx::SqliteConnection;
use std::collections::HashMap;

// ---------------------------------------------------------------- problems

pub struct NewProblem<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub difficulty: Difficulty,
    pub topic: &'a str,
    pub importance: f64,
    pub leetcode_id: Option<i64>,
}

pub async fn upsert_problem(c: &mut SqliteConnection, p: &NewProblem<'_>) -> Result<i64> {
    sqlx::query(
        "INSERT INTO problems (slug, title, difficulty, topic, importance, leetcode_id)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(slug) DO UPDATE SET title = excluded.title, difficulty = excluded.difficulty,
             topic = excluded.topic, importance = excluded.importance,
             leetcode_id = COALESCE(excluded.leetcode_id, problems.leetcode_id)",
    )
    .bind(p.slug)
    .bind(p.title)
    .bind(p.difficulty.as_str())
    .bind(p.topic)
    .bind(p.importance)
    .bind(p.leetcode_id)
    .execute(&mut *c)
    .await?;
    let id: i64 = sqlx::query_scalar("SELECT id FROM problems WHERE slug = ?").bind(p.slug).fetch_one(&mut *c).await?;
    Ok(id)
}

const PROBLEM_COLS: &str = "id, slug, title, difficulty, topic, importance";

pub async fn all_problems(c: &mut SqliteConnection) -> Result<Vec<ProblemMeta>> {
    Ok(sqlx::query_as::<_, ProblemMeta>(&format!("SELECT {PROBLEM_COLS} FROM problems ORDER BY id")).fetch_all(c).await?)
}

pub async fn count_problems(c: &mut SqliteConnection) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM problems").fetch_one(c).await?)
}

pub async fn problem_by_id(c: &mut SqliteConnection, id: i64) -> Result<Option<ProblemMeta>> {
    Ok(sqlx::query_as::<_, ProblemMeta>(&format!("SELECT {PROBLEM_COLS} FROM problems WHERE id = ?"))
        .bind(id)
        .fetch_optional(c)
        .await?)
}

pub async fn problem_by_slug(c: &mut SqliteConnection, slug: &str) -> Result<Option<ProblemMeta>> {
    Ok(sqlx::query_as::<_, ProblemMeta>(&format!("SELECT {PROBLEM_COLS} FROM problems WHERE slug = ?"))
        .bind(slug)
        .fetch_optional(c)
        .await?)
}

pub async fn search_problems(c: &mut SqliteConnection, q: &str, limit: i64) -> Result<Vec<ProblemMeta>> {
    let like = format!("%{}%", q.trim().to_lowercase());
    Ok(sqlx::query_as::<_, ProblemMeta>(&format!(
        "SELECT {PROBLEM_COLS} FROM problems
         WHERE lower(title) LIKE ? OR lower(slug) LIKE ? OR lower(topic) LIKE ? OR CAST(leetcode_id AS TEXT) = ?
         ORDER BY importance DESC, id LIMIT ?"
    ))
    .bind(&like)
    .bind(&like)
    .bind(&like)
    .bind(q.trim())
    .bind(limit)
    .fetch_all(c)
    .await?)
}

// ---------------------------------------------------------------- attempts

pub struct NewAttempt<'a> {
    pub problem_id: i64,
    pub difficulty: Difficulty,
    pub result: AttemptResult,
    pub duration_ms: Option<i64>,
    pub attempted_at: i64,
    pub topic: &'a str,
    pub source: &'a str,
    pub external_id: Option<&'a str>,
}

pub async fn insert_attempt(c: &mut SqliteConnection, a: &NewAttempt<'_>) -> Result<i64> {
    let r = sqlx::query(
        "INSERT INTO attempts (problem_id, difficulty, result, duration_ms, attempted_at, topic, source, external_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(a.problem_id)
    .bind(a.difficulty.as_str())
    .bind(a.result.as_str())
    .bind(a.duration_ms)
    .bind(a.attempted_at)
    .bind(a.topic)
    .bind(a.source)
    .bind(a.external_id)
    .execute(c)
    .await?;
    Ok(r.last_insert_rowid())
}

pub async fn attempt_exists_external(c: &mut SqliteConnection, external_id: &str) -> Result<bool> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM attempts WHERE external_id = ?").bind(external_id).fetch_one(c).await?;
    Ok(n > 0)
}

/// problem_id -> attempts since `since` (for the recommendation exposure term).
pub async fn recent_attempt_counts(c: &mut SqliteConnection, since: i64) -> Result<HashMap<i64, u32>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as("SELECT problem_id, COUNT(*) FROM attempts WHERE attempted_at >= ? GROUP BY problem_id")
        .bind(since)
        .fetch_all(c)
        .await?;
    Ok(rows.into_iter().map(|(p, n)| (p, n as u32)).collect())
}

pub async fn count_attempts(c: &mut SqliteConnection) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM attempts").fetch_one(c).await?)
}

pub async fn count_distinct_solved(c: &mut SqliteConnection) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(DISTINCT problem_id) FROM attempts WHERE result = 'solved'").fetch_one(c).await?)
}

pub async fn count_solved_where(c: &mut SqliteConnection, difficulty: Option<Difficulty>) -> Result<i64> {
    Ok(match difficulty {
        Some(d) => sqlx::query_scalar("SELECT COUNT(*) FROM attempts WHERE result = 'solved' AND difficulty = ?").bind(d.as_str()).fetch_one(c).await?,
        None => sqlx::query_scalar("SELECT COUNT(*) FROM attempts WHERE result = 'solved'").fetch_one(c).await?,
    })
}

pub async fn solved_by_difficulty(c: &mut SqliteConnection) -> Result<Vec<(String, i64)>> {
    Ok(sqlx::query_as("SELECT difficulty, COUNT(*) FROM attempts WHERE result = 'solved' GROUP BY difficulty ORDER BY difficulty")
        .fetch_all(c)
        .await?)
}

/// Solved / failed counts per local day for `[from_ts, to_ts)`. Days with no
/// activity are filled in by the caller so the chart has a bar per day.
pub async fn attempts_between(c: &mut SqliteConnection, from_ts: i64, to_ts: i64) -> Result<Vec<(i64, String)>> {
    Ok(sqlx::query_as("SELECT attempted_at, result FROM attempts WHERE attempted_at >= ? AND attempted_at < ? ORDER BY attempted_at")
        .bind(from_ts)
        .bind(to_ts)
        .fetch_all(c)
        .await?)
}

pub async fn solved_count_on(c: &mut SqliteConnection, from_ts: i64, to_ts: i64) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM attempts WHERE result = 'solved' AND attempted_at >= ? AND attempted_at < ?")
        .bind(from_ts)
        .bind(to_ts)
        .fetch_one(c)
        .await?)
}

pub async fn recent_attempts(c: &mut SqliteConnection, limit: i64) -> Result<Vec<AttemptRow>> {
    Ok(sqlx::query_as::<_, AttemptRow>(
        "SELECT id, problem_id, difficulty, result, duration_ms, attempted_at, topic, source FROM attempts ORDER BY attempted_at DESC, id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(c)
    .await?)
}

// ---------------------------------------------------------------- topics

pub async fn topic_score(c: &mut SqliteConnection, topic: &str) -> Result<Option<TopicScore>> {
    Ok(sqlx::query_as::<_, TopicScore>("SELECT topic, confidence, attempts, failures, last_reviewed FROM topic_scores WHERE topic = ?")
        .bind(topic)
        .fetch_optional(c)
        .await?)
}

pub async fn upsert_topic(c: &mut SqliteConnection, t: &TopicScore) -> Result<()> {
    sqlx::query(
        "INSERT INTO topic_scores (topic, confidence, attempts, failures, last_reviewed) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(topic) DO UPDATE SET confidence = excluded.confidence, attempts = excluded.attempts,
             failures = excluded.failures, last_reviewed = excluded.last_reviewed",
    )
    .bind(&t.topic)
    .bind(t.confidence)
    .bind(t.attempts)
    .bind(t.failures)
    .bind(t.last_reviewed)
    .execute(c)
    .await?;
    Ok(())
}

pub async fn all_topics(c: &mut SqliteConnection) -> Result<Vec<TopicScore>> {
    Ok(sqlx::query_as::<_, TopicScore>("SELECT topic, confidence, attempts, failures, last_reviewed FROM topic_scores ORDER BY confidence ASC, topic")
        .fetch_all(c)
        .await?)
}

/// Distinct topics from the problem catalogue (so the UI can list them even before any attempts).
pub async fn catalogue_topics(c: &mut SqliteConnection) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar("SELECT DISTINCT topic FROM problems ORDER BY topic").fetch_all(c).await?)
}

// ---------------------------------------------------------------- reviews

const REVIEW_COLS: &str = "problem_id, interval_idx, ease, due_at, last_reviewed, reviews, lapses";

pub async fn review_state(c: &mut SqliteConnection, problem_id: i64) -> Result<Option<ReviewState>> {
    Ok(sqlx::query_as::<_, ReviewState>(&format!("SELECT {REVIEW_COLS} FROM review_schedule WHERE problem_id = ?"))
        .bind(problem_id)
        .fetch_optional(c)
        .await?)
}

pub async fn upsert_review(c: &mut SqliteConnection, r: &ReviewState) -> Result<()> {
    sqlx::query(
        "INSERT INTO review_schedule (problem_id, interval_idx, ease, due_at, last_reviewed, reviews, lapses)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(problem_id) DO UPDATE SET interval_idx = excluded.interval_idx, ease = excluded.ease,
             due_at = excluded.due_at, last_reviewed = excluded.last_reviewed, reviews = excluded.reviews, lapses = excluded.lapses",
    )
    .bind(r.problem_id)
    .bind(r.interval_idx)
    .bind(r.ease)
    .bind(r.due_at)
    .bind(r.last_reviewed)
    .bind(r.reviews)
    .bind(r.lapses)
    .execute(c)
    .await?;
    Ok(())
}

pub async fn all_reviews(c: &mut SqliteConnection) -> Result<Vec<ReviewState>> {
    Ok(sqlx::query_as::<_, ReviewState>(&format!("SELECT {REVIEW_COLS} FROM review_schedule")).fetch_all(c).await?)
}

pub async fn due_reviews(c: &mut SqliteConnection, now: i64, limit: i64) -> Result<Vec<ProblemMeta>> {
    Ok(sqlx::query_as::<_, ProblemMeta>(
        "SELECT p.id, p.slug, p.title, p.difficulty, p.topic, p.importance
         FROM review_schedule r JOIN problems p ON p.id = r.problem_id
         WHERE r.due_at <= ? ORDER BY r.due_at ASC LIMIT ?",
    )
    .bind(now)
    .bind(limit)
    .fetch_all(c)
    .await?)
}

pub async fn count_due_reviews(c: &mut SqliteConnection, now: i64) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM review_schedule WHERE due_at <= ?").bind(now).fetch_one(c).await?)
}

pub async fn next_review_due(c: &mut SqliteConnection, after: i64) -> Result<Option<(i64, i64)>> {
    Ok(sqlx::query_as("SELECT problem_id, due_at FROM review_schedule WHERE due_at > ? ORDER BY due_at ASC LIMIT 1")
        .bind(after)
        .fetch_optional(c)
        .await?)
}

// ---------------------------------------------------------------- daily tasks

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: i64,
    date: String,
    slot: String,
    kind: String,
    title: String,
    target: i64,
    progress: i64,
    status: String,
    deadline: Option<i64>,
    reward_xp: i64,
    reward_coins: i64,
    completed_at: Option<i64>,
}

impl TaskRow {
    fn into_task(self) -> Result<DailyTask> {
        Ok(DailyTask {
            id: self.id,
            date: self.date,
            slot: Slot::parse(&self.slot).ok_or_else(|| crate::Error::Invalid(format!("bad slot {}", self.slot)))?,
            kind: serde_json::from_str(&self.kind)?,
            title: self.title,
            target: self.target,
            progress: self.progress,
            status: TaskStatus::parse(&self.status),
            deadline: self.deadline,
            reward_xp: self.reward_xp,
            reward_coins: self.reward_coins,
            completed_at: self.completed_at,
        })
    }
}

const TASK_COLS: &str = "id, date, slot, kind, title, target, progress, status, deadline, reward_xp, reward_coins, completed_at";

pub async fn tasks_for_date(c: &mut SqliteConnection, date: &str) -> Result<Vec<DailyTask>> {
    let rows = sqlx::query_as::<_, TaskRow>(&format!(
        "SELECT {TASK_COLS} FROM daily_tasks WHERE date = ?
         ORDER BY CASE slot WHEN 'breakfast' THEN 0 WHEN 'training' THEN 1 WHEN 'work' THEN 2 ELSE 3 END"
    ))
    .bind(date)
    .fetch_all(c)
    .await?;
    rows.into_iter().map(TaskRow::into_task).collect()
}

pub async fn task_by_id(c: &mut SqliteConnection, id: i64) -> Result<Option<DailyTask>> {
    let row = sqlx::query_as::<_, TaskRow>(&format!("SELECT {TASK_COLS} FROM daily_tasks WHERE id = ?")).bind(id).fetch_optional(c).await?;
    row.map(TaskRow::into_task).transpose()
}

pub struct NewTask<'a> {
    pub date: &'a str,
    pub slot: Slot,
    pub kind: &'a TaskKind,
    pub title: &'a str,
    pub target: i64,
    pub deadline: Option<i64>,
    pub reward_xp: i64,
    pub reward_coins: i64,
}

pub async fn insert_task(c: &mut SqliteConnection, t: &NewTask<'_>) -> Result<i64> {
    let r = sqlx::query(
        "INSERT INTO daily_tasks (date, slot, kind, title, target, deadline, reward_xp, reward_coins)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(t.date)
    .bind(t.slot.as_str())
    .bind(serde_json::to_string(t.kind)?)
    .bind(t.title)
    .bind(t.target)
    .bind(t.deadline)
    .bind(t.reward_xp)
    .bind(t.reward_coins)
    .execute(c)
    .await?;
    Ok(r.last_insert_rowid())
}

pub async fn update_task_progress(c: &mut SqliteConnection, t: &DailyTask) -> Result<()> {
    sqlx::query("UPDATE daily_tasks SET progress = ?, status = ?, completed_at = ? WHERE id = ?")
        .bind(t.progress)
        .bind(t.status.as_str())
        .bind(t.completed_at)
        .bind(t.id)
        .execute(c)
        .await?;
    Ok(())
}

/// Counts fully completed days (every slot done) for the "perfect day" achievement.
pub async fn perfect_days(c: &mut SqliteConnection) -> Result<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COUNT(*) FROM (SELECT date FROM daily_tasks GROUP BY date HAVING SUM(status = 'done') = COUNT(*) AND COUNT(*) >= 4)",
    )
    .fetch_one(c)
    .await?)
}

// ---------------------------------------------------------------- avatar

const AVATAR_COLS: &str = "hunger, energy, happiness, xp, level, coins, streak, best_streak, last_active_date, started_on, updated_at";

pub async fn load_avatar(c: &mut SqliteConnection) -> Result<Option<AvatarState>> {
    Ok(sqlx::query_as::<_, AvatarState>(&format!("SELECT {AVATAR_COLS} FROM avatar_state WHERE id = 1")).fetch_optional(c).await?)
}

pub async fn save_avatar(c: &mut SqliteConnection, s: &AvatarState) -> Result<()> {
    sqlx::query(
        "INSERT INTO avatar_state (id, hunger, energy, happiness, xp, level, coins, streak, best_streak, last_active_date, started_on, updated_at)
         VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET hunger = excluded.hunger, energy = excluded.energy, happiness = excluded.happiness,
             xp = excluded.xp, level = excluded.level, coins = excluded.coins, streak = excluded.streak,
             best_streak = excluded.best_streak, last_active_date = excluded.last_active_date,
             started_on = excluded.started_on, updated_at = excluded.updated_at",
    )
    .bind(s.hunger)
    .bind(s.energy)
    .bind(s.happiness)
    .bind(s.xp)
    .bind(s.level)
    .bind(s.coins)
    .bind(s.streak)
    .bind(s.best_streak)
    .bind(&s.last_active_date)
    .bind(&s.started_on)
    .bind(s.updated_at)
    .execute(c)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------- items / inventory / rooms

pub async fn upsert_item(c: &mut SqliteConnection, i: &ItemDef) -> Result<()> {
    sqlx::query(
        "INSERT INTO items (id, name, category, cost, asset_path, description, min_level) VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, category = excluded.category, cost = excluded.cost,
             asset_path = excluded.asset_path, description = excluded.description, min_level = excluded.min_level",
    )
    .bind(i.id)
    .bind(&i.name)
    .bind(&i.category)
    .bind(i.cost)
    .bind(&i.asset_path)
    .bind(&i.description)
    .bind(i.min_level)
    .execute(c)
    .await?;
    Ok(())
}

const ITEM_COLS: &str = "id, name, category, cost, asset_path, description, min_level";

pub async fn all_items(c: &mut SqliteConnection) -> Result<Vec<ItemDef>> {
    Ok(sqlx::query_as::<_, ItemDef>(&format!("SELECT {ITEM_COLS} FROM items ORDER BY min_level, cost, id")).fetch_all(c).await?)
}

pub async fn item(c: &mut SqliteConnection, id: i64) -> Result<Option<ItemDef>> {
    Ok(sqlx::query_as::<_, ItemDef>(&format!("SELECT {ITEM_COLS} FROM items WHERE id = ?")).bind(id).fetch_optional(c).await?)
}

#[derive(sqlx::FromRow)]
struct OwnedRow {
    id: i64,
    name: String,
    category: String,
    cost: i64,
    asset_path: String,
    description: String,
    min_level: i64,
    acquired_at: i64,
    equipped: i64,
}

pub async fn inventory(c: &mut SqliteConnection) -> Result<Vec<OwnedItem>> {
    let rows = sqlx::query_as::<_, OwnedRow>(
        "SELECT i.id, i.name, i.category, i.cost, i.asset_path, i.description, i.min_level, inv.acquired_at, inv.equipped
         FROM inventory inv JOIN items i ON i.id = inv.item_id ORDER BY inv.acquired_at, i.id",
    )
    .fetch_all(c)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| OwnedItem {
            item: ItemDef { id: r.id, name: r.name, category: r.category, cost: r.cost, asset_path: r.asset_path, description: r.description, min_level: r.min_level },
            acquired_at: r.acquired_at,
            equipped: r.equipped != 0,
        })
        .collect())
}

pub async fn owns_item(c: &mut SqliteConnection, item_id: i64) -> Result<bool> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM inventory WHERE item_id = ?").bind(item_id).fetch_one(c).await?;
    Ok(n > 0)
}

pub async fn insert_inventory(c: &mut SqliteConnection, item_id: i64, at: i64) -> Result<()> {
    sqlx::query("INSERT INTO inventory (item_id, acquired_at, equipped) VALUES (?, ?, 1)").bind(item_id).bind(at).execute(c).await?;
    Ok(())
}

pub async fn set_equipped(c: &mut SqliteConnection, item_id: i64, equipped: bool) -> Result<bool> {
    let r = sqlx::query("UPDATE inventory SET equipped = ? WHERE item_id = ?").bind(equipped as i64).bind(item_id).execute(c).await?;
    Ok(r.rows_affected() > 0)
}

pub async fn room_level(c: &mut SqliteConnection) -> Result<i64> {
    let v: Option<i64> = sqlx::query_scalar("SELECT upgrade_level FROM rooms WHERE room_id = 'main'").fetch_optional(c).await?;
    Ok(v.unwrap_or(0))
}

pub async fn set_room_level(c: &mut SqliteConnection, level: i64) -> Result<()> {
    sqlx::query("INSERT INTO rooms (room_id, upgrade_level) VALUES ('main', ?) ON CONFLICT(room_id) DO UPDATE SET upgrade_level = excluded.upgrade_level")
        .bind(level)
        .execute(c)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------- events / achievements

pub async fn insert_event(c: &mut SqliteConnection, kind: &str, payload: &str, at: i64) -> Result<i64> {
    let r = sqlx::query("INSERT INTO events (kind, payload, created_at) VALUES (?, ?, ?)").bind(kind).bind(payload).bind(at).execute(c).await?;
    Ok(r.last_insert_rowid())
}

pub async fn recent_events(c: &mut SqliteConnection, limit: i64) -> Result<Vec<EventRow>> {
    Ok(sqlx::query_as::<_, EventRow>("SELECT id, kind, payload, created_at FROM events ORDER BY id DESC LIMIT ?").bind(limit).fetch_all(c).await?)
}

pub async fn count_events(c: &mut SqliteConnection) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM events").fetch_one(c).await?)
}

pub async fn has_achievement(c: &mut SqliteConnection, key: &str) -> Result<bool> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM achievements WHERE key = ?").bind(key).fetch_one(c).await?;
    Ok(n > 0)
}

pub async fn insert_achievement(c: &mut SqliteConnection, key: &str, title: &str, at: i64) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO achievements (key, title, unlocked_at) VALUES (?, ?, ?)").bind(key).bind(title).bind(at).execute(c).await?;
    Ok(())
}

pub async fn all_achievements(c: &mut SqliteConnection) -> Result<Vec<Achievement>> {
    Ok(sqlx::query_as::<_, Achievement>("SELECT key, title, unlocked_at FROM achievements ORDER BY unlocked_at DESC").fetch_all(c).await?)
}

// ---------------------------------------------------------------- scheduled jobs

pub async fn insert_job(c: &mut SqliteConnection, kind: &JobKind, run_at: i64) -> Result<ScheduledJob> {
    let r = sqlx::query("INSERT INTO scheduled_jobs (kind, run_at, status) VALUES (?, ?, 'pending')")
        .bind(serde_json::to_string(kind)?)
        .bind(run_at)
        .execute(c)
        .await?;
    Ok(ScheduledJob { id: r.last_insert_rowid(), kind: kind.clone(), run_at })
}

pub async fn pending_jobs(c: &mut SqliteConnection) -> Result<Vec<ScheduledJob>> {
    let rows: Vec<(i64, String, i64)> = sqlx::query_as("SELECT id, kind, run_at FROM scheduled_jobs WHERE status = 'pending' ORDER BY run_at, id")
        .fetch_all(c)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for (id, kind, run_at) in rows {
        match serde_json::from_str::<JobKind>(&kind) {
            Ok(kind) => out.push(ScheduledJob { id, kind, run_at }),
            Err(e) => tracing::warn!(id, error = %e, "skipping unreadable job"),
        }
    }
    Ok(out)
}

pub async fn mark_job(c: &mut SqliteConnection, id: i64, status: &str) -> Result<()> {
    sqlx::query("UPDATE scheduled_jobs SET status = ? WHERE id = ?").bind(status).bind(id).execute(c).await?;
    Ok(())
}

/// Cancels every pending job that references `task_id` (deadline + warning). Returns the ids.
pub async fn cancel_jobs_for_task(c: &mut SqliteConnection, task_id: i64) -> Result<Vec<i64>> {
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM scheduled_jobs WHERE status = 'pending' AND json_extract(kind, '$.task_id') = ?")
        .bind(task_id)
        .fetch_all(&mut *c)
        .await?;
    sqlx::query("UPDATE scheduled_jobs SET status = 'cancelled' WHERE status = 'pending' AND json_extract(kind, '$.task_id') = ?")
        .bind(task_id)
        .execute(&mut *c)
        .await?;
    Ok(ids)
}

/// Is there already a pending job of exactly this kind? (keeps recurring jobs singleton)
pub async fn has_pending_job(c: &mut SqliteConnection, kind: &JobKind) -> Result<bool> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scheduled_jobs WHERE status = 'pending' AND kind = ?")
        .bind(serde_json::to_string(kind)?)
        .fetch_one(c)
        .await?;
    Ok(n > 0)
}

pub async fn prune_finished_jobs(c: &mut SqliteConnection, before: i64) -> Result<u64> {
    let r = sqlx::query("DELETE FROM scheduled_jobs WHERE status != 'pending' AND run_at < ?").bind(before).execute(c).await?;
    Ok(r.rows_affected())
}

// ---------------------------------------------------------------- metrics

pub async fn insert_metrics(c: &mut SqliteConnection, rows: &[MetricSummary], at: i64) -> Result<()> {
    for m in rows {
        sqlx::query("INSERT INTO metrics (name, count, mean_us, p50_us, p95_us, p99_us, max_us, recorded_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&m.name)
            .bind(m.count as i64)
            .bind(m.mean_us)
            .bind(m.p50_us)
            .bind(m.p95_us)
            .bind(m.p99_us)
            .bind(m.max_us)
            .bind(at)
            .execute(&mut *c)
            .await?;
    }
    Ok(())
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct MetricHistoryRow {
    pub name: String,
    pub count: i64,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub recorded_at: i64,
}

pub async fn metric_history(c: &mut SqliteConnection, name: &str, limit: i64) -> Result<Vec<MetricHistoryRow>> {
    Ok(sqlx::query_as::<_, MetricHistoryRow>(
        "SELECT name, count, p50_us, p95_us, p99_us, recorded_at FROM metrics WHERE name = ? ORDER BY recorded_at DESC LIMIT ?",
    )
    .bind(name)
    .bind(limit)
    .fetch_all(c)
    .await?)
}

// ---------------------------------------------------------------- settings

pub async fn get_setting(c: &mut SqliteConnection, key: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT value FROM settings WHERE key = ?").bind(key).fetch_optional(c).await?)
}

pub async fn set_setting(c: &mut SqliteConnection, key: &str, value: &str) -> Result<()> {
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value)
        .execute(c)
        .await?;
    Ok(())
}

pub async fn all_settings(c: &mut SqliteConnection) -> Result<HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings").fetch_all(c).await?;
    Ok(rows.into_iter().collect())
}
