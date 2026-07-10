//! Graphics-free turtle engine.
//!
//! The interpreter core owns turtle semantics, but not a windowing or drawing
//! library. Frontends implement `TurtleBackend`; tests use the headless backend
//! to assert draw operations.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TurtleEvent {
    Clear,
    Line {
        from: Point,
        to: Point,
        color: u32,
        width: f64,
    },
    State(TurtleState),
}

pub trait TurtleBackend {
    fn clear(&mut self);
    fn draw_line(&mut self, from: Point, to: Point, color: u32, width: f64);
    fn update_turtle(&mut self, state: TurtleState);
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessTurtleBackend {
    events: Vec<TurtleEvent>,
}

impl HeadlessTurtleBackend {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn events(&self) -> &[TurtleEvent] {
        &self.events
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

impl Default for HeadlessTurtleBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TurtleBackend for HeadlessTurtleBackend {
    fn clear(&mut self) {
        self.events.push(TurtleEvent::Clear);
    }

    fn draw_line(&mut self, from: Point, to: Point, color: u32, width: f64) {
        self.events.push(TurtleEvent::Line {
            from,
            to,
            color,
            width,
        });
    }

    fn update_turtle(&mut self, state: TurtleState) {
        self.events.push(TurtleEvent::State(state));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurtleState {
    pub position: Point,
    /// Degrees, where 0 points north/up, matching Logo convention.
    pub heading: f64,
    pub pen_down: bool,
    pub pen_color: u32,
    pub pen_size: f64,
    pub visible: bool,
}

impl Default for TurtleState {
    fn default() -> Self {
        Self {
            position: Point::new(0.0, 0.0),
            heading: 0.0,
            pen_down: true,
            pen_color: 0x00ff_ffff,
            pen_size: 1.0,
            visible: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TurtleWorld<B> {
    state: TurtleState,
    backend: B,
}

impl<B: TurtleBackend> TurtleWorld<B> {
    pub fn new(mut backend: B) -> Self {
        let state = TurtleState::default();
        backend.update_turtle(state);
        Self { state, backend }
    }

    pub fn state(&self) -> TurtleState {
        self.state
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn into_backend(self) -> B {
        self.backend
    }

    pub fn forward(&mut self, distance: f64) {
        let from = self.state.position;
        let to = point_from_heading(from, self.state.heading, distance);
        self.move_to(to);
    }

    pub fn back(&mut self, distance: f64) {
        self.forward(-distance);
    }

    pub fn left(&mut self, degrees: f64) {
        self.set_heading(self.state.heading - degrees);
    }

    pub fn right(&mut self, degrees: f64) {
        self.set_heading(self.state.heading + degrees);
    }

    pub fn set_heading(&mut self, heading: f64) {
        self.state.heading = normalize_degrees(heading);
        self.backend.update_turtle(self.state);
    }

    pub fn set_xy(&mut self, x: f64, y: f64) {
        self.move_to(Point::new(x, y));
    }

    pub fn set_pos(&mut self, point: Point) {
        self.move_to(point);
    }

    pub fn home(&mut self) {
        self.set_pos(Point::new(0.0, 0.0));
        self.set_heading(0.0);
    }

    pub fn clearscreen(&mut self) {
        self.backend.clear();
        self.state.position = Point::new(0.0, 0.0);
        self.state.heading = 0.0;
        self.backend.update_turtle(self.state);
    }

    pub fn pen_up(&mut self) {
        self.state.pen_down = false;
        self.backend.update_turtle(self.state);
    }

    pub fn pen_down(&mut self) {
        self.state.pen_down = true;
        self.backend.update_turtle(self.state);
    }

    pub fn set_pen_color(&mut self, color: u32) {
        self.state.pen_color = color;
        self.backend.update_turtle(self.state);
    }

    pub fn set_pen_size(&mut self, width: f64) {
        self.state.pen_size = width;
        self.backend.update_turtle(self.state);
    }

    pub fn hide_turtle(&mut self) {
        self.state.visible = false;
        self.backend.update_turtle(self.state);
    }

    pub fn show_turtle(&mut self) {
        self.state.visible = true;
        self.backend.update_turtle(self.state);
    }

    fn move_to(&mut self, to: Point) {
        let from = self.state.position;
        if self.state.pen_down {
            self.backend
                .draw_line(from, to, self.state.pen_color, self.state.pen_size);
        }
        self.state.position = to;
        self.backend.update_turtle(self.state);
    }
}

fn point_from_heading(from: Point, heading: f64, distance: f64) -> Point {
    let radians = heading.to_radians();
    Point::new(
        from.x + radians.sin() * distance,
        from.y + radians.cos() * distance,
    )
}

fn normalize_degrees(degrees: f64) -> f64 {
    degrees.rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "expected {a} ≈ {b}");
    }

    #[test]
    fn forward_draws_a_line_when_pen_is_down() {
        let backend = HeadlessTurtleBackend::new();
        let mut world = TurtleWorld::new(backend);
        world.forward(100.0);

        let events = world.backend().events();
        assert!(matches!(events[1], TurtleEvent::Line { .. }));
        let TurtleEvent::Line { from, to, .. } = events[1] else {
            unreachable!();
        };
        assert_eq!(from, Point::new(0.0, 0.0));
        approx_eq(to.x, 0.0);
        approx_eq(to.y, 100.0);
    }

    #[test]
    fn right_turn_uses_logo_heading_convention() {
        let backend = HeadlessTurtleBackend::new();
        let mut world = TurtleWorld::new(backend);
        world.right(90.0);
        world.forward(50.0);
        let TurtleEvent::Line { to, .. } = world.backend().events()[2] else {
            panic!("expected line");
        };
        approx_eq(to.x, 50.0);
        approx_eq(to.y, 0.0);
    }

    #[test]
    fn pen_up_moves_without_drawing() {
        let backend = HeadlessTurtleBackend::new();
        let mut world = TurtleWorld::new(backend);
        world.pen_up();
        world.forward(10.0);
        assert!(!world
            .backend()
            .events()
            .iter()
            .any(|event| matches!(event, TurtleEvent::Line { .. })));
    }

    #[test]
    fn clearscreen_resets_position_and_heading() {
        let backend = HeadlessTurtleBackend::new();
        let mut world = TurtleWorld::new(backend);
        world.right(90.0);
        world.forward(10.0);
        world.clearscreen();
        assert_eq!(world.state().position, Point::new(0.0, 0.0));
        assert_eq!(world.state().heading, 0.0);
        assert!(world
            .backend()
            .events()
            .iter()
            .any(|event| matches!(event, TurtleEvent::Clear)));
    }
}
