//! Adaptive recommendation + spaced repetition.
//!
//! `priority = weakness + staleness + failures + importance + review_due - recent_exposure`
//! Every term is normalised to 0..1 and weighted, and each recommendation carries a
//! human-readable reason ("graph confidence 0.41, last reviewed 17 days ago").
use crate::models::{AttemptResult, Difficulty, ProblemMeta};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DAY: i64 = 86_400;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct TopicScore {
    pub topic: String,
    pub confidence: f64,
    pub attempts: i64,
    pub failures: i64,
    pub last_reviewed: Option<i64>,
}

impl TopicScore {
    pub fn new(topic: &str) -> Self {
        Self { topic: topic.to_string(), confidence: 0.5, attempts: 0, failures: 0, last_reviewed: None }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReviewState {
    pub problem_id: i64,
    pub interval_idx: i64,
    pub ease: f64,
    pub due_at: i64,
    pub last_reviewed: i64,
    pub reviews: i64,
    pub lapses: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub problem: ProblemMeta,
    pub score: f64,
    pub reason: String,
    pub is_review: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Weights {
    pub weakness: f64,
    pub staleness: f64,
    pub failures: f64,
    pub importance: f64,
    pub review_due: f64,
    pub exposure: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Self { weakness: 2.0, staleness: 1.0, failures: 0.8, importance: 1.0, review_due: 1.5, exposure: 1.5 }
    }
}

/// Exponential moving average of "did you get it". Harder problems move the
/// needle more, so one Hard success says more than one Easy.
pub fn update_confidence(score: &mut TopicScore, result: AttemptResult, difficulty: Difficulty, now: i64) {
    let alpha = match difficulty {
        Difficulty::Easy => 0.12,
        Difficulty::Medium => 0.22,
        Difficulty::Hard => 0.35,
    };
    let target = match result {
        AttemptResult::Solved => 1.0,
        AttemptResult::Failed => 0.0,
    };
    score.confidence += alpha * (target - score.confidence);
    score.confidence = score.confidence.clamp(0.02, 0.99);
    score.attempts += 1;
    if result == AttemptResult::Failed {
        score.failures += 1;
    }
    score.last_reviewed = Some(now);
}

pub struct ScoreInput<'a> {
    pub problem: &'a ProblemMeta,
    pub topic: Option<&'a TopicScore>,
    pub review: Option<&'a ReviewState>,
    /// Attempts on this exact problem in the last 7 days.
    pub recent_attempts: u32,
    pub now: i64,
}

/// Hot path: the number only. Called for every candidate on every refresh, so
/// it allocates nothing. The human-readable reason is built separately by
/// [`explain`], for the handful of problems that actually get shown.
pub fn score_problem(input: &ScoreInput<'_>, w: &Weights) -> (f64, bool) {
    let confidence = input.topic.map(|t| t.confidence).unwrap_or(0.5);
    let weakness = 1.0 - confidence;

    let days_since_topic = input
        .topic
        .and_then(|t| t.last_reviewed)
        .map(|ts| ((input.now - ts) as f64 / DAY as f64).max(0.0));
    let staleness = days_since_topic.map(|d| (d / 30.0).min(1.0)).unwrap_or(1.0);

    let failure_rate = input
        .topic
        .filter(|t| t.attempts > 0)
        .map(|t| t.failures as f64 / t.attempts as f64)
        .unwrap_or(0.0);

    let (review_due, is_review) = match input.review {
        Some(r) if r.due_at <= input.now => {
            let overdue_days = (input.now - r.due_at) as f64 / DAY as f64;
            ((0.6 + (overdue_days / 7.0).min(0.4)), true)
        }
        // Solved and not yet due: strongly deprioritise, it's "known" right now.
        Some(_) => (-0.8, false),
        None => (0.0, false),
    };

    let exposure = (input.recent_attempts.min(3) as f64) / 3.0;

    let score = w.weakness * weakness
        + w.staleness * staleness
        + w.failures * failure_rate
        + w.importance * input.problem.importance
        + w.review_due * review_due
        - w.exposure * exposure;

    (score, is_review)
}

/// Cold path: why this problem was chosen, in words. Allocates freely because
/// it only runs for the results the caller keeps.
pub fn explain(input: &ScoreInput<'_>) -> String {
    let confidence = input.topic.map(|t| t.confidence).unwrap_or(0.5);
    let days_since_topic = input
        .topic
        .and_then(|t| t.last_reviewed)
        .map(|ts| ((input.now - ts) as f64 / DAY as f64).max(0.0));
    let failure_rate = input
        .topic
        .filter(|t| t.attempts > 0)
        .map(|t| t.failures as f64 / t.attempts as f64)
        .unwrap_or(0.0);
    let is_review = matches!(input.review, Some(r) if r.due_at <= input.now);

    let mut parts: Vec<String> = Vec::new();
    if is_review {
        parts.push("due for review".into());
    }
    if confidence < 0.6 {
        parts.push(format!("{} confidence is {:.0}%", input.problem.topic, confidence * 100.0));
    } else if confidence > 0.85 {
        parts.push(format!("{} is strong ({:.0}%)", input.problem.topic, confidence * 100.0));
    }
    match days_since_topic {
        None => parts.push(format!("you haven't touched {} yet", input.problem.topic)),
        Some(d) if d >= 7.0 => parts.push(format!("last {} practice was {} days ago", input.problem.topic, d.round() as i64)),
        _ => {}
    }
    if failure_rate > 0.3 {
        parts.push(format!("{:.0}% of your {} attempts failed", failure_rate * 100.0, input.problem.topic));
    }
    if input.problem.importance >= 0.8 {
        parts.push("interview classic".into());
    }
    if parts.is_empty() {
        parts.push("good all-round practice".into());
    }
    parts.join(", ")
}

#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub difficulty: Option<Difficulty>,
    pub topic: Option<String>,
    pub only_due: bool,
}

pub fn recommend(
    problems: &[ProblemMeta],
    topics: &[TopicScore],
    reviews: &[ReviewState],
    recent: &HashMap<i64, u32>,
    now: i64,
    filter: &Filter,
    limit: usize,
) -> Vec<Recommendation> {
    if limit == 0 || problems.is_empty() {
        return Vec::new();
    }
    let w = Weights::default();
    let topic_map: HashMap<&str, &TopicScore> = topics.iter().map(|t| (t.topic.as_str(), t)).collect();
    let review_map: HashMap<i64, &ReviewState> = reviews.iter().map(|r| (r.problem_id, r)).collect();

    let input_for = |p: &'_ ProblemMeta| ScoreInput {
        problem: p,
        topic: topic_map.get(p.topic.as_str()).copied(),
        review: review_map.get(&p.id).copied(),
        recent_attempts: recent.get(&p.id).copied().unwrap_or(0),
        now,
    };

    // (score, problem id, index). Small and Copy: scoring the whole catalogue
    // never clones a problem or formats a reason.
    let mut scored: Vec<(f64, i64, u32)> = problems
        .iter()
        .enumerate()
        .filter(|(_, p)| filter.difficulty.map_or(true, |d| p.difficulty() == d))
        .filter(|(_, p)| filter.topic.as_deref().map_or(true, |t| p.topic.eq_ignore_ascii_case(t)))
        .filter_map(|(i, p)| {
            if filter.only_due && !review_map.get(&p.id).map_or(false, |r| r.due_at <= now) {
                return None;
            }
            let (score, _) = score_problem(&input_for(p), &w);
            Some((score, p.id, i as u32))
        })
        .collect();

    // Highest score first; ties go to the lower problem id so results are stable.
    let by_rank = |a: &(f64, i64, u32), b: &(f64, i64, u32)| {
        b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1))
    };

    // Partition in O(n) instead of sorting everything we are about to discard.
    if scored.len() > limit {
        scored.select_nth_unstable_by(limit - 1, by_rank);
        scored.truncate(limit);
    }
    scored.sort_unstable_by(by_rank);

    scored
        .into_iter()
        .map(|(score, _, idx)| {
            let p = &problems[idx as usize];
            let input = input_for(p);
            let is_review = matches!(input.review, Some(r) if r.due_at <= now);
            Recommendation { problem: p.clone(), score, reason: explain(&input), is_review }
        })
        .collect()
}

pub fn weakest_topic(topics: &[TopicScore]) -> Option<&TopicScore> {
    topics.iter().min_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
}

/// Spaced repetition: a small SM-2 variant with a fixed ladder scaled by ease.
pub mod spaced {
    use super::{ReviewState, DAY};

    pub const LADDER_DAYS: [f64; 7] = [1.0, 3.0, 7.0, 14.0, 30.0, 60.0, 120.0];
    pub const MIN_EASE: f64 = 1.3;
    pub const MAX_EASE: f64 = 3.0;
    pub const BASE_EASE: f64 = 2.5;

    pub fn interval_secs(idx: i64, ease: f64) -> i64 {
        let idx = idx.clamp(0, LADDER_DAYS.len() as i64 - 1) as usize;
        (LADDER_DAYS[idx] * (ease / BASE_EASE) * DAY as f64).round() as i64
    }

    /// Computes the next review state after an attempt.
    pub fn next_review(prev: Option<&ReviewState>, problem_id: i64, success: bool, now: i64) -> ReviewState {
        let mut st = prev.cloned().unwrap_or(ReviewState {
            problem_id,
            interval_idx: -1,
            ease: BASE_EASE,
            due_at: now,
            last_reviewed: now,
            reviews: 0,
            lapses: 0,
        });
        if success {
            st.interval_idx = (st.interval_idx + 1).min(LADDER_DAYS.len() as i64 - 1);
            st.ease = (st.ease + 0.1).min(MAX_EASE);
            st.reviews += 1;
            st.due_at = now + interval_secs(st.interval_idx, st.ease);
        } else {
            st.interval_idx = (st.interval_idx - 2).max(0);
            st.ease = (st.ease - 0.2).max(MIN_EASE);
            st.lapses += 1;
            st.due_at = now + DAY;
        }
        st.last_reviewed = now;
        st
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DifficultyCol;

    fn p(id: i64, topic: &str, d: Difficulty, importance: f64) -> ProblemMeta {
        ProblemMeta { id, slug: format!("p{id}"), title: format!("P{id}"), difficulty: DifficultyCol(d), topic: topic.into(), importance }
    }

    #[test]
    fn weak_stale_topic_outranks_strong_fresh_topic() {
        let now = 1_700_000_000;
        let problems = vec![p(1, "Graphs", Difficulty::Medium, 0.7), p(2, "Arrays", Difficulty::Medium, 0.7)];
        let topics = vec![
            TopicScore { topic: "Graphs".into(), confidence: 0.41, attempts: 5, failures: 2, last_reviewed: Some(now - 17 * DAY) },
            TopicScore { topic: "Arrays".into(), confidence: 0.91, attempts: 20, failures: 1, last_reviewed: Some(now - DAY) },
        ];
        let recs = recommend(&problems, &topics, &[], &HashMap::new(), now, &Filter::default(), 5);
        assert_eq!(recs[0].problem.id, 1);
        assert!(recs[0].reason.contains("Graphs confidence is 41%"), "{}", recs[0].reason);
        assert!(recs[0].reason.contains("17 days ago"), "{}", recs[0].reason);
    }

    #[test]
    fn recently_solved_problem_is_deprioritised_until_due() {
        let now = 1_700_000_000;
        let problems = vec![p(1, "Trees", Difficulty::Easy, 0.5), p(2, "Trees", Difficulty::Easy, 0.5)];
        let reviews = vec![ReviewState { problem_id: 1, interval_idx: 2, ease: 2.5, due_at: now + 5 * DAY, last_reviewed: now - DAY, reviews: 3, lapses: 0 }];
        let recs = recommend(&problems, &[], &reviews, &HashMap::new(), now, &Filter::default(), 5);
        assert_eq!(recs[0].problem.id, 2);
        let due = vec![ReviewState { due_at: now - DAY, ..reviews[0].clone() }];
        let recs = recommend(&problems, &[], &due, &HashMap::new(), now, &Filter::default(), 5);
        assert_eq!(recs[0].problem.id, 1);
        assert!(recs[0].is_review);
    }

    #[test]
    fn filter_only_due_and_difficulty() {
        let now = 1_700_000_000;
        let problems = vec![p(1, "Trees", Difficulty::Easy, 0.5), p(2, "Trees", Difficulty::Hard, 0.5)];
        let reviews = vec![ReviewState { problem_id: 2, interval_idx: 0, ease: 2.5, due_at: now - 1, last_reviewed: now - DAY, reviews: 1, lapses: 0 }];
        let recs = recommend(&problems, &[], &reviews, &HashMap::new(), now, &Filter { only_due: true, ..Default::default() }, 5);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].problem.id, 2);
        let recs = recommend(&problems, &[], &reviews, &HashMap::new(), now, &Filter { difficulty: Some(Difficulty::Easy), ..Default::default() }, 5);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].problem.id, 1);
    }

    #[test]
    fn spaced_ladder_grows_on_success_and_shrinks_on_failure() {
        let now = 1_700_000_000;
        let s1 = spaced::next_review(None, 7, true, now);
        assert_eq!(s1.interval_idx, 0);
        assert!((s1.due_at - now - DAY).abs() < 2 * 3600 * 24 / 10); // ~1 day (ease 2.6/2.5)
        let s2 = spaced::next_review(Some(&s1), 7, true, s1.due_at);
        let s3 = spaced::next_review(Some(&s2), 7, true, s2.due_at);
        assert_eq!(s3.interval_idx, 2);
        assert!(s3.due_at - s2.due_at > s2.due_at - s1.due_at);
        let f = spaced::next_review(Some(&s3), 7, false, s3.due_at);
        assert_eq!(f.interval_idx, 0);
        assert_eq!(f.lapses, 1);
        assert_eq!(f.due_at - s3.due_at, DAY);
        assert!(f.ease < s3.ease);
    }

    #[test]
    fn confidence_ema_moves_toward_result() {
        let mut t = TopicScore::new("DP");
        update_confidence(&mut t, AttemptResult::Solved, Difficulty::Hard, 10);
        assert!(t.confidence > 0.6);
        update_confidence(&mut t, AttemptResult::Failed, Difficulty::Easy, 20);
        update_confidence(&mut t, AttemptResult::Failed, Difficulty::Easy, 30);
        assert!(t.confidence < 0.6);
        assert_eq!(t.attempts, 3);
        assert_eq!(t.failures, 2);
    }
}
