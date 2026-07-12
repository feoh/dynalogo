use dynalogo_core::dynaturtle::TurtleId;
use dynalogo_core::graphics::SoftwareCanvas;
use dynalogo_core::turtle::{events_since_clear, Point, TurtleEvent, TurtleState};
use dynalogo_core::value::{List, Value};
use dynalogo_core::vm::{ControlFlow, Vm};
use macroquad::audio::{load_sound_from_bytes, play_sound_once, Sound};
use macroquad::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlTextAreaElement;

const PROMPT_HEIGHT: f32 = 92.0;
const LOG_LINES: usize = 5;
const SIM_DT: f64 = 1.0 / 60.0;
const CANVAS_BACKGROUND: Color = Color::new(18.0 / 255.0, 20.0 / 255.0, 26.0 / 255.0, 1.0);

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new().await;

    loop {
        clear_background(CANVAS_BACKGROUND);
        app.handle_input();
        app.handle_browser_commands();
        app.update_sim();
        app.update_audio();
        app.draw();
        next_frame().await;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "DynaLOGO".to_string(),
        window_width: 1024,
        window_height: 768,
        high_dpi: true,
        ..Default::default()
    }
}

struct App {
    vm: Vm,
    input: String,
    log: Vec<String>,
    bark_sound: Option<Sound>,
    last_toot: Option<[u8; 4]>,
    bark_flash_until: f64,
}

impl App {
    async fn new() -> Self {
        let app = Self {
            vm: Vm::new(),
            input: String::new(),
            log: vec![
                "Type Logo commands, then Enter. Try: tell [0 1 2] each [setshape \"dog 12]"
                    .to_string(),
            ],
            bark_sound: load_sound_from_bytes(&generate_bark_wav()).await.ok(),
            last_toot: None,
            bark_flash_until: 0.0,
        };
        sync_browser_log(&app.log);
        app
    }

    fn handle_input(&mut self) {
        let mut events = Vec::new();
        while let Some(ch) = get_char_pressed() {
            if !ch.is_control() {
                events.push(InputEvent::Char(ch));
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            events.push(InputEvent::Backspace);
        }
        if is_key_pressed(KeyCode::Enter) {
            events.push(InputEvent::Submit);
        }
        if is_key_pressed(KeyCode::Escape) {
            events.push(InputEvent::Cancel);
        }

        for command in process_input_events(&mut self.input, events) {
            self.eval_command(command);
        }
    }

    fn handle_browser_commands(&mut self) {
        for command in process_browser_commands(drain_browser_commands()) {
            self.eval_command(command);
        }
    }

    fn eval_command(&mut self, command: String) {
        self.push_log(format!("? {command}"));
        match self.vm.eval_source(&command) {
            Ok(result) => {
                let output = result.output;
                for line in output.lines() {
                    self.push_log(line.to_string());
                }
                self.vm.clear_output();
                for value in result.stack {
                    self.push_log(value.show(self.vm.interner()));
                }
            }
            Err(error) => self.push_log(format!("Error: {error}")),
        }
    }

    fn update_sim(&mut self) {
        match self.vm.dynaturtle_tick(SIM_DT) {
            Ok(ControlFlow::None | ControlFlow::Stop) => {}
            Ok(ControlFlow::Output(value)) => self.push_log(value.show(self.vm.interner())),
            Ok(ControlFlow::Continue) => {
                self.push_log("Error: CONTINUE can only be used inside PAUSE".to_string())
            }
            Ok(ControlFlow::Throw { tag, value }) => self.push_log(format!(
                "Uncaught THROW {} {}",
                tag.show(self.vm.interner()),
                value.show(self.vm.interner())
            )),
            Err(error) => self.push_log(format!("Error: {error}")),
        }

        if !self.vm.output().is_empty() {
            let output = self.vm.output().to_string();
            self.vm.clear_output();
            for line in output.lines() {
                self.push_log(line.to_string());
            }
        }
    }

    fn update_audio(&mut self) {
        let current = self.vm.last_toot();
        if current != self.last_toot {
            if current.is_some() {
                if let Some(sound) = &self.bark_sound {
                    play_sound_once(sound);
                }
                self.bark_flash_until = get_time() + 0.4;
            }
            self.last_toot = current;
        }
    }

    fn draw(&self) {
        self.draw_canvas();
        self.draw_prompt();
    }

    fn draw_canvas(&self) {
        let canvas_height = screen_height() - PROMPT_HEIGHT;
        let screen_w = screen_width();
        draw_rectangle_lines(0.0, 0.0, screen_w, canvas_height, 2.0, GRAY);
        draw_line(
            screen_w / 2.0,
            0.0,
            screen_w / 2.0,
            canvas_height,
            1.0,
            Color::from_rgba(55, 60, 70, 255),
        );
        draw_line(
            0.0,
            canvas_height / 2.0,
            screen_w,
            canvas_height / 2.0,
            1.0,
            Color::from_rgba(55, 60, 70, 255),
        );

        let (x_scrunch, y_scrunch) = self.vm.turtles().scrunch();
        let visible: Vec<TurtleEvent> = events_since_clear(self.vm.turtles().events())
            .iter()
            .map(|event| scale_event(event, x_scrunch, y_scrunch))
            .collect();

        let mut canvas = SoftwareCanvas::new(screen_w as usize, canvas_height.max(1.0) as usize);
        canvas.rasterize_events(&visible);
        for y in 0..canvas.height() {
            let mut x = 0;
            while x < canvas.width() {
                let Some(color) = canvas.pixel(x, y) else {
                    x += 1;
                    continue;
                };
                let start_x = x;
                while x < canvas.width() && canvas.pixel(x, y) == Some(color) {
                    x += 1;
                }
                draw_rectangle(
                    start_x as f32,
                    y as f32,
                    (x - start_x) as f32,
                    1.0,
                    logo_color(color),
                );
            }
        }

        for event in &visible {
            if let TurtleEvent::Label {
                at,
                text,
                color,
                height,
            } = event
            {
                let at = logo_to_screen(*at, screen_w, canvas_height);
                draw_text(
                    text,
                    at.x,
                    at.y,
                    (*height).max(1.0) as f32,
                    logo_color(*color),
                );
            }
        }

        for (index, state) in self.vm.turtles().snapshots().into_iter().enumerate() {
            self.draw_turtle(
                TurtleId::new(index),
                state,
                screen_w,
                canvas_height,
                x_scrunch,
                y_scrunch,
            );
        }

        if self.bark_flash_until > get_time() {
            draw_text("TOOT!", 18.0, 28.0, 30.0, ORANGE);
        }
    }

    fn draw_turtle(
        &self,
        id: TurtleId,
        state: TurtleState,
        screen_w: f32,
        canvas_height: f32,
        x_scrunch: f64,
        y_scrunch: f64,
    ) {
        if !state.visible {
            return;
        }

        let center = logo_to_screen(
            scale_point(state.position, x_scrunch, y_scrunch),
            screen_w,
            canvas_height,
        );
        let (forward, right) = heading_to_vectors(state.heading);
        let shape = self.vm.turtles().shape(id).unwrap_or("turtle");
        let phase = (get_time() as f32 * 8.0).sin();

        match sprite_kind_for_shape(shape) {
            SpriteKind::Dog => draw_dog_sprite(center, forward, right, phase),
            SpriteKind::Ship => draw_ship_sprite(center, forward, right, phase),
            SpriteKind::Turtle => {
                if let Some(definition) = self.vm.shape_definition(shape) {
                    if let Some(points) = custom_shape_points(&definition) {
                        draw_custom_shape(
                            center,
                            forward,
                            right,
                            &points,
                            logo_color(state.pen_color),
                        );
                    } else {
                        draw_turtle_sprite(
                            center,
                            forward,
                            right,
                            phase,
                            logo_color(state.pen_color),
                        )
                    }
                } else {
                    draw_turtle_sprite(center, forward, right, phase, logo_color(state.pen_color))
                }
            }
        }
    }

    fn draw_prompt(&self) {
        let top = screen_height() - PROMPT_HEIGHT;
        draw_rectangle(
            0.0,
            top,
            screen_width(),
            PROMPT_HEIGHT,
            Color::from_rgba(8, 10, 14, 255),
        );
        draw_line(0.0, top, screen_width(), top, 2.0, DARKGRAY);

        let mut y = top + 18.0;
        let start = self.log.len().saturating_sub(LOG_LINES);
        for line in &self.log[start..] {
            draw_text(line, 12.0, y, 18.0, LIGHTGRAY);
            y += 16.0;
        }

        let cursor = if ((get_time() * 2.0) as i64) % 2 == 0 {
            "_"
        } else {
            ""
        };
        draw_text(
            format!("? {}{cursor}", self.input),
            12.0,
            screen_height() - 14.0,
            22.0,
            WHITE,
        );
    }

    fn push_log(&mut self, line: String) {
        append_log_line(&mut self.log, line, LOG_LINES);
        sync_browser_log(&self.log);
    }
}

enum InputEvent {
    Char(char),
    Backspace,
    Submit,
    Cancel,
}

fn process_input_events(
    input: &mut String,
    events: impl IntoIterator<Item = InputEvent>,
) -> Vec<String> {
    let mut commands = Vec::new();
    for event in events {
        match event {
            InputEvent::Char(ch) => input.push(ch),
            InputEvent::Backspace => {
                input.pop();
            }
            InputEvent::Submit => {
                let command = input.trim().to_string();
                input.clear();
                if !command.is_empty() {
                    commands.push(command);
                }
            }
            InputEvent::Cancel => input.clear(),
        }
    }
    commands
}

fn process_browser_commands(commands: Vec<String>) -> Vec<String> {
    commands
        .into_iter()
        .filter(|command| !command.trim().is_empty())
        .collect()
}

fn append_log_line(log: &mut Vec<String>, line: String, max_visible_lines: usize) {
    log.push(line);
    let keep = max_visible_lines * 2;
    if log.len() > keep {
        log.drain(0..log.len() - keep);
    }
}

fn draw_turtle_sprite(center: Vec2, forward: Vec2, right: Vec2, phase: f32, shell: Color) {
    let shell_center = center - forward * 1.5;
    draw_circle(shell_center.x, shell_center.y, 10.0, shell);
    draw_circle_lines(shell_center.x, shell_center.y, 10.0, 1.5, BLACK);

    let head = center + forward * 12.0;
    draw_circle(head.x, head.y, 4.5, Color::from_rgba(124, 191, 110, 255));

    let flipper = phase * 3.0;
    draw_circle(
        (shell_center + right * 8.0 + forward * flipper).x,
        (shell_center + right * 8.0 + forward * flipper).y,
        3.2,
        Color::from_rgba(124, 191, 110, 255),
    );
    draw_circle(
        (shell_center - right * 8.0 - forward * flipper).x,
        (shell_center - right * 8.0 - forward * flipper).y,
        3.2,
        Color::from_rgba(124, 191, 110, 255),
    );
    draw_circle(
        (shell_center + right * 6.0 - forward * 7.0).x,
        (shell_center + right * 6.0 - forward * 7.0).y,
        2.8,
        Color::from_rgba(124, 191, 110, 255),
    );
    draw_circle(
        (shell_center - right * 6.0 - forward * 7.0).x,
        (shell_center - right * 6.0 - forward * 7.0).y,
        2.8,
        Color::from_rgba(124, 191, 110, 255),
    );
}

fn draw_dog_sprite(center: Vec2, forward: Vec2, right: Vec2, phase: f32) {
    let fur = Color::from_rgba(186, 133, 85, 255);
    let chest = center - forward * 2.0;
    draw_circle(chest.x, chest.y, 9.0, fur);
    draw_circle_lines(chest.x, chest.y, 9.0, 1.5, BLACK);

    let head = center + forward * 11.0;
    draw_circle(head.x, head.y, 5.5, fur);
    let nose = head + forward * 4.0;
    draw_circle(nose.x, nose.y, 1.8, BLACK);
    let ear = head - forward * 2.0;
    draw_line(
        (ear + right * 3.0).x,
        (ear + right * 3.0).y,
        (ear + right * 6.0 - forward * 2.0).x,
        (ear + right * 6.0 - forward * 2.0).y,
        2.0,
        BLACK,
    );
    draw_line(
        (ear - right * 3.0).x,
        (ear - right * 3.0).y,
        (ear - right * 6.0 - forward * 2.0).x,
        (ear - right * 6.0 - forward * 2.0).y,
        2.0,
        BLACK,
    );

    let leg_swing = phase * 3.0;
    for offset in [right * 4.0, -right * 4.0] {
        let shoulder = chest + offset - forward * 2.0;
        let hip = chest + offset - forward * 7.0;
        draw_line(
            shoulder.x,
            shoulder.y,
            (shoulder + Vec2::new(leg_swing, 9.0)).x,
            (shoulder + Vec2::new(leg_swing, 9.0)).y,
            2.0,
            BLACK,
        );
        draw_line(
            hip.x,
            hip.y,
            (hip + Vec2::new(-leg_swing, 9.0)).x,
            (hip + Vec2::new(-leg_swing, 9.0)).y,
            2.0,
            BLACK,
        );
    }

    let tail_base = chest - forward * 9.0;
    let wag = right * (phase * 5.0);
    draw_line(
        tail_base.x,
        tail_base.y,
        (tail_base - forward * 5.0 + wag).x,
        (tail_base - forward * 5.0 + wag).y,
        2.5,
        fur,
    );
}

fn draw_ship_sprite(center: Vec2, forward: Vec2, right: Vec2, phase: f32) {
    let hull = Color::from_rgba(120, 190, 255, 255);
    let tip = center + forward * 16.0;
    let back = center - forward * 10.0;
    let left = back - right * 8.0;
    let right_point = back + right * 8.0;
    draw_triangle(tip, left, right_point, hull);
    draw_triangle_lines(tip, left, right_point, 1.5, BLACK);

    let canopy = center + forward * 3.0;
    draw_circle(
        canopy.x,
        canopy.y,
        4.0,
        Color::from_rgba(220, 240, 255, 255),
    );

    let flame = 6.0 + (phase + 1.0) * 2.0;
    let thruster = center - forward * 11.0;
    draw_triangle(
        thruster - right * 3.0,
        thruster + right * 3.0,
        thruster - forward * flame,
        ORANGE,
    );
}

fn scale_point(point: Point, x_scrunch: f64, y_scrunch: f64) -> Point {
    Point::new(point.x * x_scrunch, point.y * y_scrunch)
}

fn scale_event(event: &TurtleEvent, x_scrunch: f64, y_scrunch: f64) -> TurtleEvent {
    match event {
        TurtleEvent::Clear => TurtleEvent::Clear,
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
        TurtleEvent::State(state) => TurtleEvent::State(*state),
    }
}

fn logo_to_screen(point: Point, screen_width: f32, canvas_height: f32) -> Vec2 {
    Vec2::new(
        screen_width / 2.0 + point.x as f32,
        canvas_height / 2.0 - point.y as f32,
    )
}

/// Forward/right unit vectors for a turtle heading, in Logo's convention
/// (0 degrees points north/up, increasing clockwise).
fn heading_to_vectors(heading_deg: f64) -> (Vec2, Vec2) {
    let heading = heading_deg.to_radians() as f32;
    let forward = Vec2::new(heading.sin(), -heading.cos());
    let right = Vec2::new(forward.y, -forward.x);
    (forward, right)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpriteKind {
    Turtle,
    Dog,
    Ship,
}

fn sprite_kind_for_shape(shape: &str) -> SpriteKind {
    if shape.eq_ignore_ascii_case("dog") {
        SpriteKind::Dog
    } else if shape.eq_ignore_ascii_case("ship") || shape.eq_ignore_ascii_case("rocket") {
        SpriteKind::Ship
    } else {
        SpriteKind::Turtle
    }
}

fn custom_shape_points(value: &Value) -> Option<Vec<Vec2>> {
    let Value::List(points) = value else {
        return None;
    };
    let parsed: Option<Vec<Vec2>> = points.iter().map(point_from_value).collect();
    let parsed = parsed?;
    if parsed.len() < 2 {
        None
    } else {
        Some(parsed)
    }
}

fn point_from_value(value: &Value) -> Option<Vec2> {
    let Value::List(pair) = value else {
        return None;
    };
    if pair.len() != 2 {
        return None;
    }
    let x = point_part(pair, 1)?;
    let y = point_part(pair, 2)?;
    Some(Vec2::new(x, y))
}

fn point_part(pair: &List, index: usize) -> Option<f32> {
    let value = pair.item(index)?;
    match value {
        Value::Number(number) => Some(number.get() as f32),
        _ => None,
    }
}

fn draw_custom_shape(center: Vec2, forward: Vec2, right: Vec2, points: &[Vec2], color: Color) {
    for i in 0..points.len() {
        let a = project_shape_point(center, forward, right, points[i]);
        let b = project_shape_point(center, forward, right, points[(i + 1) % points.len()]);
        draw_line(a.x, a.y, b.x, b.y, 2.0, color);
    }
}

fn project_shape_point(center: Vec2, forward: Vec2, right: Vec2, point: Vec2) -> Vec2 {
    center + right * point.x - forward * point.y
}

fn logo_color(color: u32) -> Color {
    let r = ((color >> 16) & 0xff) as f32 / 255.0;
    let g = ((color >> 8) & 0xff) as f32 / 255.0;
    let b = (color & 0xff) as f32 / 255.0;
    Color::new(r, g, b, 1.0)
}

#[cfg(target_arch = "wasm32")]
fn drain_browser_commands() -> Vec<String> {
    let Some(window) = web_sys::window() else {
        return Vec::new();
    };
    let global = JsValue::from(window);
    let Ok(value) = js_sys::Reflect::get(&global, &JsValue::from_str("__dynalogoCommands")) else {
        return Vec::new();
    };
    if !js_sys::Array::is_array(&value) {
        return Vec::new();
    }

    let queue = js_sys::Array::from(&value);
    let mut commands = Vec::new();
    while queue.length() > 0 {
        let value = queue.shift();
        if let Some(command) = value.as_string() {
            commands.push(command);
        }
    }
    commands
}

#[cfg(not(target_arch = "wasm32"))]
fn drain_browser_commands() -> Vec<String> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
fn sync_browser_log(log: &[String]) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(element) = document.get_element_by_id("repl-log") else {
        return;
    };
    let Ok(textarea) = element.dyn_into::<HtmlTextAreaElement>() else {
        return;
    };
    textarea.set_value(&log.join("\n"));
    textarea.set_scroll_top(textarea.scroll_height());
}

#[cfg(not(target_arch = "wasm32"))]
fn sync_browser_log(_log: &[String]) {}

fn generate_bark_wav() -> Vec<u8> {
    let sample_rate = 22_050u32;
    let sample_count = (sample_rate as f32 * 0.22) as usize;
    let mut pcm = Vec::with_capacity(sample_count * 2);
    let mut noise = 0x1234_5678u32;

    for i in 0..sample_count {
        let t = i as f32 / sample_rate as f32;
        let env = if t < 0.08 {
            1.0 - (t / 0.08)
        } else if t < 0.16 {
            0.55 * (1.0 - ((t - 0.08) / 0.08))
        } else {
            0.0
        };
        noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise_sample = (((noise >> 16) & 0xff) as f32 / 127.5) - 1.0;
        let tone1 = if (t * 180.0).fract() < 0.5 { 1.0 } else { -1.0 };
        let tone2 = if (t * 320.0).fract() < 0.5 { 1.0 } else { -1.0 };
        let sample =
            (env * (0.55 * tone1 + 0.25 * tone2 + 0.20 * noise_sample) * 0.4).clamp(-1.0, 1.0);
        let sample_i16 = (sample * i16::MAX as f32) as i16;
        pcm.extend_from_slice(&sample_i16.to_le_bytes());
    }

    let data_len = pcm.len() as u32;
    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(&pcm);
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    use dynalogo_core::value::Interner;

    fn assert_vec2_close(actual: Vec2, expected: Vec2) {
        assert!(
            (actual.x - expected.x).abs() < 1e-4 && (actual.y - expected.y).abs() < 1e-4,
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn logo_to_screen_maps_origin_to_center() {
        let screen = logo_to_screen(Point::new(0.0, 0.0), 1024.0, 768.0);
        assert_vec2_close(screen, Vec2::new(512.0, 384.0));
    }

    #[test]
    fn logo_to_screen_x_increases_rightward() {
        let screen = logo_to_screen(Point::new(100.0, 0.0), 1024.0, 768.0);
        assert_vec2_close(screen, Vec2::new(612.0, 384.0));
    }

    #[test]
    fn logo_to_screen_y_increases_upward() {
        let screen = logo_to_screen(Point::new(0.0, 100.0), 1024.0, 768.0);
        assert_vec2_close(screen, Vec2::new(512.0, 284.0));
    }

    #[test]
    fn logo_to_screen_negative_coordinates() {
        let screen = logo_to_screen(Point::new(-50.0, -25.0), 1024.0, 768.0);
        assert_vec2_close(screen, Vec2::new(462.0, 409.0));
    }

    #[test]
    fn heading_zero_points_up() {
        let (forward, right) = heading_to_vectors(0.0);
        assert_vec2_close(forward, Vec2::new(0.0, -1.0));
        assert_vec2_close(right, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn heading_ninety_points_right() {
        let (forward, right) = heading_to_vectors(90.0);
        assert_vec2_close(forward, Vec2::new(1.0, 0.0));
        assert_vec2_close(right, Vec2::new(0.0, -1.0));
    }

    #[test]
    fn heading_one_eighty_points_down() {
        let (forward, right) = heading_to_vectors(180.0);
        assert_vec2_close(forward, Vec2::new(0.0, 1.0));
        assert_vec2_close(right, Vec2::new(1.0, 0.0));
    }

    #[test]
    fn heading_two_seventy_points_left() {
        let (forward, right) = heading_to_vectors(270.0);
        assert_vec2_close(forward, Vec2::new(-1.0, 0.0));
        assert_vec2_close(right, Vec2::new(0.0, 1.0));
    }

    #[test]
    fn heading_wraps_beyond_full_circle() {
        let (forward, _) = heading_to_vectors(450.0);
        assert_vec2_close(forward, Vec2::new(1.0, 0.0));
    }

    #[test]
    fn sprite_kind_selects_dog_case_insensitively() {
        assert_eq!(sprite_kind_for_shape("dog"), SpriteKind::Dog);
        assert_eq!(sprite_kind_for_shape("Dog"), SpriteKind::Dog);
        assert_eq!(sprite_kind_for_shape("DOG"), SpriteKind::Dog);
    }

    #[test]
    fn sprite_kind_selects_ship_or_rocket() {
        assert_eq!(sprite_kind_for_shape("ship"), SpriteKind::Ship);
        assert_eq!(sprite_kind_for_shape("Rocket"), SpriteKind::Ship);
        assert_eq!(sprite_kind_for_shape("ROCKET"), SpriteKind::Ship);
    }

    #[test]
    fn sprite_kind_defaults_to_turtle() {
        assert_eq!(sprite_kind_for_shape("turtle"), SpriteKind::Turtle);
        assert_eq!(sprite_kind_for_shape("unknown"), SpriteKind::Turtle);
        assert_eq!(sprite_kind_for_shape(""), SpriteKind::Turtle);
    }

    #[test]
    fn custom_shape_points_accepts_point_lists() {
        let value = Value::list([
            Value::list([Value::number(0.0), Value::number(10.0)]),
            Value::list([Value::number(8.0), Value::number(-6.0)]),
            Value::list([Value::number(-8.0), Value::number(-6.0)]),
        ]);
        let points = custom_shape_points(&value).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], Vec2::new(0.0, 10.0));
    }

    #[test]
    fn custom_shape_points_rejects_non_point_data() {
        let value = Value::list([Value::word(&mut Interner::new(), "oops")]);
        assert!(custom_shape_points(&value).is_none());
    }

    #[test]
    fn logo_color_unpacks_rgb_channels() {
        let color = logo_color(0xff8040);
        assert!((color.r - 1.0).abs() < 1e-3);
        assert!((color.g - (0x80 as f32 / 255.0)).abs() < 1e-3);
        assert!((color.b - (0x40 as f32 / 255.0)).abs() < 1e-3);
        assert!((color.a - 1.0).abs() < 1e-3);
    }

    #[test]
    fn logo_color_black_and_white() {
        let black = logo_color(0x000000);
        assert_vec2_close(Vec2::new(black.r, black.g), Vec2::new(0.0, 0.0));
        assert!((black.b).abs() < 1e-3);

        let white = logo_color(0xffffff);
        assert!((white.r - 1.0).abs() < 1e-3);
        assert!((white.g - 1.0).abs() < 1e-3);
        assert!((white.b - 1.0).abs() < 1e-3);
    }

    #[test]
    fn process_input_events_builds_typed_command() {
        let mut input = String::new();
        let commands = process_input_events(
            &mut input,
            [
                InputEvent::Char('f'),
                InputEvent::Char('d'),
                InputEvent::Char(' '),
                InputEvent::Char('5'),
                InputEvent::Submit,
            ],
        );
        assert_eq!(commands, vec!["fd 5".to_string()]);
        assert!(input.is_empty());
    }

    #[test]
    fn process_input_events_backspace_removes_last_char() {
        let mut input = String::new();
        let commands = process_input_events(
            &mut input,
            [
                InputEvent::Char('f'),
                InputEvent::Char('x'),
                InputEvent::Backspace,
                InputEvent::Char('d'),
                InputEvent::Submit,
            ],
        );
        assert_eq!(commands, vec!["fd".to_string()]);
    }

    #[test]
    fn process_input_events_cancel_clears_input_without_submitting() {
        let mut input = String::new();
        let commands = process_input_events(
            &mut input,
            [
                InputEvent::Char('f'),
                InputEvent::Char('d'),
                InputEvent::Cancel,
            ],
        );
        assert!(commands.is_empty());
        assert!(input.is_empty());
    }

    #[test]
    fn process_input_events_submit_ignores_blank_input() {
        let mut input = String::new();
        let commands = process_input_events(
            &mut input,
            [
                InputEvent::Char(' '),
                InputEvent::Char(' '),
                InputEvent::Submit,
            ],
        );
        assert!(commands.is_empty());
        assert!(input.is_empty());
    }

    #[test]
    fn process_input_events_trims_submitted_command() {
        let mut input = String::new();
        let commands = process_input_events(
            &mut input,
            [
                InputEvent::Char(' '),
                InputEvent::Char('f'),
                InputEvent::Char('d'),
                InputEvent::Char(' '),
                InputEvent::Submit,
            ],
        );
        assert_eq!(commands, vec!["fd".to_string()]);
    }

    #[test]
    fn process_browser_commands_filters_blank_entries() {
        let commands = process_browser_commands(vec![
            "fd 100".to_string(),
            "   ".to_string(),
            String::new(),
            "rt 90".to_string(),
        ]);
        assert_eq!(commands, vec!["fd 100".to_string(), "rt 90".to_string()]);
    }

    #[test]
    fn process_browser_commands_keeps_untrimmed_text() {
        let commands = process_browser_commands(vec!["  fd 100  ".to_string()]);
        assert_eq!(commands, vec!["  fd 100  ".to_string()]);
    }

    #[test]
    fn append_log_line_keeps_within_bounds() {
        let mut log = Vec::new();
        for i in 0..11 {
            append_log_line(&mut log, format!("line {i}"), LOG_LINES);
        }
        assert_eq!(log.len(), LOG_LINES * 2);
        assert_eq!(log.first().unwrap(), "line 1");
        assert_eq!(log.last().unwrap(), "line 10");
    }

    #[test]
    fn append_log_line_does_not_trim_below_limit() {
        let mut log = Vec::new();
        append_log_line(&mut log, "hello".to_string(), LOG_LINES);
        append_log_line(&mut log, "world".to_string(), LOG_LINES);
        assert_eq!(log, vec!["hello".to_string(), "world".to_string()]);
    }
}
