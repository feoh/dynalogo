//! End-to-end 1,000-turtle @ 60Hz benchmark.
//!
//! Builds a swarm of dynaturtles entirely through Logo-level primitives
//! (TELL/EACH/SETXY/SETVELOCITY/WHEN), then measures the cost of a single
//! `dynaturtle_tick` -- movement integration, spatial-hash collision
//! detection, and demon dispatch -- against the 16.67ms/60Hz frame budget.

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use dynalogo_core::collision::{Bounds, CollisionConfig};
use dynalogo_core::vm::Vm;

// 600x600 keeps spatial-hash cell occupancy (cell_size 32, radius 8) at a
// realistic ~1 turtle/cell average for 1,000 turtles; a smaller world packs
// cells pathologically densely and turns candidate-pair generation
// quadratic, which is a benchmark-setup artifact, not a real bottleneck.
const WORLD_SIZE: i64 = 600;
const TICK_DT: f64 = 1.0 / 60.0;
const TOUCHING_PAIRS: usize = 10;

/// Spawns `turtle_count` turtles at random positions inside a `WORLD_SIZE`
/// square with random velocities, registers an EDGE demon that fires for any
/// turtle bouncing off the world bounds, then forces `TOUCHING_PAIRS` turtles
/// to overlap and registers a TOUCHING demon per pair so both demon dispatch
/// paths fire every tick.
fn build_swarm(turtle_count: usize) -> Vm {
    let mut vm = Vm::new();
    vm.set_collision_config(CollisionConfig {
        cell_size: 32.0,
        turtle_radius: 8.0,
        bounds: Some(Bounds::new(0.0, 0.0, WORLD_SIZE as f64, WORLD_SIZE as f64)),
    });

    let ids: Vec<String> = (0..turtle_count).map(|i| i.to_string()).collect();
    vm.eval_source(&format!("tell [{}]", ids.join(" ")))
        .expect("tell should spawn the swarm");
    vm.eval_source(&format!(
        "each [setxy random {WORLD_SIZE} random {WORLD_SIZE} setvelocity random 40 random 40]"
    ))
    .expect("each should place and launch every turtle");

    vm.eval_source("make \"hits 0")
        .expect("hits counter should initialize");
    vm.eval_source("when [edge] [make \"hits sum :hits 1]")
        .expect("edge demon should register");

    for pair in 0..TOUCHING_PAIRS.min(turtle_count / 2) {
        let a = pair * 2;
        let b = a + 1;
        vm.eval_source(&format!("tell {a} setxy 100 100"))
            .expect("forcing pair a into contact should run");
        vm.eval_source(&format!("tell {b} setxy 100 100"))
            .expect("forcing pair b into contact should run");
        vm.eval_source(&format!(
            "when [touching {a} {b}] [make \"hits sum :hits 1]"
        ))
        .expect("touching demon should register");
    }

    vm
}

fn bench_dynaturtle_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynaturtle_tick_end_to_end");
    for turtle_count in [100usize, 1_000] {
        let swarm = build_swarm(turtle_count);
        group.bench_with_input(
            BenchmarkId::from_parameter(turtle_count),
            &turtle_count,
            |b, _| {
                b.iter_batched(
                    || swarm.clone(),
                    |mut vm| {
                        vm.dynaturtle_tick(TICK_DT)
                            .expect("tick should run cleanly");
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_dynaturtle_tick);
criterion_main!(benches);
