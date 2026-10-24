//! Event engine. Every meaningful thing that happens becomes an `AppEvent`.
//!
//! State changes (XP, coins, tasks, reviews) are applied *inside* the database
//! transaction that produced the event, and the event is written to the `events`
//! table in the same transaction (an outbox). Only after commit are events pushed
//! onto the bounded bus, where async subscribers (notifications, UI push,
//! analytics, scheduler) consume them via broadcast fan-out.
use crate::metrics::{names, Metrics};
use crate::models::{Difficulty, Slot};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    ProblemSolved { attempt_id: i64, problem_id: i64, slug: String, title: String, difficulty: Difficulty, topic: String, is_review: bool },
    ProblemFailed { attempt_id: i64, problem_id: i64, slug: String, title: String, difficulty: Difficulty, topic: String },
    ReviewCompleted { problem_id: i64, success: bool, next_due: i64 },
    DailyGoalCompleted { task_id: i64, slot: Slot, title: String },
    RewardEarned { xp: i64, coins: i64, source: String },
    ItemPurchased { item_id: i64, name: String, cost: i64 },
    StreakUpdated { streak: i64, best: i64 },
    AvatarFed { hunger: i64 },
    LevelUp { level: i64 },
    DailyPlanCreated { date: String, task_count: usize },
    TaskMissed { task_id: i64, slot: Slot, title: String },
    TaskDue { task_id: i64, slot: Slot, title: String, minutes_left: i64 },
    AchievementUnlocked { key: String, title: String },
    DayEnded { date: String, solved: i64, breakfast_done: bool },
}

impl AppEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            AppEvent::ProblemSolved { .. } => "problem_solved",
            AppEvent::ProblemFailed { .. } => "problem_failed",
            AppEvent::ReviewCompleted { .. } => "review_completed",
            AppEvent::DailyGoalCompleted { .. } => "daily_goal_completed",
            AppEvent::RewardEarned { .. } => "reward_earned",
            AppEvent::ItemPurchased { .. } => "item_purchased",
            AppEvent::StreakUpdated { .. } => "streak_updated",
            AppEvent::AvatarFed { .. } => "avatar_fed",
            AppEvent::LevelUp { .. } => "level_up",
            AppEvent::DailyPlanCreated { .. } => "daily_plan_created",
            AppEvent::TaskMissed { .. } => "task_missed",
            AppEvent::TaskDue { .. } => "task_due",
            AppEvent::AchievementUnlocked { .. } => "achievement_unlocked",
            AppEvent::DayEnded { .. } => "day_ended",
        }
    }

    /// Events that should cause the UI to re-fetch the snapshot.
    pub fn changes_state(&self) -> bool {
        !matches!(self, AppEvent::TaskDue { .. })
    }
}

/// Producer half. Bounded: if subscribers fall behind, producers await (backpressure)
/// rather than growing memory without limit.
#[derive(Clone)]
pub struct EventBus {
    tx: mpsc::Sender<AppEvent>,
    fanout: broadcast::Sender<AppEvent>,
}

pub struct EventDispatcher {
    rx: mpsc::Receiver<AppEvent>,
    fanout: broadcast::Sender<AppEvent>,
    processed: Arc<AtomicU64>,
}

impl EventBus {
    pub fn new(capacity: usize) -> (EventBus, EventDispatcher) {
        let (tx, rx) = mpsc::channel(capacity);
        let (fanout, _) = broadcast::channel(capacity.max(16));
        let bus = EventBus { tx, fanout: fanout.clone() };
        let dispatcher = EventDispatcher { rx, fanout, processed: Arc::new(AtomicU64::new(0)) };
        (bus, dispatcher)
    }

    pub async fn publish(&self, event: AppEvent) {
        if self.tx.send(event).await.is_err() {
            tracing::warn!("event bus closed; dropping event");
        }
    }

    pub async fn publish_all(&self, events: impl IntoIterator<Item = AppEvent>) {
        for e in events {
            self.publish(e).await;
        }
    }

    /// Non-blocking publish for sync contexts; returns false if the queue is full.
    pub fn try_publish(&self, event: AppEvent) -> bool {
        self.tx.try_send(event).is_ok()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.fanout.subscribe()
    }

    pub fn capacity(&self) -> usize {
        self.tx.capacity()
    }
}

impl EventDispatcher {
    pub fn processed_counter(&self) -> Arc<AtomicU64> {
        self.processed.clone()
    }

    /// Runs until cancelled. Drains remaining queued events on shutdown so nothing
    /// that was committed goes unannounced.
    pub async fn run(mut self, metrics: Arc<Metrics>, cancel: CancellationToken) {
        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                maybe = self.rx.recv() => match maybe {
                    Some(ev) => self.dispatch(ev, &metrics),
                    None => break,
                },
            }
        }
        while let Ok(ev) = self.rx.try_recv() {
            self.dispatch(ev, &metrics);
        }
        tracing::info!(processed = self.processed.load(Ordering::Relaxed), "event dispatcher stopped");
    }

    fn dispatch(&self, ev: AppEvent, metrics: &Metrics) {
        let _t = metrics.timer(names::EVENT_DISPATCH);
        tracing::debug!(kind = ev.kind(), "event");
        // Err only means there are no subscribers right now; that's fine.
        let _ = self.fanout.send(ev);
        self.processed.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn events_fan_out_to_every_subscriber() {
        let (bus, dispatcher) = EventBus::new(8);
        let metrics = Arc::new(Metrics::new());
        let cancel = CancellationToken::new();
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();
        let handle = tokio::spawn(dispatcher.run(metrics.clone(), cancel.clone()));

        bus.publish(AppEvent::LevelUp { level: 2 }).await;
        bus.publish(AppEvent::AvatarFed { hunger: 90 }).await;

        assert_eq!(a.recv().await.unwrap(), AppEvent::LevelUp { level: 2 });
        assert_eq!(b.recv().await.unwrap(), AppEvent::LevelUp { level: 2 });
        assert_eq!(a.recv().await.unwrap(), AppEvent::AvatarFed { hunger: 90 });
        assert_eq!(b.recv().await.unwrap(), AppEvent::AvatarFed { hunger: 90 });

        cancel.cancel();
        handle.await.unwrap();
        assert_eq!(metrics.get(names::EVENT_DISPATCH).unwrap().count, 2);
    }

    #[test]
    fn events_round_trip_through_json() {
        let e = AppEvent::DailyGoalCompleted { task_id: 3, slot: Slot::Breakfast, title: "Solve 1 Easy".into() };
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains("\"type\":\"daily_goal_completed\""));
        assert_eq!(serde_json::from_str::<AppEvent>(&s).unwrap(), e);
    }
}
