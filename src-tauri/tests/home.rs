//! Home edits must survive real restarts and must never spend or lose progress.
use chrono::{Local, TimeZone};
use leetlife::clock::FixedClock;
use leetlife::engine::{Engine, EngineConfig, LogAttemptInput};
use leetlife::home::Placement;
use leetlife::models::AttemptResult;
use std::sync::Arc;

fn clock() -> Arc<FixedClock> {
    Arc::new(FixedClock::at(Local.with_ymd_and_hms(2026, 10, 23, 8, 30, 0).unwrap()))
}

#[tokio::test]
async fn each_room_keeps_its_wallpaper_and_item_positions_after_a_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world.db");
    let clock = clock();
    let engine = Engine::open(&path, clock.clone(), EngineConfig::default()).await.unwrap();
    engine.bootstrap().await.unwrap();
    engine.purchase_item(17).await.unwrap(); // fern wallpaper, 55
    engine.purchase_item(22).await.unwrap(); // mushroom lamp, 45
    engine.set_wallpaper("hearth", "wallpapers/fern").await.unwrap();
    engine.set_wallpaper("studio", "wallpapers/linen").await.unwrap();
    engine.place_item(22, Placement { room: "loft".into(), x: 83.5, y: 180.0 }).await.unwrap();
    engine.set_equipped(22, false).await.unwrap();
    let before = engine.place_item(22, Placement { room: "loft".into(), x: 91.0, y: 185.0 }).await.unwrap();
    assert_eq!(before.avatar.coins, 0);
    assert!(before.inventory.iter().find(|o| o.item.id == 22).unwrap().equipped);
    engine.db().close().await;
    drop(engine);

    let reopened = Engine::open(&path, clock, EngineConfig::default()).await.unwrap();
    let after = reopened.bootstrap().await.unwrap();
    assert_eq!(after.home, before.home);
    assert_eq!(after.avatar, before.avatar);
    assert_eq!(after.home.wallpapers["hearth"], "wallpapers/fern");
    assert_eq!(after.home.wallpapers["studio"], "wallpapers/linen");
    assert!(!after.home.wallpapers.contains_key("loft"));
    assert_eq!(after.home.placements[&22].room, "loft");
    assert_eq!(after.home.placements[&22].x, 91.0);
    reopened.db().close().await;
}

#[tokio::test]
async fn decoration_rejects_unowned_treasures_unknown_rooms_and_invalid_coordinates() {
    let engine = Engine::in_memory(clock()).await.unwrap();
    engine.bootstrap().await.unwrap();
    assert!(engine.place_item(20, Placement {room:"hearth".into(),x:100.0,y:180.0}).await.is_err());
    assert!(engine.set_wallpaper("hearth", "wallpapers/starlight").await.is_err());
    assert!(engine.set_wallpaper("outside", "wallpapers/linen").await.is_err());
    engine.purchase_item(22).await.unwrap();
    assert!(engine.place_item(22, Placement {room:"hearth".into(),x:f64::NAN,y:180.0}).await.is_err());
    assert!(engine.place_item(22, Placement {room:"hearth".into(),x:500.0,y:180.0}).await.is_err());
    let after = engine.snapshot().await.unwrap();
    assert!(after.home.placements.is_empty());
    assert!(after.home.wallpapers.is_empty());
    assert_eq!(after.avatar.coins, 55);
}

#[tokio::test]
async fn concurrent_room_edits_preserve_both_placements() {
    let engine = Engine::in_memory(clock()).await.unwrap();
    engine.bootstrap().await.unwrap();
    engine.log_attempt(LogAttemptInput {slug:Some("two-sum".into()),result:AttemptResult::Solved,..Default::default()}).await.unwrap();
    engine.purchase_item(1).await.unwrap();
    engine.purchase_item(22).await.unwrap();
    let a = engine.clone();
    let b = engine.clone();
    let (a,b) = tokio::join!(
        a.place_item(1, Placement {room:"studio".into(),x:90.0,y:180.0}),
        b.place_item(22, Placement {room:"loft".into(),x:250.0,y:175.0}),
    );
    a.unwrap();b.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    assert_eq!(snapshot.home.placements.len(), 2);
    assert_eq!(snapshot.home.placements[&1].room, "studio");
    assert_eq!(snapshot.home.placements[&22].room, "loft");
}
