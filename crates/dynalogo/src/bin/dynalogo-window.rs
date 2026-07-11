use dynalogo_core::turtle::{Point, TurtleEvent, TurtleState};
use dynalogo_core::vm::Vm;
use macroquad::prelude::*;

const PROMPT_HEIGHT: f32 = 92.0;
const LOG_LINES: usize = 5;

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new();

    loop {
        clear_background(Color::from_rgba(18, 20, 26, 255));
        app.handle_input();
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
}

impl App {
    fn new() -> Self {
        Self {
            vm: Vm::new(),
            input: String::new(),
            log: vec![
                "Type Logo commands, then Enter. Example: repeat 4 [fd 100 rt 90]".to_string(),
            ],
        }
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

    fn eval_command(&mut self, command: String) {
        self.log.push(format!("? {command}"));
        match self.vm.eval_source(&command) {
            Ok(result) => {
                for line in result.output.lines() {
                    self.log.push(line.to_string());
                }
                for value in result.stack {
                    self.log.push(value.show(self.vm.interner()));
                }
            }
            Err(error) => self.log.push(format!("Error: {error}")),
        }

        let keep = LOG_LINES * 2;
        if self.log.len() > keep {
            self.log.drain(0..self.log.len() - keep);
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

        for event in self.vm.turtles().events() {
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
                    *width as f32,
                    logo_color(*color),
                );
            }
        }

        for state in self.vm.turtles().snapshots() {
            self.draw_turtle(state, canvas_height);
        }
    }

    fn draw_turtle(&self, state: TurtleState, canvas_height: f32) {
        if !state.visible {
            return;
        }
        let center = logo_to_screen(state.position, canvas_height);
        let heading = state.heading.to_radians() as f32;
        let forward = Vec2::new(heading.sin(), -heading.cos());
        let right = Vec2::new(forward.y, -forward.x);
        let tip = center + forward * 14.0;
        let back = center - forward * 10.0;
        let left = back - right * 8.0;
        let right_point = back + right * 8.0;
        draw_triangle(tip, left, right_point, YELLOW);
        draw_triangle_lines(tip, left, right_point, 1.5, BLACK);
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
