//! In-memory snapshot of everything the menu-bar window renders. A menu click
//! reads this (an `Arc` clone) instead of touching SQLite.
use crate::game::AvatarState;
use crate::models::{DailyTask, OwnedItem, RewardSummary};
use crate::recommendation::{Recommendation, TopicScore};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u64,
    pub generated_at: i64,
    pub date: String,
    pub weekday: String,
    pub day_number: i64,
    pub avatar: AvatarState,
    pub mood: String,
    pub xp_into_level: i64,
    pub xp_for_next_level: i64,
    pub tasks: Vec<DailyTask>,
    pub inventory: Vec<OwnedItem>,
    pub topics: Vec<TopicScore>,
    pub recommendations: Vec<Recommendation>,
    pub room_level: i64,
    pub due_reviews: i64,
    pub last_reward: Option<RewardSummary>,
    #[serde(default)]
    pub home: crate::home::HomeLayout,
}

pub struct StateCache {
    inner: RwLock<Option<Arc<Snapshot>>>,
    version: AtomicU64,
}

impl Default for StateCache {
    fn default() -> Self {
        Self::new()
    }
}

impl StateCache {
    pub fn new() -> Self {
        Self { inner: RwLock::new(None), version: AtomicU64::new(0) }
    }

    pub fn get(&self) -> Option<Arc<Snapshot>> {
        self.inner.read().clone()
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Replaces the snapshot, stamping it with a monotonically increasing version.
    pub fn replace(&self, mut snapshot: Snapshot) -> Arc<Snapshot> {
        let v = self.version.fetch_add(1, Ordering::AcqRel) + 1;
        snapshot.version = v;
        let arc = Arc::new(snapshot);
        *self.inner.write() = Some(arc.clone());
        arc
    }

    pub fn invalidate(&self) {
        *self.inner.write() = None;
    }
}
