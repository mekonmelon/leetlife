//! Notification service: a `Notifier` sink plus the pure mapping from events to
//! user-facing copy. The Tauri implementation lives in `app::notifier`.
use crate::events::AppEvent;
use crate::metrics::{names, Metrics};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
}

impl Notification {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self { title: title.into(), body: body.into() }
    }
}

pub trait Notifier: Send + Sync {
    fn notify(&self, n: Notification);
}

/// Logs instead of showing anything (headless / tests).
pub struct LogNotifier;
impl Notifier for LogNotifier {
    fn notify(&self, n: Notification) {
        tracing::info!(title = %n.title, body = %n.body, "notification");
    }
}

/// Collects notifications so tests can assert on them.
#[derive(Default)]
pub struct MemoryNotifier {
    pub sent: Mutex<Vec<Notification>>,
}
impl Notifier for MemoryNotifier {
    fn notify(&self, n: Notification) {
        self.sent.lock().push(n);
    }
}

/// Which events deserve interrupting the user, and what we say.
pub fn notification_for(event: &AppEvent) -> Option<Notification> {
    Some(match event {
        AppEvent::TaskDue { slot, title, minutes_left, .. } => Notification::new(
            match slot {
                crate::models::Slot::Breakfast => "Your LeetLife character hasn't had breakfast",
                _ => "LeetLife: task closing soon",
            },
            format!("{title} — {minutes_left} minutes left."),
        ),
        AppEvent::TaskMissed { slot, title, .. } => Notification::new(
            format!("{} missed", slot.label()),
            match slot {
                crate::models::Slot::Breakfast => format!("No breakfast today. \"{title}\" is still worth coins later."),
                _ => format!("\"{title}\" slipped by. The avatar is dramatically sighing."),
            },
        ),
        AppEvent::DailyGoalCompleted { slot, .. } => return Some(slot_notification(*slot)),
        AppEvent::LevelUp { level } => Notification::new("Level up!", format!("Your character reached level {level}.")),
        AppEvent::StreakUpdated { streak, .. } if *streak > 0 && streak % 7 == 0 => {
            Notification::new("Streak milestone", format!("{streak} days in a row. The house is proud of you."))
        }
        AppEvent::AchievementUnlocked { title, .. } => Notification::new("Achievement unlocked", title.clone()),
        AppEvent::DailyPlanCreated { task_count, .. } => {
            Notification::new("Good morning", format!("{task_count} things on the plan today. Breakfast first: solve one Easy before 10 AM."))
        }
        _ => return None,
    })
}

fn slot_notification(slot: crate::models::Slot) -> Notification {
    use crate::game::slot_reward;
    use crate::models::Slot;
    let r = slot_reward(slot);
    let head = match slot {
        Slot::Breakfast => "Breakfast acquired.",
        Slot::Training => "Training complete.",
        Slot::Work => "Work shipped.",
        Slot::Evening => "Evening review done.",
    };
    Notification::new("LeetLife", format!("{head} +{} XP, +{} coins.", r.xp, r.coins))
}

/// Subscriber task: turns bus events into native notifications.
pub async fn run_notification_worker(mut rx: broadcast::Receiver<AppEvent>, notifier: Arc<dyn Notifier>, metrics: Arc<Metrics>, cancel: CancellationToken) {
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            r = rx.recv() => match r {
                Ok(ev) => {
                    if let Some(n) = notification_for(&ev) {
                        let _t = metrics.timer(names::NOTIFY);
                        notifier.notify(n);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => tracing::warn!(missed = n, "notification worker lagged"),
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Slot;

    #[test]
    fn breakfast_completion_copy_matches_reward() {
        let n = notification_for(&AppEvent::DailyGoalCompleted { task_id: 1, slot: Slot::Breakfast, title: "x".into() }).unwrap();
        assert_eq!(n.body, "Breakfast acquired. +20 XP, +15 coins.");
    }

    #[test]
    fn quiet_events_do_not_notify() {
        assert!(notification_for(&AppEvent::RewardEarned { xp: 1, coins: 1, source: "x".into() }).is_none());
        assert!(notification_for(&AppEvent::StreakUpdated { streak: 3, best: 3 }).is_none());
        assert!(notification_for(&AppEvent::StreakUpdated { streak: 14, best: 14 }).is_some());
    }
}
