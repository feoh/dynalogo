//! VM dispatch benchmarks.
//!
//! `REPEAT`/`WHEN`-demon bodies are instruction lists re-executed many times
//! per second; the interesting cost here is repeated compilation of the same
//! list, which `Vm`'s chunk cache (see `vm.rs::compile_list`) is meant to
//! eliminate.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dynalogo_core::vm::Vm;

fn repeat_forward(iterations: u64) {
    let mut vm = Vm::new();
    vm.eval_source(&format!("repeat {iterations} [forward 1 right 1]"))
        .expect("repeat forward should run");
}

fn bench_repeat_body_recompilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("repeat_instruction_list");
    for iterations in [100u64, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(iterations),
            &iterations,
            |b, &iterations| {
                b.iter(|| repeat_forward(iterations));
            },
        );
    }
    group.finish();
}

fn call_user_procedure(calls: u64) {
    let mut vm = Vm::new();
    vm.eval_source("to step\nforward 1 right 1\nend")
        .expect("procedure definition should run");
    vm.eval_source(&format!("repeat {calls} [step]"))
        .expect("calling procedure should run");
}

fn bench_user_procedure_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("user_procedure_call");
    for calls in [100u64, 1_000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(calls), &calls, |b, &calls| {
            b.iter(|| call_user_procedure(calls));
        });
    }
    group.finish();
}

/// Approximates the per-turtle-per-tick cost of a `WHEN`-style demon body:
/// a fixed instruction list run once per simulated turtle for one frame.
fn simulate_demon_ticks(turtle_count: u64) {
    let mut vm = Vm::new();
    vm.eval_source("make \"body [forward 1 right 1]")
        .expect("body binding should run");
    vm.eval_source(&format!("repeat {turtle_count} [run :body]"))
        .expect("running demon body per turtle should succeed");
}

fn bench_demon_tick_at_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("demon_tick_single_frame");
    for turtle_count in [100u64, 1_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(turtle_count),
            &turtle_count,
            |b, &turtle_count| {
                b.iter(|| simulate_demon_ticks(turtle_count));
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_repeat_body_recompilation,
    bench_user_procedure_calls,
    bench_demon_tick_at_scale
);
criterion_main!(benches);
