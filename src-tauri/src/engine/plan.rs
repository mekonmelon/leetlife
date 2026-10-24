//! Pure daily-plan builder. Given what we know about the user (topic scores,
//! due reviews, weekday) it decides the four slots. No I/O, fully testable.
use crate::game::slot_reward;
use crate::models::{Difficulty, ProblemMeta, Slot, TaskKind};
use crate::recommendation::{weakest_topic, TopicScore};
use chrono::{Datelike, NaiveDate, Weekday};

#[derive(Clone, Debug, PartialEq)]
pub struct TaskSpec {
    pub slot: Slot,
    pub kind: TaskKind,
    pub title: String,
    pub target: i64,
    pub reward_xp: i64,
    pub reward_coins: i64,
}

pub struct PlanInput<'a> {
    pub date: NaiveDate,
    pub topics: &'a [TopicScore],
    pub due_reviews: &'a [ProblemMeta],
    /// Topic of the current top recommendation; used when no topic has history yet.
    pub suggested_topic: Option<&'a str>,
}

pub fn plan_day(input: &PlanInput<'_>) -> Vec<TaskSpec> {
    let mut out = Vec::with_capacity(4);
    let weekend = matches!(input.date.weekday(), Weekday::Sat | Weekday::Sun);

    let spec = |slot: Slot, kind: TaskKind, title: String| {
        let r = slot_reward(slot);
        TaskSpec { slot, kind, title, target: 1, reward_xp: r.xp, reward_coins: r.coins }
    };

    out.push(spec(Slot::Breakfast, TaskKind::SolveDifficulty { difficulty: Difficulty::Easy }, "Solve 1 Easy".into()));

    // Training: the weakest topic that has some history, else whatever the
    // recommender likes, else anything.
    // Weakest topic *among those with history* — a topic you have never tried
    // has a default confidence, which says nothing about your weaknesses.
    let practised: Vec<TopicScore> = input.topics.iter().filter(|t| t.attempts > 0).cloned().collect();
    let weak = weakest_topic(&practised).map(|t| t.topic.clone()).or_else(|| input.suggested_topic.map(str::to_string));
    match weak {
        Some(topic) => out.push(spec(Slot::Training, TaskKind::SolveTopic { topic: topic.clone() }, format!("Train {topic}"))),
        None => out.push(spec(Slot::Training, TaskKind::SolveAny, "Solve any problem".into())),
    }

    if weekend {
        let mut s = spec(Slot::Work, TaskKind::SolveDifficulty { difficulty: Difficulty::Hard }, "Weekend Hard".into());
        s.reward_xp += 20;
        s.reward_coins += 20;
        out.push(s);
    } else {
        out.push(spec(Slot::Work, TaskKind::SolveDifficulty { difficulty: Difficulty::Medium }, "Solve 1 Medium".into()));
    }

    match input.due_reviews.first() {
        Some(pm) => out.push(spec(Slot::Evening, TaskKind::Review { problem_id: pm.id, slug: pm.slug.clone() }, format!("Review {}", pm.title))),
        None => out.push(spec(Slot::Evening, TaskKind::SolveAny, "Evening: solve anything".into())),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DifficultyCol;

    fn topic(name: &str, conf: f64, attempts: i64) -> TopicScore {
        TopicScore { topic: name.into(), confidence: conf, attempts, failures: 0, last_reviewed: None }
    }

    #[test]
    fn weekday_plan_has_four_slots_in_order() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 16).unwrap(); // Wednesday
        let plan = plan_day(&PlanInput { date, topics: &[], due_reviews: &[], suggested_topic: Some("Graphs") });
        assert_eq!(plan.iter().map(|t| t.slot).collect::<Vec<_>>(), Slot::ALL.to_vec());
        assert_eq!(plan[1].kind, TaskKind::SolveTopic { topic: "Graphs".into() });
        assert_eq!(plan[2].kind, TaskKind::SolveDifficulty { difficulty: Difficulty::Medium });
        assert_eq!(plan[3].kind, TaskKind::SolveAny);
    }

    #[test]
    fn weakest_practised_topic_wins_and_weekend_is_hard() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(); // Saturday
        let topics = vec![topic("Arrays", 0.9, 10), topic("Graphs", 0.3, 2), topic("Untouched", 0.1, 0)];
        let due = vec![ProblemMeta { id: 200, slug: "number-of-islands".into(), title: "Number of Islands".into(), difficulty: DifficultyCol(Difficulty::Medium), topic: "Graphs".into(), importance: 1.0 }];
        let plan = plan_day(&PlanInput { date, topics: &topics, due_reviews: &due, suggested_topic: None });
        assert_eq!(plan[1].kind, TaskKind::SolveTopic { topic: "Graphs".into() });
        assert_eq!(plan[2].kind, TaskKind::SolveDifficulty { difficulty: Difficulty::Hard });
        assert!(plan[2].reward_xp > slot_reward(Slot::Work).xp);
        assert_eq!(plan[3].kind, TaskKind::Review { problem_id: 200, slug: "number-of-islands".into() });
    }
}
