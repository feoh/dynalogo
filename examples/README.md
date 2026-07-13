# Example programs

These programs are compatible with the current DynaLOGO feature set.

## Run from the terminal REPL

```bash
cargo run -p dynalogo --bin dynalogo < examples/square.lgo
```

The classic turtle examples work well in either frontend. The dynaturtle demos
below are best experienced in the native window or the [browser
demo](../docs/browser-demo.md) so their animation and sound can run
continuously.

## Run in the native window

Start the window app first:

```bash
cargo run -p dynalogo --bin dynalogo-window
```

Then paste a program into the prompt.

## Included examples

### Computer Science Logo Style corpus

- `csls/` — runnable examples adapted from Brian Harvey's *Computer Science Logo Style*, organized by volume/chapter with attribution, source URLs, and expected-output sidecars. See [`csls/README.md`](csls/README.md).

### Classic turtle graphics

- `square.lgo` — basic turtle motion
- `flower.lgo` — procedures and repeated petals
- `spiral.lgo` — variables, arithmetic, and nested motion

### Dynaturtle gallery

- `shape_parade.lgo` — showcases turtle, dog, and ship sprite rendering
- `dogs_in_the_park.lgo` — three dogs move toward the center, bark with
  `TOOT`, and use `WHEN [TOUCHING ...]`
- `spaceship_thrust.lgo` — a small ship-thrust / inertia sketch
- `bouncing_ball.lgo` — a moving ball that reverses direction on collision
- `orbit_simulation.lgo` — scripted multi-body orbit-style trails
- `pong_demons.lgo` — a tiny collision-demon pong sketch

The last four programs are best experienced in the native window frontend,
because their motion, collisions, and `TOOT` feedback are much easier to see
there than in the terminal REPL.
