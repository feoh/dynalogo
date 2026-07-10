# DynaLOGO — Plan

A new implementation of the LOGO programming language in Rust. Syntax follows
UCBLogo (<https://people.eecs.berkeley.edu/~bh/logo>), extended with
**Dynaturtles** — turtles with velocity, heading, and collision events, as in
Papert's _Mindstorms_, Atari LOGO, and MicroWorlds LOGO. Performance is a
first-class requirement: near-real-time simulation, collision detection, and
timing-sensitive demon execution.

- **Location:** `~/src/personal/dynalogo`
- **GitHub:** `feoh/dynalogo`, **public**
- **License:** MIT
- **Language:** Rust
- **Targets:** native desktop first; web/WASM in v0.6
- **Witan project:** `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7`
  (8 epics E0–E7 mirroring the roadmap, 33 dependency-linked sub-tasks)

## Architecture

### Workspace layout

```
dynalogo/
├── Cargo.toml            # workspace root
├── README.md
├── ROADMAP.md            # versioned milestones
├── PLAN.md               # mirror of this plan
├── crates/
│   ├── dynalogo-core/    # lib: lexer, parser, bytecode compiler, VM, values,
│   │                     #   workspace (procedures/vars/plists), sim engine
│   └── dynalogo/         # bin: frontend — window, turtle rendering, REPL
└── tests/                # integration: .lgo programs + expected output
```

`dynalogo-core` has **no graphics dependencies** — it exposes a
`TurtleBackend` trait so the core is testable headless and portable to WASM.
The `dynalogo` binary implements that trait with macroquad (targets both
native and WASM with the same code).

### Interpreter: bytecode VM, not a tree-walker

- **Lexer/Reader**: UCBLogo tokenization — words, `[lists]`, `(parens)`,
  infix operators, `"quoted` words, `:dots` (thing), `;comments`, `~`
  line continuation.
- **Parser**: instruction lists (homoiconic — lists ARE code). A compiler
  lowers instruction lists to bytecode on first execution; compiled chunks
  are cached per procedure/list and invalidated on redefinition (`TO`,
  `DEFINE`, `ERASE`).
- **VM**: stack-based, dynamic scoping via a frame stack (UCBLogo
  semantics). Tail-call optimization. `RUN`/`RUNRESULT`/`APPLY`
  compile-and-cache their list arguments keyed by list identity.
- **Values**: `Word` (interned symbols + numbers), `List` (immutable,
  Rc-based, cheap to share), `Array` (mutable, 1-origin default). Numbers
  are f64 with integer display rules matching UCBLogo.

### Dynaturtle engine

- **Fixed-timestep simulation** (60 Hz tick) decoupled from rendering;
  renderer interpolates positions between ticks.
- **Turtle storage as struct-of-arrays**: positions, velocities, headings,
  speeds, pen state, shape, visibility in flat `Vec`s.
- **Collision detection**: broad phase via spatial hash grid; narrow phase
  turtle-turtle, turtle-edge, and pen-line contact (Atari LOGO's
  `TOUCHING` included drawn lines).
- **Demons** (Atari LOGO `WHEN` model): `WHEN TOUCHING [t1 t2] [instrs]`,
  `WHEN OVER color [...]`, edge events. Events enqueue demon bodies; the VM
  drains the queue each tick with a per-tick fuel budget.
- **Concurrency**: simulation+VM on its own thread; REPL feeds it over a
  channel; renderer reads double-buffered snapshots. Typing at the REPL
  never freezes moving turtles.

### Dynaturtle language surface (v0.2)

- `SETSPEED n`, `SPEED`, `SETVELOCITY [dx dy]`, `TELL n`/`TELL [list]`,
  `ASK n [...]`, `WHO`, `EACH [...]`
- `WHEN TOUCHING [a b] [...]`, `WHEN EDGE [...]`, demon management
- `SETSHAPE`, collision radius, `BOUNCE`/`WRAP`/`FENCE`/`WINDOW` edge modes

## Version roadmap

See ROADMAP.md for the full detail. Summary:

| Version | Theme |
|---|---|
| v0.1 | Core language + static turtle graphics (lexer → bytecode VM, TO/END, core primitives, REPL, headless harness) |
| v0.2 | Dynaturtles (sim thread, TELL/ASK, velocity, spatial-hash collision, WHEN demons, edge modes) |
| v0.3 | Rich language core (templates, CATCH/THROW, plists, arrays, error messages) |
| v0.4 | Workspace & I/O (LOAD/SAVE, streams, EDIT, docs + examples) |
| v0.5 | Macros & perf (.MACRO, 1,000 colliding dynaturtles @ 60 Hz) |
| v0.6 | Web/WASM |
| v1.0 | Full UCBLogo library parity (~400 primitives) + polish |
