//! Software rasterizer for `TurtleEvent` streams.
//!
//! Frontends draw lines directly, but `FILL`/`FILLED` need real flood-fill
//! semantics rather than a stub marker, so this renders onto a pixel buffer
//! that frontends can blit.

use crate::turtle::{events_since_clear, PenMode, Point, TurtleEvent};

#[derive(Debug, Clone, PartialEq)]
pub struct SoftwareCanvas {
    width: usize,
    height: usize,
    pixels: Vec<Option<u32>>,
}

/// Incremental raster cache for a turtle-event trail.
///
/// Frontends can keep one of these alive across frames, call `update` with the
/// complete event log, and upload the RGBA bytes only when the return value is
/// `true`. The cache falls back to a full replay when the event log is truncated
/// or `SETSCRUNCH` changes, but otherwise applies only newly appended drawing
/// events.
#[derive(Debug, Clone, PartialEq)]
pub struct RasterCache {
    canvas: SoftwareCanvas,
    event_cursor: usize,
    x_scrunch: f64,
    y_scrunch: f64,
}

impl SoftwareCanvas {
    pub fn new(width: usize, height: usize) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            pixels: vec![None; width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn clear(&mut self) {
        self.pixels.fill(None);
    }

    pub fn pixel(&self, x: usize, y: usize) -> Option<u32> {
        self.pixels[y * self.width + x]
    }

    pub fn write_rgba_bytes(&self, bytes: &mut Vec<u8>) {
        let required_len = self.width * self.height * 4;
        bytes.resize(required_len, 0);
        for (index, pixel) in self.pixels.iter().enumerate() {
            let offset = index * 4;
            if let Some(color) = pixel {
                bytes[offset] = ((color >> 16) & 0xff) as u8;
                bytes[offset + 1] = ((color >> 8) & 0xff) as u8;
                bytes[offset + 2] = (color & 0xff) as u8;
                bytes[offset + 3] = 255;
            } else {
                bytes[offset] = 0;
                bytes[offset + 1] = 0;
                bytes[offset + 2] = 0;
                bytes[offset + 3] = 0;
            }
        }
    }

    pub fn color_at_logo_point(&self, point: Point) -> Option<u32> {
        let (x, y) = self.logo_to_pixel(point)?;
        self.pixel(x, y)
    }

    pub fn rasterize_events(&mut self, events: &[TurtleEvent]) {
        self.clear();
        for event in events {
            self.apply_event(event);
        }
    }

    pub fn apply_event(&mut self, event: &TurtleEvent) {
        match event {
            TurtleEvent::Clear => self.clear(),
            TurtleEvent::Line {
                from,
                to,
                color,
                width,
                mode,
            } => self.draw_line(*from, *to, *color, *width, *mode),
            TurtleEvent::Fill { at, color } => self.flood_fill(*at, *color),
            TurtleEvent::Label { .. } | TurtleEvent::State(_) => {}
        }
    }

    fn draw_line(&mut self, from: Point, to: Point, color: u32, width: f64, mode: PenMode) {
        let (mut x0, mut y0) = self.logo_to_pixel_i32(from);
        let (x1, y1) = self.logo_to_pixel_i32(to);
        let brush_radius = ((width.max(1.0) - 1.0) / 2.0).ceil() as i32;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.stamp_brush(x0, y0, brush_radius, color, mode);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn stamp_brush(&mut self, x: i32, y: i32, radius: i32, color: u32, mode: PenMode) {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius {
                    match mode {
                        PenMode::Up => {}
                        PenMode::Down => self.set_pixel(x + dx, y + dy, color),
                        PenMode::Reverse => self.xor_pixel(x + dx, y + dy, color),
                        PenMode::Erase => self.clear_pixel(x + dx, y + dy),
                    }
                }
            }
        }
    }

    fn flood_fill(&mut self, at: Point, color: u32) {
        let (x, y) = self.logo_to_pixel_i32(at);
        if !self.in_bounds(x, y) || self.pixel_i32(x, y).is_some() {
            return;
        }
        let mut stack = vec![(x, y)];
        while let Some((x, y)) = stack.pop() {
            if !self.in_bounds(x, y) || self.pixel_i32(x, y).is_some() {
                continue;
            }
            self.set_pixel(x, y, color);
            stack.push((x + 1, y));
            stack.push((x - 1, y));
            stack.push((x, y + 1));
            stack.push((x, y - 1));
        }
    }

    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32
    }

    fn pixel_i32(&self, x: i32, y: i32) -> Option<u32> {
        if !self.in_bounds(x, y) {
            return None;
        }
        self.pixels[y as usize * self.width + x as usize]
    }

    fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if self.in_bounds(x, y) {
            let index = y as usize * self.width + x as usize;
            self.pixels[index] = Some(color);
        }
    }

    fn clear_pixel(&mut self, x: i32, y: i32) {
        if self.in_bounds(x, y) {
            let index = y as usize * self.width + x as usize;
            self.pixels[index] = None;
        }
    }

    fn xor_pixel(&mut self, x: i32, y: i32, color: u32) {
        if self.in_bounds(x, y) {
            let index = y as usize * self.width + x as usize;
            let current = self.pixels[index].unwrap_or(0);
            let reversed = (current ^ color) & 0x00ff_ffff;
            self.pixels[index] = (reversed != 0).then_some(reversed);
        }
    }

    fn logo_to_pixel(&self, point: Point) -> Option<(usize, usize)> {
        let (x, y) = self.logo_to_pixel_i32(point);
        if self.in_bounds(x, y) {
            Some((x as usize, y as usize))
        } else {
            None
        }
    }

    fn logo_to_pixel_i32(&self, point: Point) -> (i32, i32) {
        let x = (self.width as f64 / 2.0 + point.x).round() as i32;
        let y = (self.height as f64 / 2.0 - point.y).round() as i32;
        (x, y)
    }
}

impl RasterCache {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            canvas: SoftwareCanvas::new(width, height),
            event_cursor: 0,
            x_scrunch: 1.0,
            y_scrunch: 1.0,
        }
    }

    pub fn width(&self) -> usize {
        self.canvas.width()
    }

    pub fn height(&self) -> usize {
        self.canvas.height()
    }

    pub fn canvas(&self) -> &SoftwareCanvas {
        &self.canvas
    }

    pub fn event_cursor(&self) -> usize {
        self.event_cursor
    }

    pub fn write_rgba_bytes(&self, bytes: &mut Vec<u8>) {
        self.canvas.write_rgba_bytes(bytes);
    }

    pub fn update(&mut self, events: &[TurtleEvent], x_scrunch: f64, y_scrunch: f64) -> bool {
        if self.event_cursor > events.len()
            || self.x_scrunch != x_scrunch
            || self.y_scrunch != y_scrunch
        {
            self.rebuild(events, x_scrunch, y_scrunch);
            return true;
        }

        let mut dirty = false;
        for event in &events[self.event_cursor..] {
            let event = scale_event(event, x_scrunch, y_scrunch);
            if event_affects_pixels(&event) {
                dirty = true;
            }
            self.canvas.apply_event(&event);
        }
        self.event_cursor = events.len();
        dirty
    }

    pub fn rebuild(&mut self, events: &[TurtleEvent], x_scrunch: f64, y_scrunch: f64) {
        self.canvas.clear();
        self.x_scrunch = x_scrunch;
        self.y_scrunch = y_scrunch;
        for event in events_since_clear(events) {
            self.canvas
                .apply_event(&scale_event(event, x_scrunch, y_scrunch));
        }
        self.event_cursor = events.len();
    }
}

pub fn scale_event(event: &TurtleEvent, x_scrunch: f64, y_scrunch: f64) -> TurtleEvent {
    match event {
        TurtleEvent::Line {
            from,
            to,
            color,
            width,
            mode,
        } => TurtleEvent::Line {
            from: scale_point(*from, x_scrunch, y_scrunch),
            to: scale_point(*to, x_scrunch, y_scrunch),
            color: *color,
            width: *width,
            mode: *mode,
        },
        TurtleEvent::Label {
            at,
            text,
            color,
            height,
        } => TurtleEvent::Label {
            at: scale_point(*at, x_scrunch, y_scrunch),
            text: text.clone(),
            color: *color,
            height: *height,
        },
        TurtleEvent::Fill { at, color } => TurtleEvent::Fill {
            at: scale_point(*at, x_scrunch, y_scrunch),
            color: *color,
        },
        TurtleEvent::State(state) => {
            let mut state = *state;
            state.position = scale_point(state.position, x_scrunch, y_scrunch);
            TurtleEvent::State(state)
        }
        TurtleEvent::Clear => TurtleEvent::Clear,
    }
}

pub fn scale_point(point: Point, x_scrunch: f64, y_scrunch: f64) -> Point {
    Point {
        x: point.x * x_scrunch,
        y: point.y * y_scrunch,
    }
}

fn event_affects_pixels(event: &TurtleEvent) -> bool {
    matches!(
        event,
        TurtleEvent::Clear | TurtleEvent::Line { .. } | TurtleEvent::Fill { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flood_fill_stops_at_drawn_boundaries() {
        let mut canvas = SoftwareCanvas::new(41, 41);
        canvas.rasterize_events(&[
            TurtleEvent::Line {
                from: Point::new(-5.0, -5.0),
                to: Point::new(5.0, -5.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(5.0, -5.0),
                to: Point::new(5.0, 5.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(5.0, 5.0),
                to: Point::new(-5.0, 5.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(-5.0, 5.0),
                to: Point::new(-5.0, -5.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Fill {
                at: Point::new(0.0, 0.0),
                color: 9,
            },
        ]);

        assert_eq!(canvas.color_at_logo_point(Point::new(0.0, 0.0)), Some(9));
        assert_eq!(canvas.color_at_logo_point(Point::new(-5.0, -5.0)), Some(3));
        assert_eq!(canvas.color_at_logo_point(Point::new(8.0, 0.0)), None);
    }

    #[test]
    fn flood_fill_is_noop_when_seed_lands_on_drawn_edge() {
        let mut canvas = SoftwareCanvas::new(41, 41);
        canvas.rasterize_events(&[
            TurtleEvent::Line {
                from: Point::new(-5.0, -5.0),
                to: Point::new(5.0, -5.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(5.0, -5.0),
                to: Point::new(5.0, 5.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(5.0, 5.0),
                to: Point::new(-5.0, 5.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(-5.0, 5.0),
                to: Point::new(-5.0, -5.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Fill {
                at: Point::new(5.0, 5.0),
                color: 9,
            },
        ]);

        assert_eq!(canvas.color_at_logo_point(Point::new(5.0, 5.0)), Some(3));
        assert_eq!(canvas.color_at_logo_point(Point::new(0.0, 0.0)), None);
    }

    #[test]
    fn flood_fill_chooses_innermost_region_containing_the_seed() {
        let mut canvas = SoftwareCanvas::new(61, 61);
        canvas.rasterize_events(&[
            TurtleEvent::Line {
                from: Point::new(-10.0, -10.0),
                to: Point::new(10.0, -10.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(10.0, -10.0),
                to: Point::new(10.0, 10.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(10.0, 10.0),
                to: Point::new(-10.0, 10.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(-10.0, 10.0),
                to: Point::new(-10.0, -10.0),
                color: 3,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(-4.0, -4.0),
                to: Point::new(4.0, -4.0),
                color: 5,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(4.0, -4.0),
                to: Point::new(4.0, 4.0),
                color: 5,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(4.0, 4.0),
                to: Point::new(-4.0, 4.0),
                color: 5,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(-4.0, 4.0),
                to: Point::new(-4.0, -4.0),
                color: 5,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Fill {
                at: Point::new(0.0, 0.0),
                color: 9,
            },
        ]);

        assert_eq!(canvas.color_at_logo_point(Point::new(0.0, 0.0)), Some(9));
        assert_eq!(canvas.color_at_logo_point(Point::new(4.0, 4.0)), Some(5));
        assert_eq!(canvas.color_at_logo_point(Point::new(8.0, 0.0)), None);
    }

    #[test]
    fn clear_event_resets_previous_pixels() {
        let mut canvas = SoftwareCanvas::new(21, 21);
        canvas.rasterize_events(&[
            TurtleEvent::Line {
                from: Point::new(0.0, 0.0),
                to: Point::new(3.0, 0.0),
                color: 7,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Clear,
        ]);

        assert_eq!(canvas.color_at_logo_point(Point::new(0.0, 0.0)), None);
    }

    #[test]
    fn reverse_pen_xors_against_existing_pixels() {
        let mut canvas = SoftwareCanvas::new(21, 21);
        canvas.rasterize_events(&[
            TurtleEvent::Line {
                from: Point::new(0.0, 0.0),
                to: Point::new(0.0, 0.0),
                color: 0x000f_000f,
                width: 1.0,
                mode: PenMode::Down,
            },
            TurtleEvent::Line {
                from: Point::new(0.0, 0.0),
                to: Point::new(0.0, 0.0),
                color: 0x00f0_00ff,
                width: 1.0,
                mode: PenMode::Reverse,
            },
        ]);

        assert_eq!(
            canvas.color_at_logo_point(Point::new(0.0, 0.0)),
            Some(0x00ff_00f0)
        );
    }

    #[test]
    fn reverse_pen_can_restore_background_on_second_pass() {
        let mut canvas = SoftwareCanvas::new(21, 21);
        let line = TurtleEvent::Line {
            from: Point::new(-2.0, 0.0),
            to: Point::new(2.0, 0.0),
            color: 0x0000_00aa,
            width: 1.0,
            mode: PenMode::Reverse,
        };
        canvas.rasterize_events(&[line.clone(), line]);

        assert_eq!(canvas.color_at_logo_point(Point::new(0.0, 0.0)), None);
    }

    #[test]
    fn label_events_are_ignored_by_the_rasterizer() {
        let mut canvas = SoftwareCanvas::new(21, 21);
        canvas.rasterize_events(&[TurtleEvent::Label {
            at: Point::new(0.0, 0.0),
            text: "hi".to_string(),
            color: 7,
            height: 12.0,
        }]);

        assert_eq!(canvas.color_at_logo_point(Point::new(0.0, 0.0)), None);
    }

    #[test]
    fn canvas_writes_transparent_rgba_for_empty_pixels() {
        let mut canvas = SoftwareCanvas::new(3, 3);
        canvas.rasterize_events(&[TurtleEvent::Line {
            from: Point::new(0.0, 0.0),
            to: Point::new(0.0, 0.0),
            color: 0x12_34_56,
            width: 1.0,
            mode: PenMode::Down,
        }]);

        let mut bytes = Vec::new();
        canvas.write_rgba_bytes(&mut bytes);

        assert!(bytes
            .chunks_exact(4)
            .any(|pixel| pixel == [0x12, 0x34, 0x56, 255]));
        assert_eq!(&bytes[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn raster_cache_applies_only_new_events() {
        let line_a = TurtleEvent::Line {
            from: Point::new(0.0, 0.0),
            to: Point::new(1.0, 0.0),
            color: 1,
            width: 1.0,
            mode: PenMode::Down,
        };
        let line_b = TurtleEvent::Line {
            from: Point::new(0.0, 1.0),
            to: Point::new(1.0, 1.0),
            color: 2,
            width: 1.0,
            mode: PenMode::Down,
        };
        let mut cache = RasterCache::new(21, 21);
        assert!(cache.update(std::slice::from_ref(&line_a), 1.0, 1.0));
        assert_eq!(cache.event_cursor(), 1);
        assert!(!cache.update(std::slice::from_ref(&line_a), 1.0, 1.0));

        assert!(cache.update(&[line_a, line_b], 1.0, 1.0));
        assert_eq!(cache.event_cursor(), 2);
        assert_eq!(
            cache.canvas().color_at_logo_point(Point::new(0.0, 0.0)),
            Some(1)
        );
        assert_eq!(
            cache.canvas().color_at_logo_point(Point::new(0.0, 1.0)),
            Some(2)
        );
    }

    #[test]
    fn raster_cache_rebuilds_from_visible_suffix_after_clear() {
        let old_line = TurtleEvent::Line {
            from: Point::new(-3.0, 0.0),
            to: Point::new(-3.0, 0.0),
            color: 1,
            width: 1.0,
            mode: PenMode::Down,
        };
        let new_line = TurtleEvent::Line {
            from: Point::new(3.0, 0.0),
            to: Point::new(3.0, 0.0),
            color: 2,
            width: 1.0,
            mode: PenMode::Down,
        };
        let events = vec![old_line, TurtleEvent::Clear, new_line];

        let mut cache = RasterCache::new(21, 21);
        cache.rebuild(&events, 1.0, 1.0);

        assert_eq!(
            cache.canvas().color_at_logo_point(Point::new(-3.0, 0.0)),
            None
        );
        assert_eq!(
            cache.canvas().color_at_logo_point(Point::new(3.0, 0.0)),
            Some(2)
        );
        assert_eq!(cache.event_cursor(), events.len());
    }

    #[test]
    fn raster_cache_rebuilds_when_scrunch_changes() {
        let events = vec![TurtleEvent::Line {
            from: Point::new(2.0, 0.0),
            to: Point::new(2.0, 0.0),
            color: 9,
            width: 1.0,
            mode: PenMode::Down,
        }];
        let mut cache = RasterCache::new(21, 21);
        cache.update(&events, 1.0, 1.0);

        assert!(cache.update(&events, 2.0, 1.0));
        assert_eq!(
            cache.canvas().color_at_logo_point(Point::new(2.0, 0.0)),
            None
        );
        assert_eq!(
            cache.canvas().color_at_logo_point(Point::new(4.0, 0.0)),
            Some(9)
        );
    }
}
