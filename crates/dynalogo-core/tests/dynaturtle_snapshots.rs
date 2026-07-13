use std::fs;
use std::path::{Path, PathBuf};

use dynalogo_core::dynaturtle::{TurtleId, TurtleStore};
use dynalogo_core::graphics::SoftwareCanvas;
use dynalogo_core::turtle::{PenMode, Point, TurtleEvent, TurtleState};
use dynalogo_core::vm::{ControlFlow, Vm};

#[test]
fn dynaturtle_visual_audio_snapshots_match_expected() {
    for fixture in fixture_inputs() {
        run_fixture(&fixture);
    }
}

fn fixture_inputs() -> Vec<PathBuf> {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/dynaturtle_snapshots");
    let mut inputs = fs::read_dir(&fixture_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", fixture_dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("lgo")).then_some(path)
        })
        .collect::<Vec<_>>();
    inputs.sort();
    inputs
}

fn run_fixture(input_path: &Path) {
    let expected_path = input_path.with_extension("snapshot");
    let source = fs::read_to_string(input_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
    let expected = fs::read_to_string(&expected_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", expected_path.display()));

    let mut vm = Vm::new();
    let result = vm
        .eval_source(&source)
        .unwrap_or_else(|error| panic!("fixture {} failed: {error}", input_path.display()));
    assert_eq!(
        result.control,
        ControlFlow::None,
        "fixture {} ended with unexpected control flow",
        input_path.display()
    );
    vm.dynaturtle_tick(0.5)
        .unwrap_or_else(|error| panic!("fixture {} tick failed: {error}", input_path.display()));

    assert_eq!(
        snapshot(&vm),
        expected,
        "fixture {} snapshot mismatch",
        input_path.display()
    );
}

fn snapshot(vm: &Vm) -> String {
    let mut out = String::new();
    out.push_str("output\n");
    out.push_str(vm.output());
    out.push_str("visual\n");
    out.push_str(&format!("background={}\n", vm.background_color()));
    out.push_str(&turtle_snapshots(vm.turtles()));
    out.push_str("audio\n");
    out.push_str(&format!("sound_envelope={:?}\n", vm.sound_envelope()));
    out.push_str(&format!("last_toot={:?}\n", vm.last_toot()));
    out.push_str("events\n");
    for event in vm.turtles().events() {
        out.push_str(&format_event(event));
    }
    out.push_str("raster-samples\n");
    out.push_str(&raster_samples(vm.turtles().events()));
    out
}

fn turtle_snapshots(turtles: &TurtleStore) -> String {
    let mut out = String::new();
    for (index, state) in turtles.snapshots().iter().enumerate() {
        let id = TurtleId::new(index);
        let velocity = turtles.velocity(id).unwrap_or(Point::new(0.0, 0.0));
        out.push_str(&format!(
            "turtle {index} pos=({}, {}) heading={} velocity=({}, {}) pen={} color={} width={} shape={} radius={} visible={}\n",
            format_number(state.position.x),
            format_number(state.position.y),
            format_number(state.heading),
            format_number(velocity.x),
            format_number(velocity.y),
            pen_mode_name(state.pen_mode),
            state.pen_color,
            format_number(state.pen_size),
            turtles.shape(id).unwrap_or(""),
            format_number(turtles.collision_radius(id).unwrap_or(0.0)),
            state.visible,
        ));
    }
    out
}

fn raster_samples(events: &[TurtleEvent]) -> String {
    let mut canvas = SoftwareCanvas::new(41, 41);
    canvas.rasterize_events(events);
    [
        Point::new(0.0, 5.0),
        Point::new(-3.0, -3.0),
        Point::new(-2.0, -5.0),
    ]
    .into_iter()
    .map(|point| {
        format!(
            "point=({}, {}) color={}\n",
            format_number(point.x),
            format_number(point.y),
            format_color(canvas.color_at_logo_point(point))
        )
    })
    .collect()
}

fn format_event(event: &TurtleEvent) -> String {
    match event {
        TurtleEvent::Clear => "clear\n".to_string(),
        TurtleEvent::Line {
            from,
            to,
            color,
            width,
            mode,
        } => format!(
            "line from=({}, {}) to=({}, {}) color={} width={} mode={}\n",
            format_number(from.x),
            format_number(from.y),
            format_number(to.x),
            format_number(to.y),
            color,
            format_number(*width),
            pen_mode_name(*mode)
        ),
        TurtleEvent::Label {
            at,
            text,
            color,
            height,
        } => format!(
            "label at=({}, {}) text={:?} color={} height={}\n",
            format_number(at.x),
            format_number(at.y),
            text,
            color,
            format_number(*height)
        ),
        TurtleEvent::Fill { at, color } => format!(
            "fill at=({}, {}) color={}\n",
            format_number(at.x),
            format_number(at.y),
            color
        ),
        TurtleEvent::State(state) => format!("state-event {}", format_state(state)),
    }
}

fn format_state(state: &TurtleState) -> String {
    format!(
        "pos=({}, {}) heading={} pen={} color={} width={} visible={}\n",
        format_number(state.position.x),
        format_number(state.position.y),
        format_number(state.heading),
        pen_mode_name(state.pen_mode),
        state.pen_color,
        format_number(state.pen_size),
        state.visible
    )
}

fn format_number(value: f64) -> String {
    if value.abs() < 0.0005 {
        "0.000".to_string()
    } else {
        format!("{value:.3}")
    }
}

fn format_color(color: Option<u32>) -> String {
    color
        .map(|color| color.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn pen_mode_name(mode: PenMode) -> &'static str {
    match mode {
        PenMode::Up => "up",
        PenMode::Down => "down",
        PenMode::Erase => "erase",
        PenMode::Reverse => "reverse",
    }
}
