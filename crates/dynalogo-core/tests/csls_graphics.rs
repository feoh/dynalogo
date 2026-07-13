use std::fs;
use std::path::{Path, PathBuf};

use dynalogo_core::dynaturtle::{TurtleId, TurtleStore};
use dynalogo_core::turtle::{PenMode, TurtleEvent, TurtleState};
use dynalogo_core::vm::{ControlFlow, Vm};

#[test]
fn csls_geometry_examples_match_expected_turtle_traces() {
    for fixture in fixture_inputs() {
        run_fixture(&fixture);
    }
}

fn fixture_inputs() -> Vec<PathBuf> {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/csls_graphics");
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
    let expected_path = input_path.with_extension("trace");
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
    assert!(
        result.output.is_empty(),
        "fixture {} should be graphics-only, got stdout {:?}",
        input_path.display(),
        result.output
    );

    assert_eq!(
        turtle_trace(vm.turtles()),
        expected,
        "fixture {} turtle trace mismatch",
        input_path.display()
    );
}

fn turtle_trace(turtles: &TurtleStore) -> String {
    let state = turtles
        .state(TurtleId::new(0))
        .expect("default turtle should exist");
    let mut trace = String::new();
    trace.push_str("state\n");
    trace.push_str(&format_state(&state));
    trace.push_str("events\n");
    for event in turtles.events() {
        trace.push_str(&format_event(event));
    }
    trace
}

fn format_state(state: &TurtleState) -> String {
    format!(
        "pos=({}, {}) heading={} pen={} color={} width={} label_height={} visible={}\n",
        format_number(state.position.x),
        format_number(state.position.y),
        format_number(state.heading),
        pen_mode_name(state.pen_mode),
        state.pen_color,
        format_number(state.pen_size),
        format_number(state.label_height),
        state.visible
    )
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

fn format_number(value: f64) -> String {
    if value.abs() < 0.0005 {
        "0.000".to_string()
    } else {
        format!("{value:.3}")
    }
}

fn pen_mode_name(mode: PenMode) -> &'static str {
    match mode {
        PenMode::Up => "up",
        PenMode::Down => "down",
        PenMode::Erase => "erase",
        PenMode::Reverse => "reverse",
    }
}
