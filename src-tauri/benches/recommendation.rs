//! Scoring is on the path of every snapshot refresh, so it is benchmarked
//! against catalogues far larger than the shipped one.
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use leetlife::models::{Difficulty, DifficultyCol, ProblemMeta};
use leetlife::recommendation::{recommend, spaced, Filter, ReviewState, TopicScore, DAY};
use std::collections::HashMap;

const TOPICS: [&str; 8] = ["Arrays", "Graphs", "Trees", "Dynamic Programming", "Strings", "Heap", "Binary Search", "Stack"];
const NOW: i64 = 1_800_000_000;

fn corpus(n: usize) -> (Vec<ProblemMeta>, Vec<TopicScore>, Vec<ReviewState>, HashMap<i64, u32>) {
    let problems: Vec<ProblemMeta> = (0..n as i64)
        .map(|i| ProblemMeta {
            id: i + 1,
            slug: format!("problem-{i}"),
            title: format!("Problem {i}"),
            difficulty: DifficultyCol(match i % 3 {
                0 => Difficulty::Easy,
                1 => Difficulty::Medium,
                _ => Difficulty::Hard,
            }),
            topic: TOPICS[i as usize % TOPICS.len()].to_string(),
            importance: ((i % 10) as f64) / 10.0,
        })
        .collect();
    let topics: Vec<TopicScore> = TOPICS
        .iter()
        .enumerate()
        .map(|(i, t)| TopicScore { topic: t.to_string(), confidence: 0.2 + (i as f64) * 0.09, attempts: 20, failures: i as i64, last_reviewed: Some(NOW - (i as i64 + 1) * DAY) })
        .collect();
    // A third of the catalogue has review state.
    let reviews: Vec<ReviewState> = problems
        .iter()
        .step_by(3)
        .map(|p| ReviewState { problem_id: p.id, interval_idx: (p.id % 5), ease: 2.5, due_at: NOW + (p.id % 11 - 5) * DAY, last_reviewed: NOW - 3 * DAY, reviews: 3, lapses: 0 })
        .collect();
    let recent: HashMap<i64, u32> = problems.iter().step_by(7).map(|p| (p.id, (p.id % 4) as u32)).collect();
    (problems, topics, reviews, recent)
}

fn bench_recommend(c: &mut Criterion) {
    let mut group = c.benchmark_group("recommend");
    for n in [100usize, 1_000, 5_000] {
        let (problems, topics, reviews, recent) = corpus(n);
        group.bench_with_input(BenchmarkId::new("top3", n), &n, |b, _| {
            b.iter(|| recommend(black_box(&problems), &topics, &reviews, &recent, NOW, &Filter::default(), 3))
        });
        group.bench_with_input(BenchmarkId::new("only_due", n), &n, |b, _| {
            b.iter(|| recommend(black_box(&problems), &topics, &reviews, &recent, NOW, &Filter { only_due: true, ..Default::default() }, 10))
        });
    }
    group.finish();
}

fn bench_spaced(c: &mut Criterion) {
    c.bench_function("spaced/ladder_10_reviews", |b| {
        b.iter(|| {
            let mut st = None;
            let mut now = NOW;
            for i in 0..10 {
                let next = spaced::next_review(st.as_ref(), 1, i % 4 != 3, now);
                now = next.due_at;
                st = Some(next);
            }
            black_box(st)
        })
    });
}

criterion_group!(benches, bench_recommend, bench_spaced);
criterion_main!(benches);
