//! Fixed-timestep simulation utilities for dynaturtles.
//!
//! Rendering and simulation should not be coupled. The simulation advances in
//! fixed quanta (60 Hz by default), while renderers interpolate between the two
//! latest snapshots. v0.2's multi-turtle store and collision pass will build on
//! these foundations.

use std::time::Duration;

use crate::turtle::{Point, TurtleState};

pub const DEFAULT_TICK_HZ: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimConfig {
    pub tick: Duration,
    pub max_steps_per_frame: usize,
}

impl SimConfig {
    pub fn at_hz(hz: u32) -> Self {
        assert!(hz > 0, "simulation frequency must be positive");
        Self {
            tick: Duration::from_secs_f64(1.0 / hz as f64),
            max_steps_per_frame: 8,
        }
    }
}

impl Default for SimConfig {
    fn default() -> Self {
        Self::at_hz(DEFAULT_TICK_HZ)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FixedTimestep {
    config: SimConfig,
    accumulator: Duration,
    tick_index: u64,
}

impl FixedTimestep {
    pub fn new(config: SimConfig) -> Self {
        Self {
            config,
            accumulator: Duration::ZERO,
            tick_index: 0,
        }
    }

    pub fn tick_index(&self) -> u64 {
        self.tick_index
    }

    pub fn config(&self) -> SimConfig {
        self.config
    }

    pub fn interpolation_alpha(&self) -> f64 {
        self.accumulator.as_secs_f64() / self.config.tick.as_secs_f64()
    }

    pub fn advance(&mut self, elapsed: Duration, mut step: impl FnMut(u64)) -> usize {
        self.accumulator += elapsed;
        let mut steps = 0;
        while self.accumulator >= self.config.tick && steps < self.config.max_steps_per_frame {
            step(self.tick_index);
            self.tick_index += 1;
            self.accumulator -= self.config.tick;
            steps += 1;
        }

        // Avoid a spiral of death after a pause or breakpoint.
        if steps == self.config.max_steps_per_frame && self.accumulator >= self.config.tick {
            self.accumulator = Duration::ZERO;
        }
        steps
    }
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self::new(SimConfig::default())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TurtleSnapshot {
    pub tick: u64,
    pub turtles: Vec<TurtleState>,
}

impl TurtleSnapshot {
    pub fn single(tick: u64, turtle: TurtleState) -> Self {
        Self {
            tick,
            turtles: vec![turtle],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SnapshotBuffer {
    previous: Option<TurtleSnapshot>,
    current: Option<TurtleSnapshot>,
}

impl SnapshotBuffer {
    pub fn push(&mut self, snapshot: TurtleSnapshot) {
        self.previous = self.current.take();
        self.current = Some(snapshot);
    }

    pub fn previous(&self) -> Option<&TurtleSnapshot> {
        self.previous.as_ref()
    }

    pub fn current(&self) -> Option<&TurtleSnapshot> {
        self.current.as_ref()
    }

    pub fn interpolated(&self, alpha: f64) -> Option<Vec<TurtleState>> {
        let current = self.current.as_ref()?;
        let Some(previous) = self.previous.as_ref() else {
            return Some(current.turtles.clone());
        };
        let alpha = alpha.clamp(0.0, 1.0);
        Some(
            previous
                .turtles
                .iter()
                .zip(&current.turtles)
                .map(|(a, b)| interpolate_turtle(*a, *b, alpha))
                .collect(),
        )
    }
}

pub fn interpolate_turtle(a: TurtleState, b: TurtleState, alpha: f64) -> TurtleState {
    TurtleState {
        position: interpolate_point(a.position, b.position, alpha),
        heading: interpolate_angle_degrees(a.heading, b.heading, alpha),
        pen_down: b.pen_down,
        pen_color: b.pen_color,
        pen_size: b.pen_size,
        label_height: b.label_height,
        visible: b.visible,
    }
}

fn interpolate_point(a: Point, b: Point, alpha: f64) -> Point {
    Point::new(lerp(a.x, b.x, alpha), lerp(a.y, b.y, alpha))
}

fn interpolate_angle_degrees(a: f64, b: f64, alpha: f64) -> f64 {
    let delta = ((b - a + 540.0) % 360.0) - 180.0;
    (a + delta * alpha).rem_euclid(360.0)
}

fn lerp(a: f64, b: f64, alpha: f64) -> f64 {
    a + (b - a) * alpha
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_at(x: f64, y: f64, heading: f64) -> TurtleState {
        TurtleState {
            position: Point::new(x, y),
            heading,
            ..TurtleState::default()
        }
    }

    #[test]
    fn fixed_timestep_runs_whole_ticks_and_keeps_remainder() {
        let mut timestep = FixedTimestep::default();
        let mut ticks = Vec::new();
        let steps = timestep.advance(Duration::from_millis(51), |tick| ticks.push(tick));
        assert_eq!(steps, 3);
        assert_eq!(ticks, vec![0, 1, 2]);
        assert_eq!(timestep.tick_index(), 3);
        assert!(timestep.interpolation_alpha() > 0.0);
        assert!(timestep.interpolation_alpha() < 1.0);
    }

    #[test]
    fn timestep_caps_steps_per_frame() {
        let mut timestep = FixedTimestep::new(SimConfig {
            tick: Duration::from_millis(10),
            max_steps_per_frame: 2,
        });
        let steps = timestep.advance(Duration::from_millis(100), |_| {});
        assert_eq!(steps, 2);
        assert_eq!(timestep.tick_index(), 2);
        assert_eq!(timestep.interpolation_alpha(), 0.0);
    }

    #[test]
    fn snapshot_buffer_interpolates_between_two_snapshots() {
        let mut buffer = SnapshotBuffer::default();
        buffer.push(TurtleSnapshot::single(0, state_at(0.0, 0.0, 0.0)));
        buffer.push(TurtleSnapshot::single(1, state_at(10.0, 20.0, 90.0)));
        let states = buffer.interpolated(0.5).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].position, Point::new(5.0, 10.0));
        assert_eq!(states[0].heading, 45.0);
    }

    #[test]
    fn angle_interpolation_uses_shortest_path() {
        let a = state_at(0.0, 0.0, 350.0);
        let b = state_at(0.0, 0.0, 10.0);
        assert_eq!(interpolate_turtle(a, b, 0.5).heading, 0.0);
    }
}
