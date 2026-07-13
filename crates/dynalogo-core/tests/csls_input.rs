use std::fs;
use std::path::{Path, PathBuf};

use dynalogo_core::vm::{ControlFlow, Vm};

#[test]
fn csls_scripted_input_examples_match_expected_output() {
    for fixture in fixture_inputs() {
        run_fixture(&fixture);
    }
}

fn fixture_inputs() -> Vec<PathBuf> {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/csls_input");
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
    let script_path = input_path.with_extension("in");
    let stdout_path = input_path.with_extension("out");
    let source = fs::read_to_string(input_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));
    let script = fs::read_to_string(&script_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", script_path.display()));
    let expected = fs::read_to_string(&stdout_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", stdout_path.display()));

    let mut vm = Vm::new();
    vm.set_scripted_input(script_path.clone(), script);
    let result = vm
        .eval_source(&source)
        .unwrap_or_else(|error| panic!("fixture {} failed: {error}", input_path.display()));
    assert_eq!(
        result.control,
        ControlFlow::None,
        "fixture {} ended with unexpected control flow",
        input_path.display()
    );
    assert_eq!(
        result.output,
        expected,
        "fixture {} output mismatch",
        input_path.display()
    );
}
