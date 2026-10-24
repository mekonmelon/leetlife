//! End-to-end tests against a real (in-memory) SQLite database and a fixed clock.
use leetlife::activity::ExternalAttempt;
use leetlife::clock::{Clock, FixedClock};
use leetlife::engine::{Engine, LogAttemptInput, SETTING_LAST_DAY_ENDED};
use leetlife::events::AppEvent;
use leetlife::models::{AttemptResult, Difficulty, Slot, TaskStatus};
use leetlife::notifications::MemoryNotifier;
use leetlife::scheduler::JobKind;
use chrono::{Local, TimeZone};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn clock_at(y: i32, m: u32, d: u32, h: u32, min: u32) -> Arc<FixedClock> {
    Arc::new(FixedClock::at(Local.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()))
}

async fn booted(clock: Arc<FixedClock>) -> Arc<Engine> {
    let engine = Engine::in_memory(clock).await.expect("open");
    engine.bootstrap().await.expect("bootstrap");
    engine
}

#[tokio::test]
async fn bootstrap_creates_avatar_plan_and_recurring_jobs() {
    let engine = booted(clock_at(2026, 9, 16, 7, 30)).await;
    let snap = engine.snapshot().await.unwrap();

    assert_eq!(snap.avatar.level, 1);
    assert_eq!(snap.avatar.coins, 100);
    assert_eq!(snap.day_number, 1);
    assert_eq!(snap.weekday, "Wednesday");
    assert_eq!(snap.tasks.len(), 4);
    assert_eq!(snap.tasks.iter().map(|t| t.slot).collect::<Vec<_>>(), Slot::ALL.to_vec());
    assert!(snap.tasks.iter().all(|t| t.status == TaskStatus::Pending));
    assert!(!snap.recommendations.is_empty(), "catalogue should produce recommendations");
    assert!(snap.mood.contains("haven't eaten"), "morning mood: {}", snap.mood);

    let kinds: Vec<JobKind> = engine.scheduler().pending().into_iter().map(|j| j.kind).collect();
    for expected in [JobKind::GenerateDailyPlan, JobKind::EndOfDay, JobKind::FlushMetrics, JobKind::SyncActivity] {
        assert!(kinds.contains(&expected), "missing {expected:?} in {kinds:?}");
    }
    // Breakfast deadline is 10:00 and we booted at 07:30, so a deadline job exists.
    assert!(kinds.iter().any(|k| matches!(k, JobKind::TaskDeadline { .. })));
}

#[tokio::test]
async fn bootstrap_is_idempotent() {
    let clock = clock_at(2026, 9, 16, 7, 30);
    let engine = booted(clock.clone()).await;
    let before = engine.snapshot().await.unwrap().tasks.len();
    engine.bootstrap().await.unwrap();
    let after = engine.snapshot().await.unwrap();
    assert_eq!(after.tasks.len(), before, "second bootstrap must not duplicate the plan");
    assert_eq!(after.avatar.xp, 0);
}

#[tokio::test]
async fn solving_an_easy_completes_breakfast_and_pays_once() {
    let clock = clock_at(2026, 9, 16, 8, 30);
    let engine = booted(clock.clone()).await;

    let out = engine
        .log_attempt(LogAttemptInput { slug: Some("two-sum".into()), result: AttemptResult::Solved, ..Default::default() })
        .await
        .unwrap();

    // Easy solve (10 xp / 15 coins) + breakfast bonus (20 xp / 15 coins).
    assert_eq!(out.reward.xp, 30);
    assert_eq!(out.reward.coins, 30);
    assert_eq!(out.completed_tasks.len(), 1);
    assert_eq!(out.completed_tasks[0].slot, Slot::Breakfast);
    assert_eq!(out.streak, 1);
    assert!(!out.is_review, "first solve is not a review");
    assert!(out.achievements.contains(&"First blood".to_string()));

    let snap = out.snapshot;
    assert_eq!(snap.avatar.xp, 30);
    assert_eq!(snap.avatar.coins, 130);
    assert_eq!(snap.avatar.hunger, 100, "breakfast feeds the avatar (70 + 40, clamped)");
    let breakfast = snap.tasks.iter().find(|t| t.slot == Slot::Breakfast).unwrap();
    assert_eq!(breakfast.status, TaskStatus::Done);

    // A second Easy must not re-pay breakfast, and advances at most one further slot.
    let again = engine
        .log_attempt(LogAttemptInput { slug: Some("valid-anagram".into()), result: AttemptResult::Solved, ..Default::default() })
        .await
        .unwrap();
    assert!(again.completed_tasks.iter().all(|t| t.slot != Slot::Breakfast), "breakfast pays once");
    assert!(again.completed_tasks.len() <= 1, "one attempt advances one slot, got {:?}", again.completed_tasks);
    assert_eq!(again.snapshot.avatar.xp, 30 + again.reward.xp, "xp is exactly the two rewards");
    assert_eq!(again.snapshot.tasks.iter().filter(|t| t.status == TaskStatus::Done).count(), 1 + again.completed_tasks.len());
}

#[tokio::test]
async fn attempt_is_one_transaction_across_every_table() {
    let clock = clock_at(2026, 9, 16, 8, 30);
    let engine = booted(clock.clone()).await;
    engine
        .log_attempt(LogAttemptInput { slug: Some("number-of-islands".into()), result: AttemptResult::Solved, duration_ms: Some(900_000), ..Default::default() })
        .await
        .unwrap();

    let stats = engine.stats().await.unwrap();
    assert_eq!(stats.total_attempts, 1);
    assert_eq!(stats.total_solved, 1);
    let graphs = stats.topics.iter().find(|t| t.topic == "Graphs").expect("topic score written");
    assert!(graphs.confidence > 0.5, "a solve raises confidence: {}", graphs.confidence);
    assert_eq!(graphs.attempts, 1);
    // The event outbox recorded the solve in the same transaction.
    assert!(stats.recent_events.iter().any(|e| e.kind == "problem_solved"));
    assert!(stats.recent_events.iter().any(|e| e.kind == "reward_earned"));
    // And the review ladder started.
    let snap = engine.snapshot().await.unwrap();
    assert_eq!(snap.due_reviews, 0, "a freshly solved problem is not due yet");
}

#[tokio::test]
async fn failed_attempt_costs_no_coins_but_lowers_confidence() {
    let clock = clock_at(2026, 9, 16, 8, 30);
    let engine = booted(clock.clone()).await;
    let out = engine
        .log_attempt(LogAttemptInput { slug: Some("edit-distance".into()), result: AttemptResult::Failed, ..Default::default() })
        .await
        .unwrap();
    assert_eq!(out.reward.coins, 0);
    assert!(out.reward.xp > 0, "effort still pays a little XP");
    assert!(out.completed_tasks.is_empty(), "a failure completes nothing");
    assert_eq!(out.snapshot.avatar.coins, 100);

    let dp = out.snapshot.topics.iter().find(|t| t.topic == "Dynamic Programming").unwrap();
    assert!(dp.confidence < 0.5, "a failure lowers confidence: {}", dp.confidence);
    assert_eq!(dp.failures, 1);
}

#[tokio::test]
async fn review_is_scheduled_then_pays_less_when_it_comes_due() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;
    let first = engine
        .log_attempt(LogAttemptInput { slug: Some("coin-change".into()), result: AttemptResult::Solved, ..Default::default() })
        .await
        .unwrap();
    assert!(first.next_review_at > clock.now_ts(), "review scheduled into the future");

    // Jump past the first interval.
    clock.advance_secs(2 * 86_400);
    engine.refresh_snapshot().await.unwrap();
    assert_eq!(engine.snapshot().await.unwrap().due_reviews, 1);

    let review = engine
        .log_attempt(LogAttemptInput { slug: Some("coin-change".into()), result: AttemptResult::Solved, ..Default::default() })
        .await
        .unwrap();
    assert!(review.is_review);
    assert!(review.reward.xp < first.reward.xp, "review pays less: {} vs {}", review.reward.xp, first.reward.xp);
    assert!(review.next_review_at > first.next_review_at, "interval grows");
}

#[tokio::test]
async fn economy_enforces_coins_level_and_double_purchase() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;

    // Item 1 (Plant) costs 60 and we start with 100 coins.
    let snap = engine.purchase_item(1).await.unwrap();
    assert_eq!(snap.avatar.coins, 40);
    assert_eq!(snap.inventory.len(), 1);
    assert_eq!(snap.inventory[0].item.name, "Whisper fern");

    assert!(matches!(engine.purchase_item(1).await, Err(leetlife::Error::AlreadyOwned)));
    // Desk costs 120, we have 40.
    assert!(matches!(engine.purchase_item(2).await, Err(leetlife::Error::InsufficientCoins { .. })));
    // Gaming PC needs level 4.
    assert!(matches!(engine.purchase_item(8).await, Err(leetlife::Error::LevelRequired(4))));
    // A failed purchase changed nothing.
    assert_eq!(engine.snapshot().await.unwrap().avatar.coins, 40);

    let shop = engine.shop().await.unwrap();
    let plant = shop.iter().find(|s| s.item.id == 1).unwrap();
    assert!(plant.owned);
    let pc = shop.iter().find(|s| s.item.id == 8).unwrap();
    assert!(!pc.unlocked && !pc.affordable);
}

#[tokio::test]
async fn missing_a_deadline_marks_the_task_missed_and_notifies() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;
    let notifier = Arc::new(MemoryNotifier::default());
    let cancel = CancellationToken::new();
    let handles = engine.spawn_workers(notifier.clone(), cancel.clone());

    // Move past the 10:00 breakfast deadline and let the scheduler catch up.
    clock.set(Local.with_ymd_and_hms(2026, 9, 16, 10, 1, 0).unwrap());
    engine.scheduler().wake();
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    let snap = engine.snapshot().await.unwrap();
    let breakfast = snap.tasks.iter().find(|t| t.slot == Slot::Breakfast).unwrap();
    assert_eq!(breakfast.status, TaskStatus::Missed);
    assert!(snap.mood.contains("Skipped breakfast"), "mood: {}", snap.mood);

    let sent = notifier.sent.lock().clone();
    assert!(sent.iter().any(|n| n.title.contains("Breakfast missed")), "notifications: {sent:?}");

    cancel.cancel();
    for h in handles {
        let _ = h.await;
    }
}

#[tokio::test]
async fn end_of_day_resets_the_streak_only_after_the_grace_day() {
    let clock = clock_at(2026, 9, 16, 12, 0);
    let engine = booted(clock.clone()).await;
    engine.log_attempt(LogAttemptInput { slug: Some("two-sum".into()), result: AttemptResult::Solved, ..Default::default() }).await.unwrap();
    assert_eq!(engine.snapshot().await.unwrap().avatar.streak, 1);

    // Close out the 16th: active that day, streak survives.
    engine.end_of_day(chrono::NaiveDate::from_ymd_opt(2026, 9, 16).unwrap()).await.unwrap();
    assert_eq!(engine.snapshot().await.unwrap().avatar.streak, 1);

    // Skip the 17th entirely, then close it: one missed day is forgiven.
    clock.set(Local.with_ymd_and_hms(2026, 9, 17, 23, 59, 0).unwrap());
    engine.end_of_day(chrono::NaiveDate::from_ymd_opt(2026, 9, 17).unwrap()).await.unwrap();
    assert_eq!(engine.snapshot().await.unwrap().avatar.streak, 1, "grace day keeps the streak");

    // Skip the 18th too: now it resets.
    clock.set(Local.with_ymd_and_hms(2026, 9, 18, 23, 59, 0).unwrap());
    engine.end_of_day(chrono::NaiveDate::from_ymd_opt(2026, 9, 18).unwrap()).await.unwrap();
    assert_eq!(engine.snapshot().await.unwrap().avatar.streak, 0);
}

#[tokio::test]
async fn restarting_after_days_away_catches_up_and_builds_todays_plan() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;
    engine.log_attempt(LogAttemptInput { slug: Some("two-sum".into()), result: AttemptResult::Solved, ..Default::default() }).await.unwrap();
    let db = engine.db().clone();
    drop(engine);

    // Same database, three days later, as if the app had been closed.
    let later = clock_at(2026, 9, 19, 8, 5);
    let engine = Engine::from_db(db, later.clone(), Default::default());
    let snap = engine.bootstrap().await.unwrap();

    assert_eq!(snap.date, "2026-09-19");
    assert_eq!(snap.day_number, 4, "day counter runs from the first launch");
    assert_eq!(snap.tasks.len(), 4, "today has a fresh plan");
    assert!(snap.tasks.iter().all(|t| t.date == "2026-09-19"));
    assert_eq!(snap.avatar.streak, 0, "three days away resets the streak");

    let settings = engine.settings().await.unwrap();
    assert_eq!(settings.get(SETTING_LAST_DAY_ENDED).map(String::as_str), Some("2026-09-18"), "caught up through yesterday");
    // The 16th's unfinished tasks were closed out rather than left pending.
    let events = engine.stats().await.unwrap().recent_events;
    assert!(events.iter().filter(|e| e.kind == "day_ended").count() >= 2);
}

#[tokio::test]
async fn external_ingestion_is_idempotent_on_the_external_id() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;
    let attempt = ExternalAttempt {
        external_id: "leetcode:12345".into(),
        slug: "climbing-stairs".into(),
        title: "Climbing Stairs".into(),
        difficulty: Some(Difficulty::Easy),
        topic: Some("Dynamic Programming".into()),
        result: AttemptResult::Solved,
        attempted_at: clock.now_ts(),
        duration_ms: None,
        source: "leetcode".into(),
    };
    assert_eq!(engine.ingest_external(vec![attempt.clone()]).await.unwrap(), 1);
    assert_eq!(engine.ingest_external(vec![attempt.clone()]).await.unwrap(), 0, "same submission must not double-count");
    assert_eq!(engine.stats().await.unwrap().total_attempts, 1);
}

#[tokio::test]
async fn unknown_slugs_are_added_to_the_catalogue_when_described() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;
    // No difficulty: we refuse to guess.
    assert!(engine
        .log_attempt(LogAttemptInput { slug: Some("some-new-problem".into()), result: AttemptResult::Solved, ..Default::default() })
        .await
        .is_err());

    let out = engine
        .log_attempt(LogAttemptInput {
            slug: Some("some-new-problem".into()),
            difficulty: Some(Difficulty::Medium),
            topic: Some("Graphs".into()),
            result: AttemptResult::Solved,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(out.problem.title, "Some New Problem");
    assert_eq!(out.problem.topic, "Graphs");
    assert!(engine.search_problems("some-new", 5).await.unwrap().iter().any(|p| p.slug == "some-new-problem"));
}

#[tokio::test]
async fn events_reach_subscribers_after_the_commit() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;
    let cancel = CancellationToken::new();
    let handles = engine.spawn_workers(Arc::new(MemoryNotifier::default()), cancel.clone());
    // Bootstrap queued its own events before any worker existed; let the
    // dispatcher drain those, then subscribe so we only see the attempt's.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let mut rx = engine.bus().subscribe();

    engine.log_attempt(LogAttemptInput { slug: Some("two-sum".into()), result: AttemptResult::Solved, ..Default::default() }).await.unwrap();

    let mut kinds = Vec::new();
    while let Ok(Ok(ev)) = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await {
        let solved_first = matches!(ev, AppEvent::ProblemSolved { .. });
        kinds.push(ev.kind());
        if kinds.len() == 1 {
            assert!(solved_first, "the solve is published first: {kinds:?}");
        }
        if kinds.len() >= 4 {
            break;
        }
    }
    assert!(kinds.contains(&"daily_goal_completed"), "{kinds:?}");
    assert!(kinds.contains(&"reward_earned"), "{kinds:?}");

    cancel.cancel();
    for h in handles {
        let _ = h.await;
    }
}

#[tokio::test]
async fn concurrent_attempts_serialise_without_losing_rewards() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;
    let slugs = ["two-sum", "valid-anagram", "contains-duplicate", "climbing-stairs", "valid-parentheses", "binary-search", "reverse-linked-list", "invert-binary-tree"];

    let mut tasks = Vec::new();
    for slug in slugs {
        let e = engine.clone();
        tasks.push(tokio::spawn(async move {
            e.log_attempt(LogAttemptInput { slug: Some(slug.into()), result: AttemptResult::Solved, ..Default::default() }).await
        }));
    }
    let mut total_xp = 0;
    for t in tasks {
        total_xp += t.await.unwrap().unwrap().reward.xp;
    }

    let snap = engine.snapshot().await.unwrap();
    assert_eq!(snap.avatar.xp, total_xp, "no reward lost or double-counted under concurrency");
    assert_eq!(engine.stats().await.unwrap().total_attempts, slugs.len() as i64);
}

#[tokio::test]
async fn metrics_are_recorded_and_flushed_to_sqlite() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;
    engine.log_attempt(LogAttemptInput { slug: Some("two-sum".into()), result: AttemptResult::Solved, ..Default::default() }).await.unwrap();
    engine.snapshot().await.unwrap();

    let summary = engine.metrics_summary();
    let names: Vec<&str> = summary.iter().map(|m| m.name.as_str()).collect();
    for expected in ["db.attempt_tx", "ipc.log_attempt", "recommend.generate", "cache.refresh"] {
        assert!(names.contains(&expected), "missing metric {expected} in {names:?}");
    }
    let tx = summary.iter().find(|m| m.name == "db.attempt_tx").unwrap();
    assert!(tx.p95_us >= tx.p50_us && tx.max_us >= tx.p99_us);

    engine.flush_metrics().await.unwrap();
    let history = engine.metric_history("db.attempt_tx", 10).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].count, tx.count as i64);
}

#[tokio::test]
async fn snapshot_cache_serves_reads_and_bumps_its_version_on_change() {
    let clock = clock_at(2026, 9, 16, 9, 0);
    let engine = booted(clock.clone()).await;
    let a = engine.snapshot().await.unwrap();
    let b = engine.snapshot().await.unwrap();
    assert!(Arc::ptr_eq(&a, &b), "a second read serves the same cached Arc");

    let after = engine
        .log_attempt(LogAttemptInput { slug: Some("two-sum".into()), result: AttemptResult::Solved, ..Default::default() })
        .await
        .unwrap()
        .snapshot;
    assert!(after.version > a.version, "writes refresh the cache");
    assert_eq!(after.last_reward.as_ref().unwrap().xp, 30);
}
