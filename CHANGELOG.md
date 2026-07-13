<!-- markdownlint-disable MD024 -->

# Changelog

All notable user-facing changes to DynaLOGO are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/); see
[docs/versioning.md](docs/versioning.md) for details.

## [Unreleased]

### Removed

- Removed the crates.io publishing workflow and related release instructions;
  DynaLOGO releases are distributed as GitHub Release archives unless a concrete
  external Rust-library consumer appears.

## [1.0.0] - 2026-07-13

### Added

- Added remaining Atari/UCBLogo surface primitives `SETBG`, `SETC`, `SETSP`,
  `CT`, `READLINE`, `ERF`, and `CATALOG`, with deterministic VM regression
  coverage.
- Added file-backed `EDIT "program.lgo` behavior that opens a Logo source file
  in the system editor, creates the file if needed, and evaluates the edited
  source when the editor exits. `EDIT` with no input now opens a blank source
  buffer when there is no previous workspace edit session.
- Added `$EDITOR`-driven `EDNS` and `EDSH` coverage for editing visible
  variables and shape definitions through the existing editor flow.
- Added deterministic Computer Science Logo Style application coverage for file
  diff, Solitaire, BASIC compiler, Pattern Matcher/Doctor, Cryptographer's
  Helper, Fourier plotting, and curated Volume 3 language/AI examples.
- Added dynaturtle visual/audio snapshot coverage, including shape rendering,
  motion, collision/audio (`TOOT`) state, and headless transcript validation.

### Changed

- `EDIT`/`ED` now use `$EDITOR`, then `$VISUAL`, with a Windows `notepad`
  fallback; existing workspace contents-list editing remains supported.
- Updated Atari LOGO validation and ROADMAP documentation to distinguish
  implemented portable compatibility from intentionally non-emulated Atari
  hardware/display-list/editor details.
- Expanded generated help/reference documentation and primitive inventory to
  cover the final 1.0 compatibility surface.

### Fixed

- Implemented true software-raster `PX` reverse/XOR compositing so a second
  reverse pass restores the prior background/pixel state.
- Fixed EOF behavior for `READCHAR`, `READLIST`, `READWORD`, and `READLINE` to
  return `[]` consistently in deterministic file/input fixtures.
- Fixed command-position `CATCH` handling so caught values are consumed in
  command contexts while reporter contexts still return the caught value.
- Allowed user-defined procedures to shadow primitive names at call time, which
  is required by several CSLS application examples.

## [0.1.4] - 2026-07-13

### Added

- Added native/window command-line cursor editing with Left/Right, Home/End,
  insertion at the cursor, Backspace before the cursor, and Delete at the
  cursor.
- Added a source-of-truth design and Witan task breakdown for a future
  interactive `HELP` facility.
- Added the canonical `docs/help/` topic schema, source layout, and starter
  topics for future runtime `HELP` / `APROPOS` content.
- Added a help-topic generator that emits `docs/help-reference.md` and embedded
  Rust help data from the canonical topic files, with CI drift checking.
- Added core `HELP`/`HELPON` and `APROPOS` primitives backed by the generated
  embedded help index.
- Expanded source-of-truth help coverage for primitive groups, data types,
  expression syntax, library procedures, and language features; the help
  generator now validates that every implemented primitive has topic coverage.
- Surfaced interactive `HELP` / `APROPOS` hints in terminal and native/window
  frontend onboarding text.

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

[Unreleased]: https://github.com/feoh/dynalogo/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/feoh/dynalogo/compare/v0.1.4...v1.0.0
[0.1.4]: https://github.com/feoh/dynalogo/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/feoh/dynalogo/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/feoh/dynalogo/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/feoh/dynalogo/releases/tag/v0.1.1
[0.1.0]: https://github.com/feoh/dynalogo/releases/tag/v0.1.0
