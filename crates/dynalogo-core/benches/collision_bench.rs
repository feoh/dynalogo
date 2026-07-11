//! Collision detection benchmarks at dynaturtle-relevant population sizes.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dynalogo_core::collision::{detect_collisions, CollisionConfig};
use dynalogo_core::dynaturtle::TurtleStore;
use dynalogo_core::turtle::Point;

fn store_with_turtles(count: usize) -> TurtleStore {
    let mut store = TurtleStore::new();
    for i in 0..count {
        let id = dynalogo_core::dynaturtle::TurtleId::new(i);
        if i > 0 {
            store.spawn_default();
        }
        let angle = (i as f64) * 0.61803398875;
        let radius = (i as f64).sqrt() * 8.0;
        store.set_position(id, Point::new(radius * angle.cos(), radius * angle.sin()));
    }
    store
}

fn bench_detect_collisions(c: &mut Criterion) {
    let mut group = c.benchmark_group("detect_collisions");
    for turtle_count in [100usize, 1_000] {
        let store = store_with_turtles(turtle_count);
        let config = CollisionConfig::default();
        group.bench_with_input(
            BenchmarkId::from_parameter(turtle_count),
            &turtle_count,
            |b, _| {
                b.iter(|| detect_collisions(&store, config));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_detect_collisions);
criterion_main!(benches);
