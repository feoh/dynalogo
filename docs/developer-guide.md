# Developer Guide

This guide is for contributors working on DynaLOGO internals rather than using
Logo from the REPL.

It focuses on the current architecture in this repository:

- `crates/dynalogo-core` for the language/runtime core
- `crates/dynalogo` for native/browser frontends

## Workspace overview

- `crates/dynalogo-core/src/lexer.rs` — tokenization
- `crates/dynalogo-core/src/parser.rs` — arity-driven parsing into expressions
- `crates/dynalogo-core/src/bytecode.rs` — bytecode lowering structures
- `crates/dynalogo-core/src/vm.rs` — main interpreter, primitive dispatch,
  workspace state, file/editor helpers, and many regression tests
- `crates/dynalogo-core/src/dynaturtle.rs` — `TurtleStore`, collision data,
  edge modes, and simulation-side turtle state
- `crates/dynalogo-core/src/demon.rs` — `WHEN`/event scheduling
- `crates/dynalogo-core/src/runtime.rs` and `sim.rs` — native/cooperative
  runtime helpers and simulation stepping
- `crates/dynalogo/src/main.rs` — terminal REPL frontend
- `crates/dynalogo/src/bin/dynalogo-window.rs` — native/browser macroquad
  frontend

## Language pipeline at a glance

1. **Lexing** converts source text into `Token`s.
2. **Parsing** uses the `ArityTable` to decide how many inputs a primitive or
   procedure consumes and builds expression trees.
3. **Evaluation/bytecode** runs through `Vm`; some paths are interpreted from
   parsed forms, while instruction-list execution and chunk caching support the
   performance-sensitive paths.
4. **Output/control flow** is represented through `ControlFlow`, `RunResult`,
   the VM stack, and `PrimitiveResult`.

For most feature work, `vm.rs` is the integration point where parser-visible
surface meets runtime semantics.

## Adding a primitive

The common workflow is:

1. **Add parser arity** in `crates/dynalogo-core/src/parser.rs`.
2. **Add VM dispatch** in `Vm::call` in `crates/dynalogo-core/src/vm.rs`.
3. **Implement the primitive method** in `vm.rs`.
4. **Add the name to `primitive_names()`** if it is a true primitive.
5. **Document it** in `docs/primitive-inventory.md` and, if relevant,
   `docs/reference-manual.md` or parity docs.
6. **Add tests** close to the feature in `vm.rs` or the compatibility fixture
   harness.

Typical implementation patterns already exist for:

- numeric inputs via helpers such as `number_input`
- word/list/array validation helpers
- `expect_arity(...)`
- workspace commands (`TEXT`, `COPYDEF`, `EDIT`, `PO*`, `ER*`)
- dynaturtle commands (`TELL`, `ASK`, `SETSPEED`, `WHEN`, `TOOT`)

## Parser and lexer extension points

### Lexer

Change `lexer.rs` when you need a new token form or delimiter behavior.
Examples:

- quoted words
- colon-prefixed variables
- infix operators
- bracket/paren handling

### Parser

Change `parser.rs` when the syntax shape changes but tokenization does not.
Examples:

- adding/removing primitive arities
- supporting greedy parenthesized calls
- adjusting unary minus or list-literal behavior
- changing how instruction lists or templates are recognized

The parser is intentionally **arity-driven**, so many syntax changes are just
arity-table and expression-shape changes rather than grammar rewrites.

## Dynaturtle architecture

`TurtleStore` is the current shared turtle state model.

It is responsible for:

- active-turtle selection (`TELL`/`ASK`/`EACH`)
- turtle state snapshots
- motion integration and velocity storage
- event accumulation (`Line`, `Label`, `Fill`, `Clear`, `State`)
- edge-mode application (`BOUNCE`, `WRAP`, `FENCE`, `WINDOW`)
- shape/collision metadata used by the simulation and frontends

Collision/event flow is roughly:

1. motion integration in `dynaturtle_tick`
2. optional edge-mode application
3. collision detection / event derivation
4. demon scheduling (`WHEN`)
5. instruction-list execution for triggered demons

`collision::SpatialHash` is the broad phase for turtle/turtle collisions. It
sorts occupied cells for deterministic traversal, emits same-cell pairs and only
forward neighboring-cell pairs so each candidate is produced once, then sorts
`CollisionPair` output by turtle id. Keep this duplicate-free invariant intact
when changing collision performance.

If you are changing dynaturtle semantics, inspect `dynaturtle.rs`, `demon.rs`,
and the `Vm::dynaturtle_tick` path together.

## Frontend responsibilities

The frontends do **not** implement Logo semantics.

They are mainly responsible for:

- collecting user input
- forwarding text to `Vm::eval_source`
- drawing from `TurtleEvent` streams and turtle snapshots
- advancing dynaturtle ticks in the window/browser path
- handling frontend-only concerns like browser command queues or sound output

The macroquad window keeps UI responsiveness separate from Logo semantics:

- The turtle trail is cached by `graphics::RasterCache`, which incrementally
  applies newly appended drawing events and writes transparent RGBA bytes for a
  persistent macroquad `Texture2D`. The cache rebuilds when the event log is
  truncated, the window texture size changes, or `SETSCRUNCH` changes. Labels
  and live turtle sprites remain vector/frontend draws on top of the cached
  trail.
- Native command evaluation is queued and run on an `EvalWorker` thread using a
  cloned `Vm`. The frame loop continues accepting input and rendering while the
  worker runs, then swaps in the returned VM/result. Simulation ticks pause
  during a native eval so foreground and worker VM state cannot diverge.
- Browser/WASM command evaluation uses the same queue shape but remains
  synchronous because browser worker/thread integration is not enabled.
- Dynaturtle simulation in `dynalogo-window` advances through `FixedTimestep`
  using measured frame deltas rather than assuming exactly one rendered frame
  per 1/60-second simulation tick.

When behavior differs between native and browser builds, prefer documenting the
limitation rather than forking language semantics.

## Browser/WASM notes for contributors

The browser build shares the same `dynalogo-window.rs` source under
`cfg(target_arch = "wasm32")` gates.

Important constraints:

- filesystem-backed primitives do not work in-browser
- audio may require a user gesture
- the browser side panel pushes commands through JS, but execution still goes
  through the same VM path as native input

See also [`wasm-and-browser.md`](wasm-and-browser.md).

## Testing strategy

Use the narrowest useful layer:

- **unit tests in `vm.rs`** for primitive semantics and error behavior
- **fixture tests** under `crates/dynalogo-core/tests/ucblogo/` for
  compatibility-style output locking
- **frontend pure-function tests** in `dynalogo-window.rs` for math/input
  helpers extracted away from macroquad
- **docs updates** when behavior changes in ways users will observe

Before considering a slice done, prefer the same validation the rest of the
repo uses:

```bash
cargo fmt --check
cargo test --workspace -q
cargo clippy --workspace --all-targets -- -D warnings
ruby scripts/validate_workflows.rb
node web/shape_editor_test.js
```

For window/browser-facing changes, also smoke-build the WASM target when
possible:

```bash
cargo build -p dynalogo --bin dynalogo-window --target wasm32-unknown-unknown
```

For performance-sensitive changes, at least compile the affected Criterion
bench target in test mode, and run full benchmarks when comparing before/after
numbers:

```bash
cargo bench -p dynalogo-core --bench collision_bench -- --test
cargo bench -p dynalogo-core --bench dynaturtle_tick_bench -- --test
cargo bench -p dynalogo-core --bench vm_bench -- --test
```

## Practical contributor advice

- Prefer extending existing helpers before adding one-off validation paths.
- Keep docs honest about partial parity.
- When older branches conflict with current architecture, port behavior — do
  not revive obsolete state models.
- For browser/native differences, keep a single semantic core and isolate only
  the platform-specific I/O/rendering pieces.

For user-facing docs, start from [`README.md`](../README.md) and
[`README.md`](README.md).
