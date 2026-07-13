<!-- markdownlint-disable MD024 -->

# Changelog

All notable user-facing changes to DynaLOGO are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/) once it
reaches 1.0. Before 1.0, minor versions (`0.x.0`) may include breaking
changes; see [docs/versioning.md](docs/versioning.md) for details.

## [Unreleased]

### Added

- Added native/window command-line cursor editing with Left/Right, Home/End,
  insertion at the cursor, Backspace before the cursor, and Delete at the
  cursor.
- Added a source-of-truth design and Witan task breakdown for a future
  interactive `HELP` facility.

### Fixed

- Fixed native/window text-size controls so command log and system output text
  scale along with the live input line.

## [0.1.3] - 2026-07-13

### Added

- Added native/window command history navigation with Up/Down arrows.
- Added native/window exit aliases (`exit`, `quit`, `bye`) and Ctrl+Q.
- Added native/window input font size controls with Ctrl+Plus/Equals and
  Ctrl+Minus, including hold-repeat behavior.

### Changed

- Disabled automatic GitHub Pages demo deployment while the GitHub-hosted
  release-mode WASM linker failure is investigated.
- Made the crates.io publish workflow skip publish steps cleanly when
  `CARGO_REGISTRY_TOKEN` is not configured.

### Fixed

- Fixed `CLEARSCREEN`/`CS` so it returns turtles home without drawing an
  unintended line from the previous turtle position to the origin.

## [0.1.2] - 2026-07-13

### Added

- Added `graphics_bench` to compare full turtle-trail replay with incremental
  `RasterCache` updates at 100- and 1,000-event scales.
- Documented the performance architecture and validation workflow for trail
  rendering, fixed timestep simulation, native command evaluation, and
  collision benchmarks.

### Changed

- Replaced native window per-frame full turtle-trail raster replay with a
  persistent `RasterCache` and `Texture2D` trail layer that incrementally
  applies newly appended drawing events.
- Advanced native window dynaturtle simulation through `FixedTimestep` using
  measured frame deltas instead of assuming one 1/60-second tick per rendered
  frame.
- Queued native window commands and evaluated them on a background worker so
  text entry and rendering can remain responsive while longer Logo commands run.
- Optimized spatial-hash collision candidate generation to avoid `HashSet`
  deduplication while preserving deterministic candidate ordering.

### Fixed

- Prevented high-refresh or overloaded native windows from changing simulation
  speed by coupling dynaturtle ticks to elapsed time instead of frame count.

## [0.1.1] - 2026-07-12

### Fixed

- Prevented combined graphics/text prompt history lines from overlapping the
  live input prompt in the native window.

## [0.1.0] - 2026-07-12

Initial public release of the workspace. This release packages the current
Logo interpreter, native frontends, browser demo, and compatibility surface.

### Added

- GitHub Actions release packaging for Linux, macOS, and Windows archives.
- A `Publish crates` GitHub Actions workflow that publishes `dynalogo-core`
  and `dynalogo` to crates.io on a `vX.Y.Z` tag push. See
  [docs/releasing.md](docs/releasing.md) for the required
  `CARGO_REGISTRY_TOKEN` secret.

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

[Unreleased]: https://github.com/feoh/dynalogo/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/feoh/dynalogo/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/feoh/dynalogo/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/feoh/dynalogo/releases/tag/v0.1.1
[0.1.0]: https://github.com/feoh/dynalogo/releases/tag/v0.1.0
