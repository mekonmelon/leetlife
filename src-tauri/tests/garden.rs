use chrono::{Local, TimeZone};
use leetlife::{clock::FixedClock, engine::{Engine, EngineConfig}, game::xp_for_level};
use std::sync::Arc;

fn clock() -> Arc<FixedClock> {
    Arc::new(FixedClock::at(Local.with_ymd_and_hms(2026, 10, 24, 10, 0, 0).unwrap()))
}
async fn unlock(engine: &Engine) {
    let mut connection = engine.db().conn().await.unwrap();
    sqlx::query("UPDATE avatar_state SET level=3,xp=?,coins=100 WHERE id=1")
        .bind(xp_for_level(3)).execute(&mut *connection).await.unwrap();
}

#[tokio::test]
async fn garden_unlock_and_validation_are_enforced_before_spending_coins() {
    let engine = Engine::in_memory(clock()).await.unwrap();
    engine.bootstrap().await.unwrap();
    assert!(engine.garden_action("plant",0,Some("daisy")).await.is_err());
    unlock(&engine).await;
    assert!(engine.garden_action("plant",6,Some("daisy")).await.is_err());
    assert!(engine.garden_action("plant",0,Some("unknown")).await.is_err());
    assert!(engine.garden_action("water",0,None).await.is_err());
    assert!(engine.garden_action("harvest",0,None).await.is_err());
    let after = engine.refresh_snapshot().await.unwrap();
    assert_eq!(after.avatar.coins,100);
    assert!(after.home.garden.is_empty());
}

#[tokio::test]
async fn flowers_survive_a_restart_and_grow_offline_without_duplicate_rewards() {
    let dir=tempfile::tempdir().unwrap();let path=dir.path().join("garden.db");let clock=clock();
    let engine=Engine::open(&path,clock.clone(),EngineConfig::default()).await.unwrap();
    engine.bootstrap().await.unwrap();unlock(&engine).await;
    let planted=engine.garden_action("plant",2,Some("moonflower")).await.unwrap();
    assert_eq!(planted.avatar.coins,92);
    assert!(engine.garden_action("plant",2,Some("poppy")).await.is_err());
    assert!(engine.garden_action("harvest",2,None).await.is_err());
    engine.garden_action("water",2,None).await.unwrap();
    assert!(engine.garden_action("water",2,None).await.is_err());
    engine.db().close().await;drop(engine);
    clock.advance_secs(1801);
    let reopened=Engine::open(&path,clock,EngineConfig::default()).await.unwrap();
    let saved=reopened.bootstrap().await.unwrap();
    assert_eq!(saved.home.garden[&2].species,"moonflower");
    assert!(saved.home.garden[&2].watered_at.is_some());
    let picked=reopened.garden_action("harvest",2,None).await.unwrap();
    assert_eq!(picked.avatar.coins,104);assert!(picked.home.garden.is_empty());
    assert!(reopened.garden_action("harvest",2,None).await.is_err());
    assert_eq!(reopened.snapshot().await.unwrap().avatar.coins,104);
    reopened.db().close().await;
}

#[tokio::test]
async fn competing_plant_actions_charge_once_and_keep_other_home_edits() {
    let engine=Engine::in_memory(clock()).await.unwrap();engine.bootstrap().await.unwrap();unlock(&engine).await;
    engine.purchase_item(22).await.unwrap();
    engine.place_item(22,leetlife::home::Placement{room:"loft".into(),x:80.,y:180.}).await.unwrap();
    let a=engine.clone();let b=engine.clone();
    let (a,b)=tokio::join!(a.garden_action("plant",0,Some("daisy")),b.garden_action("plant",0,Some("poppy")));
    assert_ne!(a.is_ok(),b.is_ok());
    let after=engine.refresh_snapshot().await.unwrap();
    assert_eq!(after.avatar.coins,47);assert_eq!(after.home.garden.len(),1);
    assert_eq!(after.home.placements[&22].room,"loft");
    engine.set_wallpaper("loft","wallpapers/linen").await.unwrap();
    assert_eq!(engine.snapshot().await.unwrap().home.garden.len(),1);
}
