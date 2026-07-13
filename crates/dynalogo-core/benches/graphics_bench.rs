//! Turtle trail renderer benchmarks.
//!
//! The native window should not allocate a fresh full-screen software canvas and
//! replay every visible turtle event on every frame. These benchmarks keep that
//! regression visible by comparing full replay with the incremental RasterCache
//! path used by `dynalogo-window`.

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use dynalogo_core::graphics::{RasterCache, SoftwareCanvas};
use dynalogo_core::turtle::{PenMode, Point, TurtleEvent};

const WIDTH: usize = 1024;
const HEIGHT: usize = 612;

fn trail_events(count: usize) -> Vec<TurtleEvent> {
    (0..count)
        .map(|index| {
            let y = (index % 300) as f64 - 150.0;
            let x0 = -480.0 + (index % 97) as f64;
            let x1 = x0 + 80.0 + (index % 31) as f64;
            TurtleEvent::Line {
                from: Point::new(x0, y),
                to: Point::new(x1, y + 12.0),
                color: 0x40_80_ff,
                width: 1.0,
                mode: PenMode::Down,
            }
        })
        .collect()
}

fn bench_trail_replay(c: &mut Criterion) {
    let mut group = c.benchmark_group("trail_rendering");
    for event_count in [100usize, 1_000] {
        let events = trail_events(event_count);
        group.bench_with_input(
            BenchmarkId::new("full_replay", event_count),
            &events,
            |b, events| {
                b.iter(|| {
                    let mut canvas = SoftwareCanvas::new(WIDTH, HEIGHT);
                    canvas.rasterize_events(events);
                    let mut bytes = Vec::new();
                    canvas.write_rgba_bytes(&mut bytes);
                    bytes
                });
            },
        );

        let base_events = trail_events(event_count.saturating_sub(1));
        let Some(next_event) = events.last().cloned() else {
            continue;
        };
        let mut base_cache = RasterCache::new(WIDTH, HEIGHT);
        base_cache.update(&base_events, 1.0, 1.0);
        group.bench_with_input(
            BenchmarkId::new("incremental_one_event", event_count),
            &event_count,
            |b, _| {
                b.iter_batched(
                    || {
                        let mut events = base_events.clone();
                        events.push(next_event.clone());
                        (base_cache.clone(), events)
                    },
                    |(mut cache, events)| {
                        let mut bytes = Vec::new();
                        if cache.update(&events, 1.0, 1.0) {
                            cache.write_rgba_bytes(&mut bytes);
                        }
                        bytes
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_trail_replay);
criterion_main!(benches);
