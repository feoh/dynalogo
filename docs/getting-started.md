# Getting Started with DynaLOGO

This tutorial gets you from either a downloaded release package or a fresh
source checkout to running Logo programs in both DynaLOGO frontends:

- the **terminal REPL**
- the **native turtle window**

It also introduces the two big ideas in the current implementation:

- classic Logo turtle graphics
- **dynaturtles** — turtles with velocity, collision detection, and `WHEN`
  event handlers

## Installation

### Use a packaged release

Most users should download the latest release from the
[GitHub Releases page](https://github.com/feoh/dynalogo/releases/latest).
Choose the archive for your platform:

- **Linux x86_64:** `dynalogo-x86_64-unknown-linux-gnu.tar.gz`
- **macOS Apple silicon:** `dynalogo-aarch64-apple-darwin.zip`
- **Windows x86_64:** `dynalogo-x86_64-pc-windows-msvc.zip`

Each archive contains both `dynalogo` (the terminal REPL) and
`dynalogo-window` (the native turtle window), plus the examples and license.
There are no installers; extract the archive and run the executable directly.

On Linux, extract and start the REPL with:

```bash
tar -xzf dynalogo-x86_64-unknown-linux-gnu.tar.gz
cd dynalogo-x86_64-unknown-linux-gnu
./dynalogo
```

On macOS, extract the Apple-silicon archive with Finder or `unzip`, then run:

```bash
unzip dynalogo-aarch64-apple-darwin.zip
cd dynalogo-aarch64-apple-darwin
./dynalogo
```

macOS packages are unsigned, so macOS may require approval in **System
Settings → Privacy & Security** before the executable can run.

On Windows PowerShell, extract the archive and start the REPL with:

```powershell
Expand-Archive .\dynalogo-x86_64-pc-windows-msvc.zip -DestinationPath .
Set-Location .\dynalogo-x86_64-pc-windows-msvc
.\dynalogo.exe
```

Windows packages are unsigned and may receive a SmartScreen warning. The
window frontend is the sibling executable: `./dynalogo-window` on Linux/macOS
or `.\dynalogo-window.exe` on Windows. To use the commands from any
terminal, add the extracted directory to your `PATH`.

For the package contents, supported platforms, and current release limitations,
see [`release-process.md`](release-process.md).

### Build from source instead

A source build requires a working Rust toolchain with Cargo installed. From
the repository root, build everything once:

```bash
cargo build
```

The commands below use `cargo run` so they work from a source checkout. If you
installed a release package, replace the corresponding `cargo run ... dynalogo`
command with `dynalogo` (or `./dynalogo`), and replace
`cargo run ... dynalogo-window` with `dynalogo-window` (or
`./dynalogo-window`).

## 1. Start the terminal REPL

Launch the command-line frontend:

```bash
cargo run -p dynalogo --bin dynalogo
```

You should see a `?` prompt.

Try a simple arithmetic command:

```logo
print sum 2 3
```

Expected output:

```text
5
```

Leave the REPL with:

```text
bye
```

You can also evaluate one short program directly from the shell:

```bash
cargo run -p dynalogo --bin dynalogo -- --eval 'print sum 2 3'
```

## 2. Draw your first turtle square

Start the REPL again:

```bash
cargo run -p dynalogo --bin dynalogo
```

Now enter:

```logo
repeat 4 [fd 100 rt 90]
```

This uses:

- `REPEAT` to loop
- `FD` (`FORWARD`) to move
- `RT` (`RIGHT`) to turn

If you prefer, you can run the same program from a file:

```bash
cargo run -p dynalogo --bin dynalogo < examples/square.lgo
```

## 3. Define a procedure

DynaLOGO supports classic `TO ... END` procedure definitions.

In the REPL, define a reusable triangle procedure:

```logo
to triangle :size
  repeat 3 [fd :size rt 120]
end
```

Then call it:

```logo
triangle 80
```

A few important ideas show up here:

- `:size` reads a variable
- procedures are introduced with `TO name ... END`
- procedure bodies can use other Logo commands freely

## 4. Run the native turtle window

The terminal REPL is useful for quick experiments, but the window frontend is
better for visual turtle work and dynaturtle demos.

Launch it with:

```bash
cargo run -p dynalogo --bin dynalogo-window
```

The window shows:

- a centered Cartesian drawing area
- a small command log at the bottom
- a prompt where you can type or paste Logo commands

Paste this into the window prompt:

```logo
repeat 36 [fd 120 rt 170]
```

You should see a dense starburst pattern.

## 5. Run the bundled examples

The repository includes examples in [`examples/`](../examples/).

Classic turtle examples:

- `examples/square.lgo`
- `examples/flower.lgo`
- `examples/spiral.lgo`

Dynaturtle examples:

- `examples/shape_parade.lgo`
- `examples/dogs_in_the_park.lgo`

To run an example in the terminal frontend:

```bash
cargo run -p dynalogo --bin dynalogo < examples/flower.lgo
```

For the window frontend, start the app first and then paste the file contents
into the prompt.

## 6. Meet dynaturtles

Dynaturtles extend classic Logo with multiple turtles and simple simulation
behavior.

The most important commands to learn first are:

- `TELL` — choose one or more active turtles
- `ASK` — run a block as one turtle
- `EACH` — run a block for every active turtle
- `WHO` — report the current active turtle selection
- `SETVELOCITY` / `SETSPEED` — continuous motion
- `SETSHAPE` — choose a rendered shape such as `"turtle`, `"dog`, or `"ship`
- `WHEN` — register event-driven behavior
- `TOUCHING` — collision predicate / demon condition

A tiny multi-turtle example:

```logo
tell [0 1 2]
ask 0 [setshape "turtle 10 setxy 20 110]
ask 1 [setshape "dog 12 setxy 160 70]
ask 2 [setshape "ship 12 setxy 300 120]
print "ready
```

This is essentially the same idea as `examples/shape_parade.lgo`.

## 7. Try animated dynaturtles in the window

Open the native window:

```bash
cargo run -p dynalogo --bin dynalogo-window
```

Paste this small program:

```logo
tell [0 1 2]
ask 0 [setshape "dog 12 setxy 20 20 setheading 45 setspeed 55]
ask 1 [setshape "dog 12 setxy 220 20 setheading 315 setspeed 55]
ask 2 [setshape "ship 12 setxy 120 200 setheading 180 setspeed 55]
```

You should see animated shapes moving continuously because the window frontend
advances the dynaturtle simulation every frame.

## 8. Try a collision + sound demo

DynaLOGO currently includes a `TOOT` primitive that the native window frontend
turns into a short bark-like sound effect.

The easiest way to try it is with the bundled dogs demo.

Start the window:

```bash
cargo run -p dynalogo --bin dynalogo-window
```

Then paste `examples/dogs_in_the_park.lgo` into the prompt.

That demo shows:

- three dogs with `SETSHAPE "dog`
- movement driven by `SETSPEED`
- `WHEN [TOUCHING ...]` collision handlers
- `TOOT`-driven sound and a visible `TOOT!` flash in the window

## 9. Useful workflow shortcuts

A few commands are handy while exploring:

Run one expression from the shell:

```bash
cargo run -p dynalogo --bin dynalogo -- --eval 'print word "dyna "logo'
```

Run a file in the terminal frontend:

```bash
cargo run -p dynalogo --bin dynalogo < examples/spiral.lgo
```

Inspect the example list:

```bash
sed -n '1,200p' examples/README.md
```

## 10. Where to go next

After this tutorial, useful next reads are:

- [`../README.md`](../README.md) — project overview and current status
- [`browser-demo.md`](browser-demo.md) — running the WASM browser demo and
  what differs from the native window you just used
- [`../examples/README.md`](../examples/README.md) — example gallery
- [`primitive-inventory.md`](primitive-inventory.md) — implemented primitive
  snapshot
- [`reference-manual.md`](reference-manual.md) — the main feature-by-feature
  user reference
- [`ucblogo-compatibility.md`](ucblogo-compatibility.md) — compatibility corpus
  and intentional divergence notes
