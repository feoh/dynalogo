# Example programs

These programs are compatible with the current DynaLOGO feature set.

## Run from the terminal REPL

```bash
cargo run -p dynalogo --bin dynalogo < examples/square.lgo
```

## Run in the native window

Start the window app first:

```bash
cargo run -p dynalogo --bin dynalogo-window
```

Then paste a program into the prompt.

## Included examples

- `square.lgo` — basic turtle motion
- `flower.lgo` — procedures and repeated petals
- `spiral.lgo` — variables, arithmetic, and nested motion
