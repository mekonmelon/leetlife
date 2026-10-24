//! Event bus throughput and the cost of the snapshot cache's read path.
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use leetlife::cache::{Snapshot, StateCache};
use leetlife::events::{AppEvent, EventBus};
use leetlife::game::AvatarState;
use leetlife::metrics::Metrics;
use leetlife::models::{Difficulty, Slot};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn sample_event(i: i64) -> AppEvent {
    match i % 4 {
        0 => AppEvent::ProblemSolved {
            attempt_id: i,
            problem_id: i,
            slug: "two-sum".into(),
            title: "Two Sum".into(),
            difficulty: Difficulty::Easy,
            topic: "Arrays".into(),
            is_review: false,
        },
        1 => AppEvent::RewardEarned { xp: 10, coins: 15, source: "manual".into() },
        2 => AppEvent::DailyGoalCompleted { task_id: i, slot: Slot::Breakfast, title: "Solve 1 Easy".into() },
        _ => AppEvent::StreakUpdated { streak: i, best: i },
    }
}

fn empty_snapshot() -> Snapshot {
    Snapshot {
        version: 0,
        generated_at: 0,
        date: "2026-09-16".into(),
        weekday: "Wednesday".into(),
        day_number: 1,
        avatar: AvatarState::fresh("2026-09-16", 0),
        mood: "Ready when you are.".into(),
        xp_into_level: 0,
        xp_for_next_level: 50,
        tasks: Vec::new(),
        inventory: Vec::new(),
        topics: Vec::new(),
        recommendations: Vec::new(),
        room_level: 0,
        due_reviews: 0,
        last_reward: None,
    }
}

fn bench_bus(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let mut group = c.benchmark_group("event_bus");

    for subscribers in [1usize, 4] {
        group.bench_function(format!("publish_1000_to_{subscribers}_subscribers"), |b| {
            b.to_async(&rt).iter(|| async move {
                let (bus, dispatcher) = EventBus::new(1024);
                let cancel = CancellationToken::new();
                let mut rxs: Vec<_> = (0..subscribers).map(|_| bus.subscribe()).collect();
                let handle = tokio::spawn(dispatcher.run(Arc::new(Metrics::new()), cancel.clone()));
                let drains: Vec<_> = rxs
                    .drain(..)
                    .map(|mut rx| {
                        tokio::spawn(async move {
                            let mut n = 0;
                            while n < 1000 && rx.recv().await.is_ok() {
                                n += 1;
                            }
                            n
                        })
                    })
                    .collect();
                for i in 0..1000 {
                    bus.publish(sample_event(i)).await;
                }
                for d in drains {
                    black_box(d.await.unwrap());
                }
                cancel.cancel();
                handle.await.unwrap();
            })
        });
    }
    group.finish();
}

fn bench_cache(c: &mut Criterion) {
    let cache = StateCache::new();
    cache.replace(empty_snapshot());
    c.bench_function("cache/get", |b| b.iter(|| black_box(cache.get())));
    c.bench_function("cache/replace", |b| b.iter(|| black_box(cache.replace(empty_snapshot()))));
    let m = Metrics::new();
    c.bench_function("metrics/record", |b| b.iter(|| m.record_us("bench", black_box(42.0))));
}

criterion_group!(benches, bench_bus, bench_cache);
criterion_main!(benches);
