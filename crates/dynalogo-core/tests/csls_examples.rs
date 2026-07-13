use std::fs;
use std::path::{Path, PathBuf};

use dynalogo_core::vm::{ControlFlow, Vm};

#[test]
fn computer_science_logo_style_examples_match_expected_output() {
    for fixture in fixture_inputs() {
        run_fixture(&fixture);
    }
}

fn fixture_inputs() -> Vec<PathBuf> {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/csls");
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
    let stdout_path = input_path.with_extension("out");
    let error_path = input_path.with_extension("err");
    let source = fs::read_to_string(input_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", input_path.display()));

    let mut vm = Vm::new();
    match (stdout_path.exists(), error_path.exists()) {
        (true, false) => {
            let expected = fs::read_to_string(&stdout_path).unwrap_or_else(|error| {
                panic!("failed to read {}: {error}", stdout_path.display())
            });
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
        (false, true) => {
            let expected = fs::read_to_string(&error_path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", error_path.display()));
            let error = match vm.eval_source(&source) {
                Ok(result) => panic!(
                    "fixture {} unexpectedly succeeded with output {:?}",
                    input_path.display(),
                    result.output
                ),
                Err(error) => error,
            };
            assert_eq!(
                error.message,
                expected.trim_end(),
                "fixture {} error mismatch",
                input_path.display()
            );
        }
        (true, true) => panic!(
            "fixture {} cannot define both .out and .err expectations",
            input_path.display()
        ),
        (false, false) => panic!(
            "fixture {} must define either a .out or .err expectation",
            input_path.display()
        ),
    }
}
