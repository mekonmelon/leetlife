//! Exercise the actual disk-backed save path and backup, rather than only RAM.
use chrono::{Local, TimeZone};
use leetlife::clock::FixedClock;
use leetlife::engine::{Engine, EngineConfig, LogAttemptInput};
use leetlife::models::AttemptResult;
use leetlife::storage::Db;
use std::sync::Arc;

#[tokio::test]
async fn progress_and_decorations_survive_a_real_database_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("new-save-folder").join("leetlife.db");
    let clock = Arc::new(FixedClock::at(Local.with_ymd_and_hms(2026, 10, 23, 8, 30, 0).unwrap()));
    let engine = Engine::open(&path, clock.clone(), EngineConfig::default()).await.unwrap();
    engine.bootstrap().await.unwrap();
    engine.log_attempt(LogAttemptInput { slug: Some("two-sum".into()), result: AttemptResult::Solved, ..Default::default() }).await.unwrap();
    engine.purchase_item(1).await.unwrap();
    engine.set_equipped(1, false).await.unwrap();
    engine.set_setting("leetcode_username", "little-coder").await.unwrap();
    let before = engine.snapshot().await.unwrap();
    engine.db().close().await;
    drop(engine);

    let reopened = Engine::open(&path, clock, EngineConfig::default()).await.unwrap();
    let after = reopened.bootstrap().await.unwrap();
    assert_eq!(after.avatar, before.avatar);
    assert_eq!(after.tasks, before.tasks);
    assert_eq!(after.inventory, before.inventory);
    assert_eq!(reopened.stats().await.unwrap().total_solved, 1);
    assert_eq!(reopened.settings().await.unwrap().get("leetcode_username").unwrap(), "little-coder");
    reopened.db().close().await;
}

#[tokio::test]
async fn backup_contains_committed_wal_data_and_does_not_overwrite_existing_files() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("leetlife.db");
    let backup = directory.path().join("backups").join("world.db");
    let clock = Arc::new(FixedClock::at(Local.with_ymd_and_hms(2026, 10, 23, 8, 30, 0).unwrap()));
    let engine = Engine::open(&path, clock.clone(), EngineConfig::default()).await.unwrap();
    engine.bootstrap().await.unwrap();
    engine.log_attempt(LogAttemptInput { slug: Some("two-sum".into()), result: AttemptResult::Solved, ..Default::default() }).await.unwrap();
    engine.db().backup(&backup).await.unwrap();
    assert!(engine.db().backup(&backup).await.is_err());

    let restored = Engine::from_db(Db::open(&backup).await.unwrap(), clock, EngineConfig::default());
    let snapshot = restored.bootstrap().await.unwrap();
    assert_eq!(snapshot.avatar.xp, 30);
    assert_eq!(snapshot.avatar.coins, 130);
    assert_eq!(restored.stats().await.unwrap().total_solved, 1);
    restored.db().close().await;
    engine.db().close().await;
}

#[tokio::test]
async fn first_solve_after_midnight_creates_the_new_days_quests() {
    let clock = Arc::new(FixedClock::at(Local.with_ymd_and_hms(2026, 10, 23, 23, 59, 0).unwrap()));
    let engine = Engine::in_memory(clock.clone()).await.unwrap();
    engine.bootstrap().await.unwrap();
    clock.advance_secs(120);
    let out = engine.log_attempt(LogAttemptInput { slug: Some("two-sum".into()), result: AttemptResult::Solved, ..Default::default() }).await.unwrap();
    assert_eq!(out.snapshot.date, "2026-10-24");
    assert_eq!(out.snapshot.tasks.len(), 4);
    assert_eq!(out.completed_tasks.len(), 1);
    assert_eq!(out.completed_tasks[0].date, "2026-10-24");
    assert_eq!(out.reward.xp, 30);
}

#[tokio::test]
async fn a_long_absence_catches_up_to_yesterday_in_one_launch() {
    let clock = Arc::new(FixedClock::at(Local.with_ymd_and_hms(2026, 5, 1, 8, 30, 0).unwrap()));
    let engine = Engine::in_memory(clock.clone()).await.unwrap();
    engine.bootstrap().await.unwrap();
    clock.set(Local.with_ymd_and_hms(2026, 10, 23, 8, 30, 0).unwrap());
    let snapshot = engine.bootstrap().await.unwrap();
    assert_eq!(snapshot.date, "2026-10-23");
    assert_eq!(engine.settings().await.unwrap().get("last_day_ended").unwrap(), "2026-10-22");
}
