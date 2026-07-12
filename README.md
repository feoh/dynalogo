# DynaLOGO

A new implementation of the LOGO programming language, written in Rust.

DynaLOGO's syntax closely follows
[UCBLogo](https://people.eecs.berkeley.edu/~bh/logo), extended with
**dynaturtles** — turtles that have velocity as well as position, with
real-time collision detection and event "demons" — as described in Seymour
Papert's _Mindstorms_ and realized in Atari LOGO and MicroWorlds LOGO.

```logo
TO SPACESHIP
  TELL 0
  SETSHAPE "ship
  SETSPEED 20
  WHEN TOUCHING [0 1] [EXPLODE]
  WHEN EDGE [BOUNCE]
END
```

## Goals

- **UCBLogo compatibility** — the full rich syntax and library (~400
  primitives) is the v1.0 target, including dynamic scope, templates,
  macros, property lists, and UCBLogo-accurate error messages.
- **Dynaturtles** — velocity-bearing turtles, `SETSPEED`/`SETVELOCITY`,
  `WHEN TOUCHING`/`WHEN EDGE` collision demons, `BOUNCE`/`WRAP`/`FENCE`
  edge modes.
- **Performance** — bytecode VM (not a tree-walker), fixed 60 Hz simulation
  tick decoupled from rendering, struct-of-arrays turtle storage,
  spatial-hash collision. Target: 1,000 colliding dynaturtles at 60 Hz.
- **Live** — the REPL and the simulation run concurrently; typing never
  freezes moving turtles.

## Workspace layout

- `crates/dynalogo-core` — lexer, parser, bytecode compiler, VM, values, the
  dynaturtle sim engine, and both native/cooperative runtime utilities
- `crates/dynalogo` — native frontend: window, turtle rendering, and REPL

`dynalogo-core` is headless and has no graphics dependencies.

## Documentation

- [`docs/getting-started.md`](docs/getting-started.md) — the best place to
  begin if you are new to DynaLOGO
- [`docs/browser-demo.md`](docs/browser-demo.md) — running the WASM browser
  demo locally and how it differs from the native window
- [`docs/reference-manual.md`](docs/reference-manual.md) — user reference for
  the currently implemented language and dynaturtle surface
- [`docs/debugging-and-errors.md`](docs/debugging-and-errors.md) — practical
  debugging and error-handling guidance
- [`docs/developer-guide.md`](docs/developer-guide.md) — contributor-oriented
  internals and extension points
- [`docs/wasm-and-browser.md`](docs/wasm-and-browser.md) — WASM/browser build
  and embedding guidance
- [`docs/ucblogo-compatibility.md`](docs/ucblogo-compatibility.md) —
  compatibility corpus, live-UCBLogo harness notes, and intentional
  divergences
- [`docs/README.md`](docs/README.md) — documentation index
- [`docs/versioning.md`](docs/versioning.md) — versioning policy and
  changelog process
- [`docs/releasing.md`](docs/releasing.md) — how crates.io releases are cut

## Running DynaLOGO

### Terminal REPL

```bash
cargo run -p dynalogo --bin dynalogo
```

Useful commands:

- `cargo run -p dynalogo --bin dynalogo -- --eval 'print sum 2 3'`
- `cargo run -p dynalogo --bin dynalogo < examples/square.lgo`

### Native turtle window

```bash
cargo run -p dynalogo --bin dynalogo-window
```

The window frontend keeps a small command log at the bottom and renders turtle
lines on a centered Cartesian canvas.

### Browser demo (WASM)

DynaLOGO now includes a browser-oriented macroquad build of
`dynalogo-window`, plus a GitHub Pages workflow that publishes a small in-page
REPL demo.

You can validate the browser build locally with:

```bash
rustup target add wasm32-unknown-unknown
cargo build -p dynalogo --bin dynalogo-window --target wasm32-unknown-unknown
```

The committed demo shell lives in `web/index.html`, and the Pages workflow
copies that shell, the compiled `dynalogo-window.wasm`, and the example `.lgo`
files into the published site artifact. The shell includes a grouped example
gallery dropdown covering every program in `examples/`, plus a Starter
snippet; picking one and pressing "Load Example" fills the REPL textarea for
"Run in Demo". It also includes a small browser-side shape editor for building
`PUTSH` / `SETSHAPE` commands against the current shape registry.

See [`docs/browser-demo.md`](docs/browser-demo.md) for a full local-serving
walkthrough and an honest list of what differs from the native window (mainly:
no real file I/O, and audio may need a user gesture first).

### WASM/core status

`dynalogo-core` also builds for `wasm32-unknown-unknown` and exposes a
cooperative runtime designed to be advanced from a browser render loop such as
`requestAnimationFrame`.

You can validate the core build with:

```bash
cargo check -p dynalogo-core --target wasm32-unknown-unknown
```

## Current feature snapshot

Implemented today:

- `TO ... END` procedures with dynamic scope and recursion
- Core control: `REPEAT IF IFELSE RUN RUNRESULT REPCOUNT`
- Richer v0.3 control: `TEST IFTRUE IFFALSE CATCH THROW ERROR WAIT`
- Library control structures: `FOR WHILE UNTIL DO.WHILE CASE COND`
- Lists/words: `FIRST BUTFIRST LAST BUTLAST FPUT LPUT SENTENCE LIST WORD`
- Data utilities: `COUNT ITEM EMPTYP EQUALP MEMBERP`
- Arithmetic, infix operators, comparisons, and boolean logic
- Variables and property lists: `MAKE THING LOCAL PPROP GPROP PLIST REMPROP`
- Arrays and templates: `ARRAY SETITEM LISTTOARRAY ARRAYTOLIST MAP FILTER`
  `REDUCE FOREACH APPLY CASCADE CASCADE.2 TRANSFER`
- Macros: `.MACRO .DEFMACRO MACROP MACRO? MACROEXPAND`
- File/outside-world helpers: `LOAD SAVE OPENREAD OPENWRITE OPENAPPEND`
  `READWORD READCHAR DRIBBLE KEYP JOY PADDLE TIMEOUT SETCURSOR SETENV`
- Static turtle graphics: `FD BK LT RT SETXY SETPOS SETH HOME CS PU PD`
  `PN SETPN PC SETPC SETPENSIZE SETSCRUNCH SETLABELHEIGHT LABEL HT ST POS HEADING XCOR YCOR`
- Dynaturtle shape surface: `SETSHAPE SHAPE PUTSH GETSH`, registry-backed
  custom-outline rendering, the browser shape editor, and `$EDITOR`-driven
  `EDSH`

Remaining compatibility notes now live in the reference and compatibility docs;
see [`docs/reference-manual.md`](docs/reference-manual.md),
[`docs/ucblogo-compatibility.md`](docs/ucblogo-compatibility.md), and
[`docs/atari-logo-validation.md`](docs/atari-logo-validation.md).

## Example programs

See [`examples/`](examples/) for runnable programs.

Classic turtle examples:

- `square.lgo` — the smallest turtle demo
- `flower.lgo` — procedures + repeated drawing
- `spiral.lgo` — arithmetic, variables, and looping

Dynaturtle examples:

- `shape_parade.lgo` — animated turtle / dog / ship shapes
- `dogs_in_the_park.lgo` — collision-driven barking with `WHEN` and `TOOT`
- `spaceship_thrust.lgo` — ship-thrust / inertia sketch
- `bouncing_ball.lgo` — collision-based bouncing demo
- `orbit_simulation.lgo` — orbit-style multi-body trails
- `pong_demons.lgo` — a small collision-demon pong sketch

## Releases

Tagged pushes (`v*.*.*`) trigger a GitHub Actions workflow that builds native
`dynalogo`/`dynalogo-window` binaries for Linux, macOS (arm64), and Windows
and attaches them to a GitHub Release. See
[docs/release-process.md](docs/release-process.md) for the build matrix and
known limitations (no code signing/notarization, no installers).

## Status

Early development, but the current REPL and window frontend are already useful
for experimenting with core Logo, static turtle graphics, and early
dynaturtle programs. See [ROADMAP.md](ROADMAP.md) for the version plan,
[PLAN.md](PLAN.md) for the architecture,
[docs/getting-started.md](docs/getting-started.md) for onboarding, and
[docs/primitive-inventory.md](docs/primitive-inventory.md) for a snapshot of
the currently implemented primitive surface.

## Versioning

See [docs/versioning.md](docs/versioning.md) for the version policy and
changelog process, and [CHANGELOG.md](CHANGELOG.md) for what's changed.

## License

MIT — see [LICENSE](LICENSE).
