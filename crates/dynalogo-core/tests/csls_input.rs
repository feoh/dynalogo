use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

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
    let scratch_dir = unique_scratch_dir(input_path);
    fs::create_dir_all(&scratch_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", scratch_dir.display()));
    let scratch = logo_path(&scratch_dir);
    let source = fs::read_to_string(input_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()))
        .replace("__SCRATCH__", &scratch);
    let script = fs::read_to_string(&script_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", script_path.display()))
        .replace("__SCRATCH__", &scratch);
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
    let _ = fs::remove_dir_all(scratch_dir);
}

fn unique_scratch_dir(input_path: &Path) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("csls-input");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("dynalogo-{stem}-{}-{nanos}", process::id()))
}

fn logo_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
