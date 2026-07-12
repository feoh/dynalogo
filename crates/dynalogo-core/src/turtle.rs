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

/// Atari LOGO's four pen modes: `PU`/`PD` gate whether the pen draws at
/// all, while `PE`/`PX` change how a drawn line composites against
/// whatever is already on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenMode {
    Up,
    Down,
    Erase,
    Reverse,
}

impl PenMode {
    /// The word `PEN` reports for this mode, matching Atari LOGO's PD/PU/PE/PX.
    pub fn as_word(self) -> &'static str {
        match self {
            PenMode::Up => "PU",
            PenMode::Down => "PD",
            PenMode::Erase => "PE",
            PenMode::Reverse => "PX",
        }
    }

    /// Whether this mode leaves a mark on the canvas at all (`PU` does not).
    pub fn draws(self) -> bool {
        self != PenMode::Up
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
        /// How this segment should composite against the canvas.
        ///
        /// `PenMode::Reverse` (Atari's `PX`) is tracked here so frontends
        /// can distinguish it, but true per-pixel XOR compositing is not
        /// implemented: the vector event-replay renderers in this
        /// workspace have no persistent raster canvas to invert, so
        /// `Reverse` segments currently render identically to `Down`.
        mode: PenMode,
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
    pub pen_mode: PenMode,
    /// The active pen's color, kept in sync with `pens[active_pen - 1]`.
    pub pen_color: u32,
    /// Atari LOGO gives each turtle 3 independently colored pens.
    pub pens: [u32; 3],
    /// 1-based index into `pens`, as reported/selected by `PN`/`SETPN`.
    pub active_pen: u8,
    pub pen_size: f64,
    pub label_height: f64,
    pub visible: bool,
}

impl Default for TurtleState {
    fn default() -> Self {
        Self {
            position: Point::new(0.0, 0.0),
            heading: 0.0,
            pen_mode: PenMode::Down,
            pen_color: 0x00ff_ffff,
            pens: [0x00ff_ffff; 3],
            active_pen: 1,
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

/// Returns the suffix of `events` since the most recent `TurtleEvent::Clear`,
/// i.e. the events that should still be visible/considered on screen.
pub fn events_since_clear(events: &[TurtleEvent]) -> &[TurtleEvent] {
    let start = events
        .iter()
        .rposition(|event| matches!(event, TurtleEvent::Clear))
        .map_or(0, |index| index + 1);
    &events[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(x: f64) -> TurtleEvent {
        TurtleEvent::Line {
            from: Point::new(0.0, 0.0),
            to: Point::new(x, 0.0),
            color: 0,
            width: 1.0,
            mode: PenMode::Down,
        }
    }

    #[test]
    fn events_since_clear_returns_all_events_when_no_clear_present() {
        let events = vec![line(1.0), line(2.0)];
        assert_eq!(events_since_clear(&events), &events[..]);
    }

    #[test]
    fn events_since_clear_returns_empty_slice_immediately_after_clear() {
        let events = vec![line(1.0), TurtleEvent::Clear];
        assert!(events_since_clear(&events).is_empty());
    }

    #[test]
    fn events_since_clear_returns_events_after_most_recent_clear() {
        let events = vec![
            line(1.0),
            TurtleEvent::Clear,
            line(2.0),
            TurtleEvent::Clear,
            line(3.0),
            line(4.0),
        ];
        assert_eq!(events_since_clear(&events), &events[4..]);
    }

    #[test]
    fn events_since_clear_on_empty_events_returns_empty() {
        let events: Vec<TurtleEvent> = Vec::new();
        assert!(events_since_clear(&events).is_empty());
    }
}
