# Example programs

These examples include small programs compatible with the current DynaLOGO
feature set plus a credited source corpus from Brian Harvey's *Computer Science
Logo Style* (CSLS). Some full CSLS transcripts intentionally preserve upstream
UCBLogo code that may exceed DynaLOGO's current compatibility surface.

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

- `csls/` — examples from Brian Harvey's *Computer Science Logo Style*,
  organized by volume/chapter with attribution, source URLs, expected-output
  sidecars for deterministic examples, and full upstream transcripts for the
  larger application/graphics examples. See [`csls/README.md`](csls/README.md).

### Classic turtle graphics

- `square.lgo` — basic turtle motion
- `flower.lgo` — procedures and repeated petals
- `spiral.lgo` — variables, arithmetic, and nested motion

### Dynaturtle gallery

- `shape_parade.lgo` — showcases turtle, dog, and ship sprite rendering
- `dogs_in_the_park.lgo` — three dogs scatter to random starting spots in a
  fenced, grassy park, bark and bounce off each other with
  `WHEN [TOUCHING ...]`, and bounce off the fence and trees via a
  turtle-over-pixel `WHEN [OVER ...]` collision against those obstacles'
  pen color
- `spaceship_thrust.lgo` — a small ship-thrust / inertia sketch
- `bouncing_ball.lgo` — a moving ball that reverses direction on collision
- `orbit_simulation.lgo` — scripted multi-body orbit-style trails
- `pong_demons.lgo` — a tiny collision-demon pong sketch

The last four programs are best experienced in the native window frontend,
because their motion, collisions, and `TOOT` feedback are much easier to see
there than in the terminal REPL.
