//! Cross-cutting data types shared by storage, engine, cache and the IPC layer.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub fn as_str(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "easy" => Some(Difficulty::Easy),
            "medium" => Some(Difficulty::Medium),
            "hard" => Some(Difficulty::Hard),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttemptResult {
    Solved,
    Failed,
}

impl Default for AttemptResult {
    fn default() -> Self {
        AttemptResult::Solved
    }
}

impl AttemptResult {
    pub fn as_str(self) -> &'static str {
        match self {
            AttemptResult::Solved => "solved",
            AttemptResult::Failed => "failed",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "solved" => Some(AttemptResult::Solved),
            "failed" => Some(AttemptResult::Failed),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProblemMeta {
    pub id: i64,
    pub slug: String,
    pub title: String,
    #[sqlx(try_from = "String")]
    pub difficulty: DifficultyCol,
    pub topic: String,
    pub importance: f64,
}

/// Newtype so SQLx can decode the TEXT column into `Difficulty`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DifficultyCol(pub Difficulty);

impl TryFrom<String> for DifficultyCol {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Difficulty::parse(&s)
            .map(DifficultyCol)
            .ok_or_else(|| format!("bad difficulty {s}"))
    }
}

impl ProblemMeta {
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct AttemptRow {
    pub id: i64,
    pub problem_id: i64,
    pub difficulty: String,
    pub result: String,
    pub duration_ms: Option<i64>,
    pub attempted_at: i64,
    pub topic: String,
    pub source: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Slot {
    Breakfast,
    Training,
    Work,
    Evening,
}

impl Slot {
    pub const ALL: [Slot; 4] = [Slot::Breakfast, Slot::Training, Slot::Work, Slot::Evening];
    pub fn as_str(self) -> &'static str {
        match self {
            Slot::Breakfast => "breakfast",
            Slot::Training => "training",
            Slot::Work => "work",
            Slot::Evening => "evening",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "breakfast" => Some(Slot::Breakfast),
            "training" => Some(Slot::Training),
            "work" => Some(Slot::Work),
            "evening" => Some(Slot::Evening),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Slot::Breakfast => "Breakfast",
            Slot::Training => "Training",
            Slot::Work => "Work",
            Slot::Evening => "Evening",
        }
    }
    /// Local hour after which the task counts as missed (None = end of day).
    pub fn deadline_hour(self) -> Option<u32> {
        match self {
            Slot::Breakfast => Some(10),
            Slot::Training => Some(14),
            Slot::Work => Some(19),
            Slot::Evening => None,
        }
    }
}

/// What a daily task asks for. Stored as JSON in `daily_tasks.kind`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskKind {
    /// Solve any problem of this difficulty.
    SolveDifficulty { difficulty: Difficulty },
    /// Solve any problem in this topic (weak-topic training).
    SolveTopic { topic: String },
    /// Re-solve a specific problem that is due for spaced-repetition review.
    Review { problem_id: i64, slug: String },
    /// Solve any problem at all.
    SolveAny,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Done,
    Missed,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Done => "done",
            TaskStatus::Missed => "missed",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "done" => TaskStatus::Done,
            "missed" => TaskStatus::Missed,
            _ => TaskStatus::Pending,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DailyTask {
    pub id: i64,
    pub date: String,
    pub slot: Slot,
    pub kind: TaskKind,
    pub title: String,
    pub target: i64,
    pub progress: i64,
    pub status: TaskStatus,
    pub deadline: Option<i64>,
    pub reward_xp: i64,
    pub reward_coins: i64,
    pub completed_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ItemDef {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub cost: i64,
    pub asset_path: String,
    pub description: String,
    pub min_level: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnedItem {
    pub item: ItemDef,
    pub acquired_at: i64,
    pub equipped: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShopItem {
    pub item: ItemDef,
    pub owned: bool,
    pub affordable: bool,
    pub unlocked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RewardSummary {
    pub headline: String,
    pub xp: i64,
    pub coins: i64,
    pub at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DayCount {
    pub date: String,
    pub solved: i64,
    pub failed: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub total_solved: i64,
    pub total_attempts: i64,
    pub solved_by_difficulty: Vec<(String, i64)>,
    pub last_14_days: Vec<DayCount>,
    pub topics: Vec<crate::recommendation::TopicScore>,
    pub due_reviews: Vec<ProblemMeta>,
    pub recent_events: Vec<EventRow>,
    pub achievements: Vec<Achievement>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventRow {
    pub id: i64,
    pub kind: String,
    pub payload: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Achievement {
    pub key: String,
    pub title: String,
    pub unlocked_at: i64,
}
