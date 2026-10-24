//! Game state engine: pure functions over `AvatarState`. Nothing here touches
//! the database or the UI; the engine applies these inside a transaction.
use crate::models::{AttemptResult, DailyTask, Difficulty, ItemDef, Slot, TaskStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct AvatarState {
    pub hunger: i64,
    pub energy: i64,
    pub happiness: i64,
    pub xp: i64,
    pub level: i64,
    pub coins: i64,
    pub streak: i64,
    pub best_streak: i64,
    pub last_active_date: Option<String>,
    pub started_on: String,
    pub updated_at: i64,
}

impl AvatarState {
    pub fn fresh(today: &str, now: i64) -> Self {
        Self {
            hunger: 70,
            energy: 80,
            happiness: 70,
            xp: 0,
            level: 1,
            coins: 100,
            streak: 0,
            best_streak: 0,
            last_active_date: None,
            started_on: today.to_string(),
            updated_at: now,
        }
    }

    /// Applies a reward, clamps meters to 0..=100, and returns the new level if it changed.
    pub fn apply(&mut self, r: &Reward, now: i64) -> Option<i64> {
        self.hunger = clamp(self.hunger + r.hunger);
        self.energy = clamp(self.energy + r.energy);
        self.happiness = clamp(self.happiness + r.happiness);
        self.xp = (self.xp + r.xp).max(0);
        self.coins = (self.coins + r.coins).max(0);
        self.updated_at = now;
        let new_level = level_for_xp(self.xp);
        if new_level != self.level {
            self.level = new_level;
            Some(new_level)
        } else {
            None
        }
    }

    /// "Day N" counter shown in the header (1-based, from the day the app was first opened).
    pub fn day_number(&self, today: &str) -> i64 {
        crate::clock::days_between(&self.started_on, today).unwrap_or(0) + 1
    }

    /// Records activity on `today` and advances / resets the streak.
    /// Returns `Some(streak)` if it changed. Missing a single day is forgiven
    /// (grace day) so a bad Tuesday doesn't erase a month.
    pub fn touch_streak(&mut self, today: &str) -> Option<i64> {
        if self.last_active_date.as_deref() == Some(today) {
            return None;
        }
        let gap = self
            .last_active_date
            .as_deref()
            .and_then(|d| crate::clock::days_between(d, today))
            .unwrap_or(i64::MAX);
        self.streak = if gap <= 2 { self.streak + 1 } else { 1 };
        self.best_streak = self.best_streak.max(self.streak);
        self.last_active_date = Some(today.to_string());
        Some(self.streak)
    }

    pub fn xp_into_level(&self) -> (i64, i64) {
        let base = xp_for_level(self.level);
        let next = xp_for_level(self.level + 1);
        (self.xp - base, next - base)
    }
}

fn clamp(v: i64) -> i64 {
    v.clamp(0, 100)
}

/// Level curve: 50 XP for level 2, 200 for 3, 450 for 4 ... (quadratic, gentle).
pub fn level_for_xp(xp: i64) -> i64 {
    1 + ((xp.max(0) as f64 / 50.0).sqrt().floor() as i64)
}

pub fn xp_for_level(level: i64) -> i64 {
    let l = (level - 1).max(0);
    l * l * 50
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reward {
    pub xp: i64,
    pub coins: i64,
    pub hunger: i64,
    pub energy: i64,
    pub happiness: i64,
}

impl Reward {
    pub fn add(&mut self, o: &Reward) {
        self.xp += o.xp;
        self.coins += o.coins;
        self.hunger += o.hunger;
        self.energy += o.energy;
        self.happiness += o.happiness;
    }
    pub fn scaled(self, f: f64) -> Reward {
        let s = |v: i64| ((v as f64) * f).round() as i64;
        Reward { xp: s(self.xp), coins: s(self.coins), hunger: s(self.hunger), energy: s(self.energy), happiness: s(self.happiness) }
    }
}

/// Reward for a single attempt. Reviews pay 60% (you already knew it), failures
/// still pay a little XP for the effort and never cost coins.
pub fn reward_for_attempt(d: Difficulty, result: AttemptResult, is_review: bool) -> Reward {
    let base = match d {
        Difficulty::Easy => Reward { xp: 10, coins: 15, hunger: 0, energy: 3, happiness: 3 },
        Difficulty::Medium => Reward { xp: 25, coins: 30, hunger: 0, energy: 0, happiness: 5 },
        Difficulty::Hard => Reward { xp: 60, coins: 70, hunger: 0, energy: -5, happiness: 10 },
    };
    match result {
        AttemptResult::Solved if is_review => base.scaled(0.6),
        AttemptResult::Solved => base,
        AttemptResult::Failed => Reward { xp: 3, coins: 0, hunger: 0, energy: -3, happiness: -2 },
    }
}

/// Bonus paid when a daily slot completes. Breakfast is what feeds the avatar.
pub fn slot_reward(slot: Slot) -> Reward {
    match slot {
        Slot::Breakfast => Reward { xp: 20, coins: 15, hunger: 40, energy: 5, happiness: 5 },
        Slot::Training => Reward { xp: 25, coins: 20, hunger: 0, energy: 10, happiness: 5 },
        Slot::Work => Reward { xp: 30, coins: 30, hunger: 0, energy: -5, happiness: 8 },
        Slot::Evening => Reward { xp: 20, coins: 20, hunger: 0, energy: 0, happiness: 12 },
    }
}

/// Overnight tick applied when a new day starts. Deliberately gentle: a bad day
/// makes the avatar grumpy, it doesn't wipe progress.
pub fn overnight(state: &mut AvatarState, breakfast_done: bool, any_solved: bool, now: i64) {
    state.hunger = clamp(state.hunger - if breakfast_done { 15 } else { 30 });
    state.energy = clamp(state.energy.max(60) + 15);
    if !any_solved {
        state.happiness = clamp(state.happiness - 8);
    }
    state.updated_at = now;
}

/// Does an attempt satisfy a task? (used by the engine to advance daily tasks)
pub fn attempt_matches(task: &DailyTask, problem_id: i64, difficulty: Difficulty, topic: &str, result: AttemptResult) -> bool {
    use crate::models::TaskKind::*;
    if task.status != TaskStatus::Pending || result != AttemptResult::Solved {
        return false;
    }
    match &task.kind {
        SolveDifficulty { difficulty: d } => *d == difficulty,
        SolveTopic { topic: t } => t.eq_ignore_ascii_case(topic),
        Review { problem_id: p, .. } => *p == problem_id,
        SolveAny => true,
    }
}

/// The speech-bubble line. Derived entirely from state so the UI can't drift.
pub fn mood_line(state: &AvatarState, tasks: &[DailyTask], hour: u32) -> String {
    let breakfast = tasks.iter().find(|t| t.slot == Slot::Breakfast);
    let done = tasks.iter().filter(|t| t.status == TaskStatus::Done).count();
    let pending = tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();

    if let Some(b) = breakfast {
        if b.status == TaskStatus::Pending && hour < 10 {
            return "I haven't eaten yet... one Easy would really hit the spot.".into();
        }
        if b.status == TaskStatus::Missed {
            return "Skipped breakfast. Running on vibes and leftover recursion.".into();
        }
    }
    if state.hunger < 30 {
        return "So hungry I'm hallucinating off-by-one errors.".into();
    }
    if pending == 0 && !tasks.is_empty() {
        return "Everything done. Time to admire the furniture.".into();
    }
    if state.streak >= 7 && done == 0 {
        return format!("{}-day streak on the line. No pressure.", state.streak);
    }
    if state.energy < 30 {
        return "Low energy. Maybe a Medium will wake me up?".into();
    }
    if hour >= 21 && pending > 0 {
        return "Late night grind? One quick review before bed.".into();
    }
    if done > 0 {
        return "Feeling sharp. What's next?".into();
    }
    match state.happiness {
        h if h >= 80 => "Life is good. Graphs are not scary.".into(),
        h if h >= 50 => "Ready when you are.".into(),
        _ => "Could use a win today.".into(),
    }
}

/// Seed catalogue for the shop. IDs are stable; the UI maps `asset_path` to visuals.
pub fn catalog() -> Vec<ItemDef> {
    let mk = |id, name: &str, category: &str, cost, asset: &str, desc: &str, min_level| ItemDef {
        id,
        name: name.into(),
        category: category.into(),
        cost,
        asset_path: asset.into(),
        description: desc.into(),
        min_level,
    };
    vec![
        mk(1, "Plant", "decor", 60, "furniture/plant", "A small green friend. Photosynthesis, not recursion.", 1),
        mk(2, "Desk", "furniture", 120, "furniture/desk", "Somewhere to put the keyboard.", 1),
        mk(3, "Bed", "furniture", 150, "furniture/bed", "For power naps between Hards.", 1),
        mk(4, "Couch", "furniture", 200, "furniture/couch", "Where Mediums get solved horizontally.", 2),
        mk(5, "Poster", "decor", 80, "furniture/poster", "Big-O cheat sheet, framed.", 1),
        mk(6, "Lamp", "decor", 90, "furniture/lamp", "Warm light for late-night DP.", 1),
        mk(7, "Bookshelf", "furniture", 260, "furniture/bookshelf", "CLRS lives here. Unread, but present.", 3),
        mk(8, "Gaming PC", "tech", 500, "furniture/pc", "RGB. Compiles Rust marginally faster.", 4),
        mk(9, "Cat", "pet", 400, "pets/cat", "Sits on the keyboard during interviews.", 3),
        mk(10, "Dog", "pet", 450, "pets/dog", "Believes every solution is accepted.", 4),
        mk(11, "Rug", "decor", 110, "furniture/rug", "Really ties the room together.", 2),
        mk(12, "Window View", "decor", 300, "furniture/window", "A view of something other than a terminal.", 3),
        mk(13, "Studio Upgrade", "room", 800, "rooms/studio", "Bigger walls, more furniture slots.", 5),
        mk(14, "Loft Upgrade", "room", 2000, "rooms/loft", "Exposed brick. Peak developer aesthetic.", 8),
        mk(15, "Hoodie", "outfit", 140, "outfits/hoodie", "The uniform.", 2),
        mk(16, "Headphones", "outfit", 180, "outfits/headphones", "Lo-fi beats to solve graphs to.", 2),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_curve_is_monotonic_and_matches_inverse() {
        for lvl in 1..30 {
            assert_eq!(level_for_xp(xp_for_level(lvl)), lvl);
            assert_eq!(level_for_xp(xp_for_level(lvl + 1) - 1), lvl);
        }
    }

    #[test]
    fn apply_clamps_and_levels_up() {
        let mut s = AvatarState::fresh("2026-09-18", 0);
        let up = s.apply(&Reward { xp: 60, coins: 10, hunger: 500, energy: -500, happiness: 0 }, 1);
        assert_eq!(up, Some(2));
        assert_eq!(s.hunger, 100);
        assert_eq!(s.energy, 0);
        assert_eq!(s.coins, 110);
    }

    #[test]
    fn streak_has_grace_day_but_resets_after_two() {
        let mut s = AvatarState::fresh("2026-09-01", 0);
        assert_eq!(s.touch_streak("2026-09-01"), Some(1));
        assert_eq!(s.touch_streak("2026-09-01"), None);
        assert_eq!(s.touch_streak("2026-09-02"), Some(2));
        assert_eq!(s.touch_streak("2026-09-04"), Some(3)); // skipped the 3rd: forgiven
        assert_eq!(s.touch_streak("2026-09-09"), Some(1)); // long gap: reset
        assert_eq!(s.best_streak, 3);
    }

    #[test]
    fn failures_never_cost_coins() {
        for d in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            let r = reward_for_attempt(d, AttemptResult::Failed, false);
            assert_eq!(r.coins, 0);
            assert!(r.xp > 0);
        }
    }

    #[test]
    fn review_pays_less_than_first_solve() {
        let full = reward_for_attempt(Difficulty::Medium, AttemptResult::Solved, false);
        let review = reward_for_attempt(Difficulty::Medium, AttemptResult::Solved, true);
        assert!(review.xp < full.xp && review.coins < full.coins);
    }
}
