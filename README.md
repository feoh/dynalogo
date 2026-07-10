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
|---|---|
| `crates/dynalogo-core` | Lexer, parser, bytecode compiler, VM, values, dynaturtle sim engine. Headless — no graphics dependencies. |
| `crates/dynalogo` | Native frontend: window, turtle rendering, REPL. |

## Status

Early development. See [ROADMAP.md](ROADMAP.md) for the version plan and
[PLAN.md](PLAN.md) for the architecture.

## License

MIT — see [LICENSE](LICENSE).
