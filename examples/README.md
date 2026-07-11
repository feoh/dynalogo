# Example programs

These programs are compatible with the current DynaLOGO feature set.

## Run from the terminal REPL

```bash
cargo run -p dynalogo --bin dynalogo < examples/square.lgo
```

The classic turtle examples work well in either frontend. The dynaturtle demos
below are best experienced in the native window so their animation and sound
can run continuously.

## Run in the native window

Start the window app first:

```bash
cargo run -p dynalogo --bin dynalogo-window
```

Then paste a program into the prompt.

## Included examples

### Classic turtle graphics

- `square.lgo` — basic turtle motion
- `flower.lgo` — procedures and repeated petals
- `spiral.lgo` — variables, arithmetic, and nested motion

### Dynaturtle gallery

- `shape_parade.lgo` — showcases turtle, dog, and ship sprite rendering
- `dogs_in_the_park.lgo` — three dogs move toward the center, bark with
  `TOOT`, and use `WHEN [TOUCHING ...]`
