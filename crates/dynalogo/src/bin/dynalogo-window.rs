use std::collections::VecDeque;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
use std::time::Duration;

use dynalogo_core::dynaturtle::TurtleId;
use dynalogo_core::graphics::{scale_event, scale_point, RasterCache};
use dynalogo_core::sim::{FixedTimestep, SimConfig};
use dynalogo_core::turtle::{events_since_clear, Point, TurtleEvent, TurtleState};
use dynalogo_core::value::{List, Value};
use dynalogo_core::vm::{ControlFlow, RunResult, Vm, VmError};
use macroquad::audio::{load_sound_from_bytes, play_sound_once, Sound};
use macroquad::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlTextAreaElement;

const PROMPT_HEIGHT: f32 = 156.0;
const LOG_LINES: usize = 5;
const LOG_FONT_SIZE: f32 = 18.0;
const DEFAULT_INPUT_FONT_SIZE: f32 = 22.0;
const MIN_INPUT_FONT_SIZE: f32 = 14.0;
const MAX_INPUT_FONT_SIZE: f32 = 42.0;
const INPUT_FONT_SIZE_STEP: f32 = 2.0;
const INPUT_FONT_REPEAT_DELAY: f64 = 0.35;
const INPUT_FONT_REPEAT_INTERVAL: f64 = 0.06;
const LOG_LINE_HEIGHT: f32 = 20.0;
const PROMPT_TOP_PADDING: f32 = 12.0;
const INPUT_BASELINE_OFFSET: f32 = 14.0;
const LOG_INPUT_GAP: f32 = 8.0;
const CANVAS_BACKGROUND: Color = Color::new(18.0 / 255.0, 20.0 / 255.0, 26.0 / 255.0, 1.0);

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new().await;

    loop {
        clear_background(CANVAS_BACKGROUND);
        app.handle_input();
        app.handle_browser_commands();
        app.poll_eval();
        app.update_sim();
        app.update_audio();
        app.draw();
        if app.should_quit {
            macroquad::miniquad::window::quit();
            break;
        }
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
    input: InputState,
    log: Vec<String>,
    bark_sound: Option<Sound>,
    last_toot: Option<[u8; 4]>,
    bark_flash_until: f64,
    timestep: FixedTimestep,
    trail_texture: TrailTexture,
    command_queue: VecDeque<String>,
    eval_worker: EvalWorker,
    should_quit: bool,
    input_font_size: f32,
    input_font_repeat: FontSizeRepeat,
}

impl App {
    async fn new() -> Self {
        let app = Self {
            vm: Vm::new(),
            input: InputState::default(),
            log: vec![
                "Type Logo commands, then Enter. Try: tell [0 1 2] each [setshape \"dog 12]"
                    .to_string(),
            ],
            bark_sound: load_sound_from_bytes(&generate_bark_wav()).await.ok(),
            last_toot: None,
            bark_flash_until: 0.0,
            timestep: FixedTimestep::new(SimConfig::default()),
            trail_texture: TrailTexture::new(1, 1),
            command_queue: VecDeque::new(),
            eval_worker: EvalWorker::idle(),
            should_quit: false,
            input_font_size: DEFAULT_INPUT_FONT_SIZE,
            input_font_repeat: FontSizeRepeat::default(),
        };
        sync_browser_log(&app.log);
        app
    }

    fn handle_input(&mut self) {
        let mut events = Vec::new();
        let ctrl_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let mut font_steps = 0;
        while let Some(ch) = get_char_pressed() {
            if ctrl_down {
                font_steps += input_font_char_step(ch);
            } else if !ch.is_control() {
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
        if is_key_pressed(KeyCode::Up) {
            events.push(InputEvent::HistoryPrevious);
        }
        if is_key_pressed(KeyCode::Down) {
            events.push(InputEvent::HistoryNext);
        }
        if is_key_pressed(KeyCode::Left) {
            events.push(InputEvent::CursorLeft);
        }
        if is_key_pressed(KeyCode::Right) {
            events.push(InputEvent::CursorRight);
        }
        if is_key_pressed(KeyCode::Delete) {
            events.push(InputEvent::Delete);
        }
        if is_key_pressed(KeyCode::Home) {
            events.push(InputEvent::CursorHome);
        }
        if is_key_pressed(KeyCode::End) {
            events.push(InputEvent::CursorEnd);
        }
        if ctrl_down && is_key_pressed(KeyCode::Q) {
            self.request_quit();
        }

        if font_steps == 0 {
            font_steps += self.input_font_repeat.steps_for(
                input_font_key_direction(ctrl_down),
                input_font_key_pressed(ctrl_down),
                get_time(),
            );
        } else {
            self.input_font_repeat.reset();
        }
        if font_steps != 0 {
            self.adjust_input_font_size(font_steps as f32 * INPUT_FONT_SIZE_STEP);
        }

        for command in process_input_state_events(&mut self.input, events) {
            self.handle_command(command);
        }
    }

    fn handle_browser_commands(&mut self) {
        for command in process_browser_commands(drain_browser_commands()) {
            self.handle_command(command);
        }
    }

    fn handle_command(&mut self, command: String) {
        if is_exit_command(&command) {
            self.push_log(format!("? {command}"));
            self.request_quit();
        } else {
            self.enqueue_command(command);
        }
    }

    fn enqueue_command(&mut self, command: String) {
        self.push_log(format!("? {command}"));
        self.command_queue.push_back(command);
        self.start_next_eval();
    }

    fn request_quit(&mut self) {
        self.should_quit = true;
    }

    fn adjust_input_font_size(&mut self, delta: f32) {
        self.input_font_size = adjust_input_font_size(self.input_font_size, delta);
    }

    fn poll_eval(&mut self) {
        if let Some(outcome) = self.eval_worker.try_recv() {
            self.vm = outcome.vm;
            self.handle_eval_result(outcome.result);
            self.start_next_eval();
        }
    }

    fn start_next_eval(&mut self) {
        if self.eval_worker.is_running() {
            return;
        }
        let Some(command) = self.command_queue.pop_front() else {
            return;
        };
        self.eval_worker.start(self.vm.clone(), command);
    }

    fn handle_eval_result(&mut self, result: Result<RunResult, VmError>) {
        match result {
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
        if self.eval_worker.is_running() {
            return;
        }

        let elapsed = Duration::from_secs_f32(get_frame_time().max(0.0));
        let mut timestep = std::mem::take(&mut self.timestep);
        let tick_seconds = timestep.config().tick.as_secs_f64();
        timestep.advance(elapsed, |_| self.run_sim_tick(tick_seconds));
        self.timestep = timestep;
    }

    fn run_sim_tick(&mut self, tick_seconds: f64) {
        match self.vm.dynaturtle_tick(tick_seconds) {
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

    fn draw(&mut self) {
        self.draw_canvas();
        self.draw_prompt();
    }

    fn draw_canvas(&mut self) {
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
        let events = self.vm.turtles().events();
        self.trail_texture.draw(
            events,
            screen_w as usize,
            canvas_height.max(1.0) as usize,
            x_scrunch,
            y_scrunch,
        );

        for event in events_since_clear(events)
            .iter()
            .map(|event| scale_event(event, x_scrunch, y_scrunch))
        {
            if let TurtleEvent::Label {
                at,
                text,
                color,
                height,
            } = event
            {
                let at = logo_to_screen(at, screen_w, canvas_height);
                draw_text(&text, at.x, at.y, height.max(1.0) as f32, logo_color(color));
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
        let layout = prompt_layout(
            screen_height(),
            PROMPT_HEIGHT,
            LOG_LINES,
            self.input_font_size,
        );
        draw_rectangle(
            0.0,
            layout.top,
            screen_width(),
            PROMPT_HEIGHT,
            Color::from_rgba(8, 10, 14, 255),
        );
        draw_line(0.0, layout.top, screen_width(), layout.top, 2.0, DARKGRAY);

        let start = self.log.len().saturating_sub(layout.log_lines);
        for (index, line) in self.log[start..].iter().enumerate() {
            let y = layout.log_start_y + index as f32 * layout.log_line_height;
            draw_text(line, 12.0, y, LOG_FONT_SIZE, LIGHTGRAY);
        }

        let input_x = 12.0;
        let prompt_text = format!("? {}", self.input.line());
        draw_text(
            &prompt_text,
            input_x,
            layout.input_baseline,
            self.input_font_size,
            WHITE,
        );
        if ((get_time() * 2.0) as i64) % 2 == 0 {
            let prefix = format!("? {}", self.input.cursor_prefix());
            let metrics = measure_text(&prefix, None, self.input_font_size.round() as u16, 1.0);
            let cursor_x = input_x + metrics.width;
            draw_line(
                cursor_x,
                layout.input_baseline - self.input_font_size,
                cursor_x,
                layout.input_baseline + 4.0,
                1.5,
                WHITE,
            );
        }
    }

    fn push_log(&mut self, line: String) {
        append_log_line(&mut self.log, line, LOG_LINES);
        sync_browser_log(&self.log);
    }
}

struct EvalOutcome {
    vm: Vm,
    result: Result<RunResult, VmError>,
}

#[cfg(not(target_arch = "wasm32"))]
enum EvalWorker {
    Idle,
    Running(Receiver<EvalOutcome>),
}

#[cfg(not(target_arch = "wasm32"))]
impl EvalWorker {
    fn idle() -> Self {
        Self::Idle
    }

    fn is_running(&self) -> bool {
        matches!(self, Self::Running(_))
    }

    fn start(&mut self, mut vm: Vm, command: String) {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = vm.eval_source(&command);
            let _ = tx.send(EvalOutcome { vm, result });
        });
        *self = Self::Running(rx);
    }

    fn try_recv(&mut self) -> Option<EvalOutcome> {
        let Self::Running(rx) = self else {
            return None;
        };
        match rx.try_recv() {
            Ok(outcome) => {
                *self = Self::Idle;
                Some(outcome)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                *self = Self::Idle;
                None
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
struct EvalWorker {
    completed: Option<EvalOutcome>,
}

#[cfg(target_arch = "wasm32")]
impl EvalWorker {
    fn idle() -> Self {
        Self { completed: None }
    }

    fn is_running(&self) -> bool {
        self.completed.is_some()
    }

    fn start(&mut self, mut vm: Vm, command: String) {
        let result = vm.eval_source(&command);
        self.completed = Some(EvalOutcome { vm, result });
    }

    fn try_recv(&mut self) -> Option<EvalOutcome> {
        self.completed.take()
    }
}

struct TrailTexture {
    cache: RasterCache,
    image: Image,
    texture: Option<Texture2D>,
}

impl TrailTexture {
    fn new(width: usize, height: usize) -> Self {
        let (width, height) = texture_dimensions(width, height);
        Self {
            cache: RasterCache::new(width as usize, height as usize),
            image: blank_image(width, height),
            texture: None,
        }
    }

    fn draw(
        &mut self,
        events: &[TurtleEvent],
        width: usize,
        height: usize,
        x_scrunch: f64,
        y_scrunch: f64,
    ) {
        let resized = self.resize_if_needed(width, height);
        let dirty = self.cache.update(events, x_scrunch, y_scrunch);
        if resized || dirty || self.texture.is_none() {
            self.cache.write_rgba_bytes(&mut self.image.bytes);
            match &self.texture {
                Some(texture) => texture.update(&self.image),
                None => {
                    let texture = Texture2D::from_image(&self.image);
                    texture.set_filter(FilterMode::Nearest);
                    self.texture = Some(texture);
                }
            }
        }

        if let Some(texture) = &self.texture {
            draw_texture(texture, 0.0, 0.0, WHITE);
        }
    }

    fn resize_if_needed(&mut self, width: usize, height: usize) -> bool {
        let (width, height) = texture_dimensions(width, height);
        if self.cache.width() == width as usize && self.cache.height() == height as usize {
            return false;
        }
        self.cache = RasterCache::new(width as usize, height as usize);
        self.image = blank_image(width, height);
        self.texture = None;
        true
    }
}

fn texture_dimensions(width: usize, height: usize) -> (u16, u16) {
    let width = width.clamp(1, u16::MAX as usize) as u16;
    let height = height.clamp(1, u16::MAX as usize) as u16;
    (width, height)
}

fn blank_image(width: u16, height: u16) -> Image {
    Image {
        bytes: vec![0; width as usize * height as usize * 4],
        width,
        height,
    }
}

enum InputEvent {
    Char(char),
    Backspace,
    Delete,
    Submit,
    Cancel,
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,
    HistoryPrevious,
    HistoryNext,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct InputState {
    line: String,
    cursor: usize,
    history: Vec<String>,
    history_cursor: Option<usize>,
    history_draft: String,
}

impl InputState {
    fn line(&self) -> &str {
        &self.line
    }

    #[cfg(test)]
    fn cursor(&self) -> usize {
        self.cursor
    }

    fn cursor_prefix(&self) -> &str {
        &self.line[..self.cursor]
    }

    fn set_line(&mut self, line: String) {
        self.line = line;
        self.cursor = self.line.len();
    }

    fn exit_history_edit(&mut self) {
        self.history_cursor = None;
        self.history_draft.clear();
    }

    fn remember_command(&mut self, command: &str) {
        if self.history.last().is_none_or(|last| last != command) {
            self.history.push(command.to_string());
        }
    }

    fn insert_char(&mut self, ch: char) {
        self.line.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let previous = previous_char_boundary(&self.line, self.cursor);
        self.line.drain(previous..self.cursor);
        self.cursor = previous;
    }

    fn delete(&mut self) {
        if self.cursor == self.line.len() {
            return;
        }
        let next = next_char_boundary(&self.line, self.cursor);
        self.line.drain(self.cursor..next);
    }

    fn cursor_left(&mut self) {
        self.cursor = previous_char_boundary(&self.line, self.cursor);
    }

    fn cursor_right(&mut self) {
        self.cursor = next_char_boundary(&self.line, self.cursor);
    }

    fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    fn cursor_end(&mut self) {
        self.cursor = self.line.len();
    }

    fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let index = match self.history_cursor {
            Some(index) => index.saturating_sub(1),
            None => {
                self.history_draft = self.line.clone();
                self.history.len() - 1
            }
        };
        self.history_cursor = Some(index);
        self.set_line(self.history[index].clone());
    }

    fn history_next(&mut self) {
        let Some(index) = self.history_cursor else {
            return;
        };
        if index + 1 < self.history.len() {
            let next = index + 1;
            self.history_cursor = Some(next);
            self.set_line(self.history[next].clone());
        } else {
            self.history_cursor = None;
            let draft = std::mem::take(&mut self.history_draft);
            self.set_line(draft);
        }
    }
}

fn previous_char_boundary(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .char_indices()
        .last()
        .map_or(0, |(index, _)| index)
}

fn next_char_boundary(value: &str, cursor: usize) -> usize {
    value[cursor..]
        .char_indices()
        .nth(1)
        .map_or(value.len(), |(index, _)| cursor + index)
}

#[cfg(test)]
fn process_input_events(
    input: &mut String,
    events: impl IntoIterator<Item = InputEvent>,
) -> Vec<String> {
    let line = std::mem::take(input);
    let mut state = InputState {
        cursor: line.len(),
        line,
        ..InputState::default()
    };
    let commands = process_input_state_events(&mut state, events);
    *input = state.line;
    commands
}

fn process_input_state_events(
    input: &mut InputState,
    events: impl IntoIterator<Item = InputEvent>,
) -> Vec<String> {
    let mut commands = Vec::new();
    for event in events {
        match event {
            InputEvent::Char(ch) => {
                input.exit_history_edit();
                input.insert_char(ch);
            }
            InputEvent::Backspace => {
                input.exit_history_edit();
                input.backspace();
            }
            InputEvent::Delete => {
                input.exit_history_edit();
                input.delete();
            }
            InputEvent::Submit => {
                let command = input.line.trim().to_string();
                input.line.clear();
                input.cursor = 0;
                input.exit_history_edit();
                if !command.is_empty() {
                    input.remember_command(&command);
                    commands.push(command);
                }
            }
            InputEvent::Cancel => {
                input.line.clear();
                input.cursor = 0;
                input.exit_history_edit();
            }
            InputEvent::CursorLeft => input.cursor_left(),
            InputEvent::CursorRight => input.cursor_right(),
            InputEvent::CursorHome => input.cursor_home(),
            InputEvent::CursorEnd => input.cursor_end(),
            InputEvent::HistoryPrevious => input.history_previous(),
            InputEvent::HistoryNext => input.history_next(),
        }
    }
    commands
}

fn is_exit_command(command: &str) -> bool {
    matches!(
        command.trim().to_ascii_lowercase().as_str(),
        "exit" | "quit" | "bye"
    )
}

fn process_browser_commands(commands: Vec<String>) -> Vec<String> {
    commands
        .into_iter()
        .filter(|command| !command.trim().is_empty())
        .collect()
}

struct PromptLayout {
    top: f32,
    log_start_y: f32,
    log_line_height: f32,
    log_lines: usize,
    input_baseline: f32,
}

fn prompt_layout(
    screen_height: f32,
    prompt_height: f32,
    requested_log_lines: usize,
    input_font_size: f32,
) -> PromptLayout {
    let top = (screen_height - prompt_height).max(0.0);
    let input_baseline = (screen_height - INPUT_BASELINE_OFFSET).max(0.0);
    let log_start_y = top + PROMPT_TOP_PADDING + LOG_FONT_SIZE;
    let last_log_baseline = input_baseline - input_font_size - LOG_INPUT_GAP;
    let available_height = (last_log_baseline - log_start_y).max(0.0);
    let available_lines = if log_start_y <= last_log_baseline {
        (available_height / LOG_LINE_HEIGHT).floor() as usize + 1
    } else {
        0
    };

    PromptLayout {
        top,
        log_start_y,
        log_line_height: LOG_LINE_HEIGHT,
        log_lines: requested_log_lines.min(available_lines),
        input_baseline,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct FontSizeRepeat {
    direction: i32,
    next_at: f64,
}

impl FontSizeRepeat {
    fn reset(&mut self) {
        self.direction = 0;
        self.next_at = 0.0;
    }

    fn steps_for(&mut self, direction: i32, pressed_now: bool, now: f64) -> i32 {
        if direction == 0 {
            self.reset();
            return 0;
        }
        if pressed_now || self.direction != direction {
            self.direction = direction;
            self.next_at = now + INPUT_FONT_REPEAT_DELAY;
            return direction;
        }
        if now >= self.next_at {
            self.next_at = now + INPUT_FONT_REPEAT_INTERVAL;
            return direction;
        }
        0
    }
}

fn input_font_key_direction(ctrl_down: bool) -> i32 {
    if !ctrl_down {
        return 0;
    }
    let increase = is_key_down(KeyCode::Equal) || is_key_down(KeyCode::KpAdd);
    let decrease = is_key_down(KeyCode::Minus) || is_key_down(KeyCode::KpSubtract);
    match (increase, decrease) {
        (true, false) => 1,
        (false, true) => -1,
        _ => 0,
    }
}

fn input_font_key_pressed(ctrl_down: bool) -> bool {
    ctrl_down
        && (is_key_pressed(KeyCode::Equal)
            || is_key_pressed(KeyCode::KpAdd)
            || is_key_pressed(KeyCode::Minus)
            || is_key_pressed(KeyCode::KpSubtract))
}

fn input_font_char_step(ch: char) -> i32 {
    match ch {
        '+' | '=' => 1,
        '-' | '_' => -1,
        _ => 0,
    }
}

fn adjust_input_font_size(current: f32, delta: f32) -> f32 {
    (current + delta).clamp(MIN_INPUT_FONT_SIZE, MAX_INPUT_FONT_SIZE)
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
        let Some(points) = custom_shape_points(&value) else {
            panic!("valid custom shape points");
        };
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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn eval_worker_returns_updated_vm_and_result() {
        let mut worker = EvalWorker::idle();
        worker.start(Vm::new(), "make \"x 42 print :x".to_string());
        assert!(worker.is_running());

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let outcome = loop {
            if let Some(outcome) = worker.try_recv() {
                break outcome;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "eval worker timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        };

        let result = match outcome.result {
            Ok(result) => result,
            Err(error) => panic!("worker eval should succeed: {error}"),
        };
        assert_eq!(result.output, "42\n");
        assert!(!worker.is_running());
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
    fn cursor_left_allows_insertion_inside_command() {
        let mut input = String::new();
        let commands = process_input_events(
            &mut input,
            [
                InputEvent::Char('f'),
                InputEvent::Char('d'),
                InputEvent::Char(' '),
                InputEvent::Char('1'),
                InputEvent::Char('0'),
                InputEvent::CursorLeft,
                InputEvent::Char('0'),
                InputEvent::Submit,
            ],
        );
        assert_eq!(commands, vec!["fd 100".to_string()]);
    }

    #[test]
    fn backspace_removes_character_before_cursor() {
        let mut input = String::new();
        process_input_events(
            &mut input,
            [
                InputEvent::Char('f'),
                InputEvent::Char('x'),
                InputEvent::Char('d'),
                InputEvent::CursorLeft,
                InputEvent::Backspace,
            ],
        );
        assert_eq!(input, "fd");
    }

    #[test]
    fn delete_removes_character_at_cursor() {
        let mut input = String::new();
        process_input_events(
            &mut input,
            [
                InputEvent::Char('f'),
                InputEvent::Char('x'),
                InputEvent::Char('d'),
                InputEvent::CursorHome,
                InputEvent::CursorRight,
                InputEvent::Delete,
            ],
        );
        assert_eq!(input, "fd");
    }

    #[test]
    fn home_and_end_move_to_command_boundaries() {
        let mut input = String::new();
        process_input_events(
            &mut input,
            [
                InputEvent::Char('d'),
                InputEvent::Char(' '),
                InputEvent::CursorHome,
                InputEvent::Char('f'),
                InputEvent::CursorEnd,
                InputEvent::Char('1'),
            ],
        );
        assert_eq!(input, "fd 1");
    }

    #[test]
    fn cursor_editing_respects_utf8_boundaries() {
        let mut input = InputState::default();
        process_input_state_events(
            &mut input,
            [
                InputEvent::Char('λ'),
                InputEvent::Char('x'),
                InputEvent::CursorLeft,
                InputEvent::Backspace,
            ],
        );
        assert_eq!(input.line(), "x");
        assert_eq!(input.cursor(), 0);
        process_input_state_events(&mut input, [InputEvent::Delete]);
        assert!(input.line().is_empty());
        assert_eq!(input.cursor(), 0);
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
    fn input_history_steps_back_and_forward_through_commands() {
        let mut input = InputState::default();
        assert_eq!(
            process_input_state_events(
                &mut input,
                [
                    InputEvent::Char('f'),
                    InputEvent::Char('d'),
                    InputEvent::Char(' '),
                    InputEvent::Char('1'),
                    InputEvent::Submit,
                    InputEvent::Char('r'),
                    InputEvent::Char('t'),
                    InputEvent::Char(' '),
                    InputEvent::Char('9'),
                    InputEvent::Char('0'),
                    InputEvent::Submit,
                ],
            ),
            vec!["fd 1".to_string(), "rt 90".to_string()]
        );

        process_input_state_events(&mut input, [InputEvent::HistoryPrevious]);
        assert_eq!(input.line(), "rt 90");
        process_input_state_events(&mut input, [InputEvent::HistoryPrevious]);
        assert_eq!(input.line(), "fd 1");
        process_input_state_events(&mut input, [InputEvent::HistoryNext]);
        assert_eq!(input.line(), "rt 90");
    }

    #[test]
    fn input_history_restores_in_progress_draft() {
        let mut input = InputState::default();
        process_input_state_events(
            &mut input,
            [
                InputEvent::Char('f'),
                InputEvent::Char('d'),
                InputEvent::Char(' '),
                InputEvent::Char('1'),
                InputEvent::Submit,
                InputEvent::Char('p'),
                InputEvent::Char('r'),
            ],
        );

        process_input_state_events(&mut input, [InputEvent::HistoryPrevious]);
        assert_eq!(input.line(), "fd 1");
        assert_eq!(input.cursor(), input.line().len());
        process_input_state_events(&mut input, [InputEvent::HistoryNext]);
        assert_eq!(input.line(), "pr");
        assert_eq!(input.cursor(), input.line().len());
    }

    #[test]
    fn input_history_editing_selected_command_exits_history_mode() {
        let mut input = InputState::default();
        process_input_state_events(
            &mut input,
            [
                InputEvent::Char('f'),
                InputEvent::Char('d'),
                InputEvent::Submit,
                InputEvent::HistoryPrevious,
                InputEvent::Char(' '),
                InputEvent::Char('5'),
            ],
        );

        assert_eq!(input.line(), "fd 5");
        process_input_state_events(&mut input, [InputEvent::HistoryNext]);
        assert_eq!(input.line(), "fd 5");
    }

    #[test]
    fn exit_command_recognizes_window_exit_aliases() {
        assert!(is_exit_command("exit"));
        assert!(is_exit_command(" QUIT "));
        assert!(is_exit_command("Bye"));
        assert!(!is_exit_command("print \"bye"));
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
    fn prompt_layout_keeps_five_history_lines_above_input() {
        let layout = prompt_layout(768.0, PROMPT_HEIGHT, LOG_LINES, DEFAULT_INPUT_FONT_SIZE);
        assert_eq!(layout.log_lines, LOG_LINES);
        assert!(layout.log_line_height >= LOG_FONT_SIZE);

        let last_log_baseline =
            layout.log_start_y + (layout.log_lines - 1) as f32 * layout.log_line_height;
        assert!(
            last_log_baseline + DEFAULT_INPUT_FONT_SIZE + LOG_INPUT_GAP <= layout.input_baseline
        );
    }

    #[test]
    fn prompt_layout_reduces_history_capacity_for_compact_windows() {
        let layout = prompt_layout(120.0, PROMPT_HEIGHT, LOG_LINES, DEFAULT_INPUT_FONT_SIZE);
        assert!(layout.log_lines < LOG_LINES);
        assert!(layout.top >= 0.0);
        assert!(layout.input_baseline >= layout.top);
    }

    #[test]
    fn prompt_layout_accounts_for_larger_input_font() {
        let normal = prompt_layout(768.0, PROMPT_HEIGHT, LOG_LINES, DEFAULT_INPUT_FONT_SIZE);
        let large = prompt_layout(768.0, PROMPT_HEIGHT, LOG_LINES, MAX_INPUT_FONT_SIZE);
        assert!(large.log_lines <= normal.log_lines);

        let last_log_baseline =
            large.log_start_y + large.log_lines.saturating_sub(1) as f32 * large.log_line_height;
        assert!(last_log_baseline + MAX_INPUT_FONT_SIZE + LOG_INPUT_GAP <= large.input_baseline);
    }

    #[test]
    fn adjust_input_font_size_clamps_to_supported_range() {
        assert_eq!(
            adjust_input_font_size(DEFAULT_INPUT_FONT_SIZE, INPUT_FONT_SIZE_STEP),
            DEFAULT_INPUT_FONT_SIZE + INPUT_FONT_SIZE_STEP
        );
        assert_eq!(
            adjust_input_font_size(MAX_INPUT_FONT_SIZE, INPUT_FONT_SIZE_STEP),
            MAX_INPUT_FONT_SIZE
        );
        assert_eq!(
            adjust_input_font_size(MIN_INPUT_FONT_SIZE, -INPUT_FONT_SIZE_STEP),
            MIN_INPUT_FONT_SIZE
        );
    }

    #[test]
    fn input_font_char_step_accepts_plus_equals_and_minus_variants() {
        assert_eq!(input_font_char_step('+'), 1);
        assert_eq!(input_font_char_step('='), 1);
        assert_eq!(input_font_char_step('-'), -1);
        assert_eq!(input_font_char_step('_'), -1);
        assert_eq!(input_font_char_step('x'), 0);
    }

    #[test]
    fn font_size_repeat_emits_initial_and_repeated_steps() {
        let mut repeat = FontSizeRepeat::default();
        assert_eq!(repeat.steps_for(1, true, 10.0), 1);
        assert_eq!(repeat.steps_for(1, false, 10.1), 0);
        assert_eq!(repeat.steps_for(1, false, 10.35), 1);
        assert_eq!(repeat.steps_for(1, false, 10.36), 0);
        assert_eq!(repeat.steps_for(-1, false, 10.37), -1);
        assert_eq!(repeat.steps_for(0, false, 10.38), 0);
        assert_eq!(repeat, FontSizeRepeat::default());
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
