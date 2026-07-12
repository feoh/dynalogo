# Changelog

All notable user-facing changes to DynaLOGO are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/) once it
reaches 1.0. Before 1.0, minor versions (`0.x.0`) may include breaking
changes; see [docs/versioning.md](docs/versioning.md) for details.

## [Unreleased]

### Added

- A `Publish crates` GitHub Actions workflow that publishes `dynalogo-core`
  and `dynalogo` to crates.io on a `vX.Y.Z` tag push. See
  [docs/releasing.md](docs/releasing.md) for the manual tag step and the
  required `CARGO_REGISTRY_TOKEN` secret.

## [0.1.0] - Unreleased

Baseline snapshot of the workspace at the start of changelog tracking. This
entry summarizes the state of the project rather than a tagged release —
see [ROADMAP.md](ROADMAP.md) for the version plan.

### Added

- Core Logo language: lexer, parser, bytecode VM, `TO`/`END`, REPL, and a
  headless test harness.
- Static turtle graphics primitives and a native window frontend.
- Dynaturtle simulation: sim thread, `TELL`/`ASK`/`EACH`/`WHO`,
  `SETVELOCITY`/`SETSPEED`, spatial-hash collision, `WHEN` demons, and
  edge modes (`BOUNCE`/`WRAP`/`FENCE`/`WINDOW`).
- Rich language core: templates, `CATCH`/`THROW`, property lists, arrays,
  and UCBLogo-style error messages.
- Workspace and I/O primitives: `LOAD`/`SAVE`, streams, `EDIT`/`ED`.
- Logo macros (`.MACRO`) and a suite of Atari LOGO compatibility helpers.
- Browser/WASM demo published via GitHub Pages.
- CI workflow running `cargo fmt`, `cargo clippy`, and `cargo test`.

[Unreleased]: https://github.com/feoh/dynalogo/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/feoh/dynalogo/releases/tag/v0.1.0
