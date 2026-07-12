use dynalogo_core::dynaturtle::TurtleId;
use dynalogo_core::turtle::{Point, TurtleEvent, TurtleState};
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

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new().await;

    loop {
        clear_background(Color::from_rgba(18, 20, 26, 255));
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
        while let Some(ch) = get_char_pressed() {
            if !ch.is_control() {
                self.input.push(ch);
            }
        }

        if is_key_pressed(KeyCode::Backspace) {
            self.input.pop();
        }

        if is_key_pressed(KeyCode::Enter) {
            let command = self.input.trim().to_string();
            self.input.clear();
            if !command.is_empty() {
                self.eval_command(command);
            }
        }

        if is_key_pressed(KeyCode::Escape) {
            self.input.clear();
        }
    }

    fn handle_browser_commands(&mut self) {
        for command in drain_browser_commands() {
            if !command.trim().is_empty() {
                self.eval_command(command);
            }
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
        draw_rectangle_lines(0.0, 0.0, screen_width(), canvas_height, 2.0, GRAY);
        draw_line(
            screen_width() / 2.0,
            0.0,
            screen_width() / 2.0,
            canvas_height,
            1.0,
            Color::from_rgba(55, 60, 70, 255),
        );
        draw_line(
            0.0,
            canvas_height / 2.0,
            screen_width(),
            canvas_height / 2.0,
            1.0,
            Color::from_rgba(55, 60, 70, 255),
        );

        let events = self.vm.turtles().events();
        let start = events
            .iter()
            .rposition(|event| matches!(event, TurtleEvent::Clear))
            .map_or(0, |index| index + 1);

        for event in &events[start..] {
            if let TurtleEvent::Line {
                from,
                to,
                color,
                width,
            } = event
            {
                let from = logo_to_screen(*from, canvas_height);
                let to = logo_to_screen(*to, canvas_height);
                draw_line(
                    from.x,
                    from.y,
                    to.x,
                    to.y,
                    (*width).max(1.0) as f32,
                    logo_color(*color),
                );
            }
        }

        for (index, state) in self.vm.turtles().snapshots().into_iter().enumerate() {
            self.draw_turtle(TurtleId::new(index), state, canvas_height);
        }

        if self.bark_flash_until > get_time() {
            draw_text("TOOT!", 18.0, 28.0, 30.0, ORANGE);
        }
    }

    fn draw_turtle(&self, id: TurtleId, state: TurtleState, canvas_height: f32) {
        if !state.visible {
            return;
        }

        let center = logo_to_screen(state.position, canvas_height);
        let heading = state.heading.to_radians() as f32;
        let forward = Vec2::new(heading.sin(), -heading.cos());
        let right = Vec2::new(forward.y, -forward.x);
        let shape = self.vm.turtles().shape(id).unwrap_or("turtle");
        let phase = (get_time() as f32 * 8.0).sin();

        if shape.eq_ignore_ascii_case("dog") {
            draw_dog_sprite(center, forward, right, phase);
        } else if shape.eq_ignore_ascii_case("ship") || shape.eq_ignore_ascii_case("rocket") {
            draw_ship_sprite(center, forward, right, phase);
        } else {
            draw_turtle_sprite(center, forward, right, phase, logo_color(state.pen_color));
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
        self.log.push(line);
        let keep = LOG_LINES * 2;
        if self.log.len() > keep {
            self.log.drain(0..self.log.len() - keep);
        }
        sync_browser_log(&self.log);
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

fn logo_to_screen(point: Point, canvas_height: f32) -> Vec2 {
    Vec2::new(
        screen_width() / 2.0 + point.x as f32,
        canvas_height / 2.0 - point.y as f32,
    )
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
