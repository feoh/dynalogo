//! Graphics-free turtle primitives shared by `dynaturtle::TurtleStore`.
//!
//! The interpreter core owns turtle semantics, but not a windowing or drawing
//! library; frontends render from `TurtleEvent`/`TurtleState` snapshots.

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
    Label {
        at: Point,
        text: String,
        color: u32,
        height: f64,
    },
    Fill {
        at: Point,
        color: u32,
    },
    State(TurtleState),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurtleState {
    pub position: Point,
    /// Degrees, where 0 points north/up, matching Logo convention.
    pub heading: f64,
    pub pen_down: bool,
    pub pen_color: u32,
    pub pen_size: f64,
    pub label_height: f64,
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
            label_height: 12.0,
            visible: true,
        }
    }
}

/// Projects a point `distance` units forward along `heading` (degrees, 0 = north).
pub(crate) fn point_from_heading(from: Point, heading: f64, distance: f64) -> Point {
    let radians = heading.to_radians();
    Point::new(
        from.x + radians.sin() * distance,
        from.y + radians.cos() * distance,
    )
}
