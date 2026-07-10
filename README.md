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

| Crate | Purpose |
| --- | --- |
| `crates/dynalogo-core` | Lexer, parser, bytecode compiler, VM, values, dynaturtle sim engine. |
| `crates/dynalogo` | Native frontend: window, turtle rendering, REPL. |

`dynalogo-core` is headless and has no graphics dependencies.

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
- Arrays and templates: `ARRAY SETITEM LISTTOARRAY ARRAYTOLIST MAP FILTER REDUCE FOREACH APPLY`
- Static turtle graphics: `FD BK LT RT SETXY SETPOS SETH HOME CS PU PD SETPC SETPENSIZE HT ST POS HEADING XCOR YCOR`

Still in progress: file/workspace primitives, macro support, full UCBLogo error
parity, dynaturtle simulation commands, and browser/WASM support.

## Example programs

See [`examples/`](examples/) for small programs that run against the current
v0.1 feature set:

- `square.lgo` — the smallest turtle demo
- `flower.lgo` — procedures + repeated drawing
- `spiral.lgo` — arithmetic, variables, and looping

## Status

Early development, but the current REPL and window frontend are already useful
for experimenting with core Logo and static turtle graphics. See
[ROADMAP.md](ROADMAP.md) for the version plan, [PLAN.md](PLAN.md) for the
architecture, and [docs/primitive-inventory.md](docs/primitive-inventory.md)
for a snapshot of the currently implemented primitive surface.

## License

MIT — see [LICENSE](LICENSE).
