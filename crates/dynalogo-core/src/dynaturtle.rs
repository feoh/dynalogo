//! Multi-turtle storage for v0.2 dynaturtles.
//!
//! The store is deliberately struct-of-arrays: per-tick updates and collision
//! passes walk positions, headings, velocities, and flags as dense vectors.
//! Language-level `TELL`, `ASK`, `EACH`, and `WHO` map to the active selection.

use crate::turtle::{point_from_heading, Point, TurtleEvent, TurtleState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeMode {
    Bounce,
    Wrap,
    Fence,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl WorldBounds {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TurtleId(usize);

impl TurtleId {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TurtleStore {
    positions: Vec<Point>,
    velocities: Vec<Point>,
    headings: Vec<f64>,
    pen_down: Vec<bool>,
    pen_color: Vec<u32>,
    pen_size: Vec<f64>,
    visible: Vec<bool>,
    shape: Vec<String>,
    collision_radius: Vec<f64>,
    active: Vec<TurtleId>,
    events: Vec<TurtleEvent>,
}

impl TurtleStore {
    pub fn new() -> Self {
        let mut store = Self {
            positions: Vec::new(),
            velocities: Vec::new(),
            headings: Vec::new(),
            pen_down: Vec::new(),
            pen_color: Vec::new(),
            pen_size: Vec::new(),
            visible: Vec::new(),
            shape: Vec::new(),
            collision_radius: Vec::new(),
            active: Vec::new(),
            events: Vec::new(),
        };
        let turtle = store.spawn_default();
        store.active = vec![turtle];
        store
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn spawn_default(&mut self) -> TurtleId {
        self.spawn(TurtleState::default())
    }

    pub fn spawn(&mut self, state: TurtleState) -> TurtleId {
        let id = TurtleId(self.positions.len());
        self.positions.push(state.position);
        self.velocities.push(Point::new(0.0, 0.0));
        self.headings.push(state.heading);
        self.pen_down.push(state.pen_down);
        self.pen_color.push(state.pen_color);
        self.pen_size.push(state.pen_size);
        self.visible.push(state.visible);
        self.shape.push("turtle".to_string());
        self.collision_radius.push(8.0);
        id
    }

    pub fn ensure(&mut self, id: TurtleId) {
        while self.len() <= id.index() {
            self.spawn_default();
        }
    }

    pub fn state(&self, id: TurtleId) -> Option<TurtleState> {
        let i = id.index();
        Some(TurtleState {
            position: *self.positions.get(i)?,
            heading: *self.headings.get(i)?,
            pen_down: *self.pen_down.get(i)?,
            pen_color: *self.pen_color.get(i)?,
            pen_size: *self.pen_size.get(i)?,
            visible: *self.visible.get(i)?,
        })
    }

    pub fn set_state(&mut self, id: TurtleId, state: TurtleState) {
        self.ensure(id);
        let i = id.index();
        self.positions[i] = state.position;
        self.headings[i] = state.heading;
        self.pen_down[i] = state.pen_down;
        self.pen_color[i] = state.pen_color;
        self.pen_size[i] = state.pen_size;
        self.visible[i] = state.visible;
    }

    pub fn set_shape(&mut self, id: TurtleId, shape: impl Into<String>, collision_radius: f64) {
        self.ensure(id);
        let i = id.index();
        self.shape[i] = shape.into();
        self.collision_radius[i] = collision_radius;
    }

    pub fn shape(&self, id: TurtleId) -> Option<&str> {
        self.shape.get(id.index()).map(String::as_str)
    }

    pub fn collision_radius(&self, id: TurtleId) -> Option<f64> {
        self.collision_radius.get(id.index()).copied()
    }

    pub fn active(&self) -> &[TurtleId] {
        &self.active
    }

    pub fn who(&self) -> Option<TurtleId> {
        self.active.first().copied()
    }

    pub fn tell_one(&mut self, id: TurtleId) {
        self.ensure(id);
        self.active = vec![id];
    }

    pub fn tell_many(&mut self, ids: impl IntoIterator<Item = TurtleId>) {
        let ids: Vec<TurtleId> = ids.into_iter().collect();
        for id in &ids {
            self.ensure(*id);
        }
        self.active = ids;
    }

    pub fn ask<R>(&mut self, id: TurtleId, f: impl FnOnce(&mut Self) -> R) -> R {
        let previous = self.active.clone();
        self.tell_one(id);
        let result = f(self);
        self.active = previous;
        result
    }

    pub fn each(&mut self, mut f: impl FnMut(&mut Self, TurtleId)) {
        let ids = self.active.clone();
        for id in ids {
            self.tell_one(id);
            f(self, id);
        }
    }

    pub fn set_position(&mut self, id: TurtleId, position: Point) {
        self.ensure(id);
        self.positions[id.index()] = position;
    }

    pub fn set_heading(&mut self, id: TurtleId, heading: f64) {
        self.ensure(id);
        self.headings[id.index()] = heading.rem_euclid(360.0);
    }

    /// Moves `id` to `to`, recording a draw line event if its pen is down.
    pub fn goto(&mut self, id: TurtleId, to: Point) {
        self.ensure(id);
        let i = id.index();
        if self.pen_down[i] {
            self.events.push(TurtleEvent::Line {
                from: self.positions[i],
                to,
                color: self.pen_color[i],
                width: self.pen_size[i],
            });
        }
        self.positions[i] = to;
    }

    pub fn forward(&mut self, id: TurtleId, distance: f64) {
        self.ensure(id);
        let i = id.index();
        let to = point_from_heading(self.positions[i], self.headings[i], distance);
        self.goto(id, to);
    }

    pub fn back(&mut self, id: TurtleId, distance: f64) {
        self.forward(id, -distance);
    }

    pub fn left(&mut self, id: TurtleId, degrees: f64) {
        self.ensure(id);
        self.set_heading(id, self.headings[id.index()] - degrees);
    }

    pub fn right(&mut self, id: TurtleId, degrees: f64) {
        self.ensure(id);
        self.set_heading(id, self.headings[id.index()] + degrees);
    }

    pub fn set_xy(&mut self, id: TurtleId, x: f64, y: f64) {
        self.goto(id, Point::new(x, y));
    }

    pub fn home(&mut self, id: TurtleId) {
        self.goto(id, Point::new(0.0, 0.0));
        self.set_heading(id, 0.0);
    }

    /// Erases the canvas and sends `ids` home, matching classic CLEARSCREEN
    /// broadcast to the active turtle selection.
    pub fn clearscreen(&mut self, ids: &[TurtleId]) {
        self.events.push(TurtleEvent::Clear);
        for &id in ids {
            self.home(id);
        }
    }

    pub fn set_pen_down(&mut self, id: TurtleId, down: bool) {
        self.ensure(id);
        self.pen_down[id.index()] = down;
    }

    pub fn set_pen_color(&mut self, id: TurtleId, color: u32) {
        self.ensure(id);
        self.pen_color[id.index()] = color;
    }

    pub fn set_pen_size(&mut self, id: TurtleId, size: f64) {
        self.ensure(id);
        self.pen_size[id.index()] = size;
    }

    pub fn set_visible(&mut self, id: TurtleId, visible: bool) {
        self.ensure(id);
        self.visible[id.index()] = visible;
    }

    /// Records a zero-length draw event at `target` without moving `id`.
    pub fn draw_dot(&mut self, id: TurtleId, target: Point) {
        self.ensure(id);
        let i = id.index();
        self.events.push(TurtleEvent::Line {
            from: target,
            to: target,
            color: self.pen_color[i],
            width: self.pen_size[i],
        });
    }

    pub fn events(&self) -> &[TurtleEvent] {
        &self.events
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn velocity(&self, id: TurtleId) -> Option<Point> {
        self.velocities.get(id.index()).copied()
    }

    pub fn speed(&self, id: TurtleId) -> Option<f64> {
        self.velocity(id)
            .map(|velocity| velocity.x.hypot(velocity.y))
    }

    pub fn set_velocity(&mut self, id: TurtleId, velocity: Point) {
        self.ensure(id);
        self.velocities[id.index()] = velocity;
    }

    pub fn set_speed(&mut self, id: TurtleId, speed: f64) {
        self.ensure(id);
        let heading = self.headings[id.index()].to_radians();
        self.velocities[id.index()] = Point::new(heading.sin() * speed, heading.cos() * speed);
    }

    pub fn integrate(&mut self, dt_seconds: f64) {
        for (position, velocity) in self.positions.iter_mut().zip(&self.velocities) {
            position.x += velocity.x * dt_seconds;
            position.y += velocity.y * dt_seconds;
        }
    }

    pub fn snapshots(&self) -> Vec<TurtleState> {
        (0..self.len())
            .map(|index| self.state(TurtleId(index)).expect("index in range"))
            .collect()
    }

    pub fn positions(&self) -> &[Point] {
        &self.positions
    }

    pub fn headings(&self) -> &[f64] {
        &self.headings
    }

    pub fn velocities(&self) -> &[Point] {
        &self.velocities
    }

    pub fn collision_radii(&self) -> &[f64] {
        &self.collision_radius
    }

    pub fn apply_edge_mode(&mut self, bounds: WorldBounds, mode: EdgeMode) {
        if mode == EdgeMode::Window {
            return;
        }

        for index in 0..self.len() {
            let radius = self.collision_radius[index];
            let min_x = bounds.min_x + radius;
            let max_x = bounds.max_x - radius;
            let min_y = bounds.min_y + radius;
            let max_y = bounds.max_y - radius;

            match mode {
                EdgeMode::Bounce => self.apply_bounce(index, min_x, max_x, min_y, max_y),
                EdgeMode::Wrap => self.apply_wrap(index, bounds, radius),
                EdgeMode::Fence => self.apply_fence(index, min_x, max_x, min_y, max_y),
                EdgeMode::Window => unreachable!(),
            }
        }
    }

    fn apply_bounce(&mut self, index: usize, min_x: f64, max_x: f64, min_y: f64, max_y: f64) {
        if self.positions[index].x < min_x || self.positions[index].x > max_x {
            self.velocities[index].x = -self.velocities[index].x;
            self.positions[index].x = self.positions[index].x.clamp(min_x, max_x);
        }
        if self.positions[index].y < min_y || self.positions[index].y > max_y {
            self.velocities[index].y = -self.velocities[index].y;
            self.positions[index].y = self.positions[index].y.clamp(min_y, max_y);
        }
    }

    fn apply_wrap(&mut self, index: usize, bounds: WorldBounds, radius: f64) {
        if self.positions[index].x + radius < bounds.min_x {
            self.positions[index].x = bounds.max_x + radius;
        } else if self.positions[index].x - radius > bounds.max_x {
            self.positions[index].x = bounds.min_x - radius;
        }
        if self.positions[index].y + radius < bounds.min_y {
            self.positions[index].y = bounds.max_y + radius;
        } else if self.positions[index].y - radius > bounds.max_y {
            self.positions[index].y = bounds.min_y - radius;
        }
    }

    fn apply_fence(&mut self, index: usize, min_x: f64, max_x: f64, min_y: f64, max_y: f64) {
        let old = self.positions[index];
        self.positions[index].x = self.positions[index].x.clamp(min_x, max_x);
        self.positions[index].y = self.positions[index].y.clamp(min_y, max_y);
        if self.positions[index].x != old.x {
            self.velocities[index].x = 0.0;
        }
        if self.positions[index].y != old.y {
            self.velocities[index].y = 0.0;
        }
    }
}

impl Default for TurtleStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_starts_with_turtle_zero_active() {
        let store = TurtleStore::new();
        assert_eq!(store.len(), 1);
        assert_eq!(store.who(), Some(TurtleId::new(0)));
        assert_eq!(store.active(), &[TurtleId::new(0)]);
    }

    #[test]
    fn tell_spawns_missing_turtles_and_sets_active_selection() {
        let mut store = TurtleStore::new();
        store.tell_many([TurtleId::new(2), TurtleId::new(4)]);
        assert_eq!(store.len(), 5);
        assert_eq!(store.active(), &[TurtleId::new(2), TurtleId::new(4)]);
    }

    #[test]
    fn ask_temporarily_changes_active_turtle() {
        let mut store = TurtleStore::new();
        store.tell_many([TurtleId::new(0), TurtleId::new(1)]);
        store.ask(TurtleId::new(3), |store| {
            assert_eq!(store.active(), &[TurtleId::new(3)]);
            store.set_position(TurtleId::new(3), Point::new(10.0, 20.0));
        });
        assert_eq!(store.active(), &[TurtleId::new(0), TurtleId::new(1)]);
        assert_eq!(
            store.state(TurtleId::new(3)).unwrap().position,
            Point::new(10.0, 20.0)
        );
    }

    #[test]
    fn each_iterates_over_original_active_selection() {
        let mut store = TurtleStore::new();
        store.tell_many([TurtleId::new(1), TurtleId::new(2), TurtleId::new(3)]);
        store.each(|store, id| store.set_heading(id, id.index() as f64 * 10.0));
        assert_eq!(store.headings()[1], 10.0);
        assert_eq!(store.headings()[2], 20.0);
        assert_eq!(store.headings()[3], 30.0);
    }

    #[test]
    fn snapshots_return_dense_turtle_states() {
        let mut store = TurtleStore::new();
        store.tell_one(TurtleId::new(2));
        store.set_position(TurtleId::new(2), Point::new(5.0, 6.0));
        let snapshots = store.snapshots();
        assert_eq!(snapshots.len(), 3);
        assert_eq!(snapshots[2].position, Point::new(5.0, 6.0));
    }

    #[test]
    fn set_velocity_and_integrate_moves_turtles_continuously() {
        let mut store = TurtleStore::new();
        store.set_velocity(TurtleId::new(0), Point::new(10.0, -5.0));
        store.integrate(0.5);
        assert_eq!(
            store.state(TurtleId::new(0)).unwrap().position,
            Point::new(5.0, -2.5)
        );
    }

    #[test]
    fn set_speed_projects_along_logo_heading() {
        let mut store = TurtleStore::new();
        store.set_heading(TurtleId::new(0), 90.0);
        store.set_speed(TurtleId::new(0), 20.0);
        let velocity = store.velocity(TurtleId::new(0)).unwrap();
        assert!((velocity.x - 20.0).abs() < 1e-9);
        assert!(velocity.y.abs() < 1e-9);
        assert!((store.speed(TurtleId::new(0)).unwrap() - 20.0).abs() < 1e-9);
    }

    #[test]
    fn shape_metadata_tracks_collision_radius() {
        let mut store = TurtleStore::new();
        store.set_shape(TurtleId::new(0), "ship", 12.5);
        assert_eq!(store.shape(TurtleId::new(0)), Some("ship"));
        assert_eq!(store.collision_radius(TurtleId::new(0)), Some(12.5));
        assert_eq!(store.collision_radii(), &[12.5]);
    }

    #[test]
    fn bounce_mode_clamps_position_and_flips_velocity() {
        let mut store = TurtleStore::new();
        store.set_position(TurtleId::new(0), Point::new(105.0, 0.0));
        store.set_velocity(TurtleId::new(0), Point::new(4.0, 1.0));
        store.apply_edge_mode(
            WorldBounds::new(-100.0, -100.0, 100.0, 100.0),
            EdgeMode::Bounce,
        );
        assert_eq!(store.state(TurtleId::new(0)).unwrap().position.x, 92.0);
        assert_eq!(
            store.velocity(TurtleId::new(0)).unwrap(),
            Point::new(-4.0, 1.0)
        );
    }

    #[test]
    fn fence_mode_clamps_position_and_stops_blocked_axis() {
        let mut store = TurtleStore::new();
        store.set_position(TurtleId::new(0), Point::new(0.0, -120.0));
        store.set_velocity(TurtleId::new(0), Point::new(3.0, -7.0));
        store.apply_edge_mode(
            WorldBounds::new(-100.0, -100.0, 100.0, 100.0),
            EdgeMode::Fence,
        );
        assert_eq!(store.state(TurtleId::new(0)).unwrap().position.y, -92.0);
        assert_eq!(
            store.velocity(TurtleId::new(0)).unwrap(),
            Point::new(3.0, 0.0)
        );
    }

    fn approx_eq(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "expected {a} ≈ {b}");
    }

    #[test]
    fn forward_draws_a_line_when_pen_is_down() {
        let mut store = TurtleStore::new();
        store.forward(TurtleId::new(0), 100.0);

        let TurtleEvent::Line { from, to, .. } = store.events()[0] else {
            panic!("expected line event");
        };
        assert_eq!(from, Point::new(0.0, 0.0));
        approx_eq(to.x, 0.0);
        approx_eq(to.y, 100.0);
    }

    #[test]
    fn right_turn_uses_logo_heading_convention() {
        let mut store = TurtleStore::new();
        let id = TurtleId::new(0);
        store.right(id, 90.0);
        store.forward(id, 50.0);
        let TurtleEvent::Line { to, .. } = store.events()[0] else {
            panic!("expected line event");
        };
        approx_eq(to.x, 50.0);
        approx_eq(to.y, 0.0);
    }

    #[test]
    fn pen_up_moves_without_drawing() {
        let mut store = TurtleStore::new();
        let id = TurtleId::new(0);
        store.set_pen_down(id, false);
        store.forward(id, 10.0);
        assert!(store
            .events()
            .iter()
            .all(|event| !matches!(event, TurtleEvent::Line { .. })));
    }

    #[test]
    fn clearscreen_resets_position_and_heading_and_erases_lines() {
        let mut store = TurtleStore::new();
        let id = TurtleId::new(0);
        store.right(id, 90.0);
        store.forward(id, 10.0);
        store.clearscreen(&[id]);
        assert_eq!(store.state(id).unwrap().position, Point::new(0.0, 0.0));
        assert_eq!(store.state(id).unwrap().heading, 0.0);
        assert!(store
            .events()
            .iter()
            .any(|event| matches!(event, TurtleEvent::Clear)));
    }

    #[test]
    fn clearscreen_broadcasts_home_to_every_active_turtle() {
        let mut store = TurtleStore::new();
        store.tell_many([TurtleId::new(1), TurtleId::new(2)]);
        store.set_position(TurtleId::new(1), Point::new(5.0, 5.0));
        store.set_position(TurtleId::new(2), Point::new(-5.0, -5.0));
        let ids = store.active().to_vec();
        store.clearscreen(&ids);
        assert_eq!(
            store.state(TurtleId::new(1)).unwrap().position,
            Point::new(0.0, 0.0)
        );
        assert_eq!(
            store.state(TurtleId::new(2)).unwrap().position,
            Point::new(0.0, 0.0)
        );
    }

    #[test]
    fn wrap_mode_wraps_fully_past_edge() {
        let mut store = TurtleStore::new();
        store.set_position(TurtleId::new(0), Point::new(110.0, 0.0));
        store.apply_edge_mode(
            WorldBounds::new(-100.0, -100.0, 100.0, 100.0),
            EdgeMode::Wrap,
        );
        assert_eq!(store.state(TurtleId::new(0)).unwrap().position.x, -108.0);
    }
}
