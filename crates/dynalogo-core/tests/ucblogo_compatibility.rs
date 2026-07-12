use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use dynalogo_core::vm::{ControlFlow, Vm};

#[derive(Clone, Copy)]
enum FixtureMode {
    MatchUcbLogo,
    DynalogoOnly,
}

struct Fixture {
    stem: &'static str,
    mode: FixtureMode,
    note: &'static str,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        stem: "core_control",
        mode: FixtureMode::MatchUcbLogo,
        note: "core control and expression evaluation",
    },
    Fixture {
        stem: "error_processing",
        mode: FixtureMode::MatchUcbLogo,
        note: "CATCH \"ERROR and ERROR list behavior",
    },
    Fixture {
        stem: "workspace_text",
        mode: FixtureMode::MatchUcbLogo,
        note: "procedure output and TEXT workspace source exposure",
    },
    Fixture {
        stem: "error_codes",
        mode: FixtureMode::MatchUcbLogo,
        note: "CATCH \"ERROR/ERROR list contents for codes 5, 9, 11, 25, and 35",
    },
    Fixture {
        stem: "dynaturtle_selection",
        mode: FixtureMode::DynalogoOnly,
        note: "dynaturtle addressing primitives are an intentional extension beyond UCBLogo",
    },
];

#[test]
fn dynalogo_matches_committed_ucblogo_reference_outputs() {
    let live_ucblogo = find_ucblogo_binary();
    for fixture in FIXTURES {
        if !matches!(fixture.mode, FixtureMode::MatchUcbLogo) {
            continue;
        }
        let source = read_fixture(fixture.stem, "lgo");
        let expected = read_fixture(fixture.stem, "out");
        let actual = run_dynalogo(&source)
            .unwrap_or_else(|error| panic!("dynalogo fixture {} failed: {error}", fixture.stem));
        assert_eq!(
            actual, expected,
            "dynalogo output mismatch for {} ({})",
            fixture.stem, fixture.note
        );

        if let Some(bin) = &live_ucblogo {
            let actual_ucblogo = run_ucblogo(bin, &source).unwrap_or_else(|error| {
                panic!(
                    "ucblogo fixture {} failed via {}: {error}",
                    fixture.stem,
                    bin.display()
                )
            });
            assert_eq!(
                actual_ucblogo, expected,
                "ucblogo output mismatch for {} ({})",
                fixture.stem, fixture.note
            );
        }
    }
}

#[test]
fn dynalogo_only_fixtures_document_intentional_divergences() {
    for fixture in FIXTURES {
        if !matches!(fixture.mode, FixtureMode::DynalogoOnly) {
            continue;
        }
        let source = read_fixture(fixture.stem, "lgo");
        let expected = read_fixture(fixture.stem, "dynalogo.out");
        let actual = run_dynalogo(&source)
            .unwrap_or_else(|error| panic!("dynalogo fixture {} failed: {error}", fixture.stem));
        assert_eq!(
            actual, expected,
            "dynalogo-only fixture mismatch for {} ({})",
            fixture.stem, fixture.note
        );
    }
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ucblogo")
}

fn read_fixture(stem: &str, extension: &str) -> String {
    let path = fixtures_dir().join(format!("{stem}.{extension}"));
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn run_dynalogo(source: &str) -> Result<String, String> {
    let mut vm = Vm::new();
    match vm.eval_source(source) {
        Ok(result) => Ok(render_run_result(&mut vm, result)),
        Err(error) => Err(error.to_string()),
    }
}

fn render_run_result(vm: &mut Vm, result: dynalogo_core::vm::RunResult) -> String {
    let mut rendered = result.output;
    for value in result.stack {
        rendered.push_str(&value.show(vm.interner()));
        rendered.push('\n');
    }
    match result.control {
        ControlFlow::None | ControlFlow::Stop => {}
        ControlFlow::Output(value) => {
            rendered.push_str(&value.show(vm.interner()));
            rendered.push('\n');
        }
        ControlFlow::Continue => {
            rendered.push_str("CONTINUE can only be used inside PAUSE\n");
        }
        ControlFlow::Throw { tag, value } => {
            rendered.push_str("Uncaught THROW ");
            rendered.push_str(&tag.show(vm.interner()));
            rendered.push(' ');
            rendered.push_str(&value.show(vm.interner()));
            rendered.push('\n');
        }
    }
    rendered
}

fn find_ucblogo_binary() -> Option<PathBuf> {
    if let Ok(path) = env::var("UCBLOGO_BIN") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        for name in ["ucblogo", "logo"] {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn run_ucblogo(binary: &Path, source: &str) -> Result<String, String> {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn {}: {error}", binary.display()))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "failed to open stdin for UCBLogo".to_string())?;
        stdin
            .write_all(format!("{source}\nbye\n").as_bytes())
            .map_err(|error| format!("failed to write UCBLogo stdin: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for UCBLogo: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "exit status {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(normalize_ucblogo_output(
        &String::from_utf8_lossy(&output.stdout),
        source,
    ))
}

fn normalize_ucblogo_output(stdout: &str, source: &str) -> String {
    let source_lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<HashSet<_>>();

    let mut normalized = String::new();
    for raw_line in stdout.lines() {
        let line = raw_line.trim_end();
        if line.is_empty()
            || line.starts_with("Berkeley Logo")
            || line.starts_with("Welcome to Berkeley Logo")
            || line.starts_with("Thank you for using Berkeley Logo")
        {
            continue;
        }

        let stripped = line.strip_prefix("? ").unwrap_or(line).trim();
        if stripped.is_empty()
            || stripped.eq_ignore_ascii_case("bye")
            || source_lines.contains(stripped)
        {
            continue;
        }

        normalized.push_str(stripped);
        normalized.push('\n');
    }
    normalized
}
