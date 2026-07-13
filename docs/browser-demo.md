# Browser Demo (WASM)

DynaLOGO's window frontend, `dynalogo-window`, also compiles to
`wasm32-unknown-unknown` via [macroquad](https://github.com/not-fl3/macroquad)
and ships as a small in-page REPL demo. This page covers running it locally,
how it's structured, and how it differs from the native window.

## Running it locally

Add the WASM target once:

```bash
rustup target add wasm32-unknown-unknown
```

Build the release binary (this is the build Pages should run once deployment
is re-enabled):

```bash
cargo build --release -p dynalogo --bin dynalogo-window \
  --target wasm32-unknown-unknown
```

Assemble a scratch site directory and serve it over HTTP. A browser won't let
the page `fetch()` the compiled `.wasm` or the example `.lgo` files over
`file://`, so you need a local HTTP server, not just opening `index.html`
directly:

```bash
mkdir -p /tmp/dynalogo-web/examples
cp web/index.html web/mq_js_bundle.js /tmp/dynalogo-web/
cp target/wasm32-unknown-unknown/release/dynalogo-window.wasm /tmp/dynalogo-web/
cp examples/*.lgo /tmp/dynalogo-web/examples/
cd /tmp/dynalogo-web && python3 -m http.server 8080
```

Then open `http://localhost:8080` in a browser.

This mirrors the intended Pages artifact layout. Automatic Pages deployment is
currently disabled because GitHub-hosted release-mode WASM linking fails for the
macroquad browser binary even though local release builds pass. See
[`.github/workflows/pages.yml`](../.github/workflows/pages.yml) for the disabled
workflow note before re-enabling the live deployment.

## How the demo page works

[`web/index.html`](../web/index.html) loads the compiled binary onto a
`<canvas>` and adds a side panel that:

- lets you load one of the bundled example programs into a textarea
- pushes the textarea's contents onto a JS array,
  `window.__dynalogoCommands`, when you press **Run in Demo**
- includes a small shape-editor section that builds `PUTSH` / `SETSHAPE`
  commands for registry-backed custom outline shapes
- mirrors the in-app REPL log back into a read-only textarea

On the Rust side, `dynalogo-window` drains that JS array once per frame
(`drain_browser_commands` / `handle_browser_commands`) and evaluates each
queued command exactly the same way the native window evaluates typed input.
There is no separate browser code path for running Logo — the panel is just
an alternate way of getting text into the same `eval_command` call the native
prompt uses.

You can also type directly into the canvas prompt as you would in the native
window, but the canvas needs focus first — click it once before typing, or
keystrokes may not reach the app.

The shape-editor panel has a dependency-free regression test:

```bash
node web/shape_editor_test.js
```

That test extracts the actual inline shape-editor functions from
`web/index.html`, runs them against a fake DOM, and verifies the queued `PUTSH`
/ `SETSHAPE` commands.

## What's different from the native window

The browser build shares the same `dynalogo-window` source and VM core as the
native window frontend, so classic turtle graphics, dynaturtles, `WHEN`
demons, and the REPL all behave the same. A few things genuinely differ:

- **No real file I/O.** `LOAD`, `SAVE`, `OPENREAD`, `OPENWRITE`,
  `OPENAPPEND`, and `DRIBBLE` call directly into `std::fs`. On
  `wasm32-unknown-unknown` in a browser there is no filesystem for those
  calls to reach, so they will error rather than read or write anything
  useful. Loading example programs into the demo works because the page
  fetches file contents over HTTP into the textarea, not through these
  primitives.
- **Audio may need a user gesture.** Browsers commonly block audio playback
  until the page has seen a user interaction. If `TOOT` (the bark sound in
  `dogs_in_the_park.lgo`) seems silent on first load, click the canvas or a
  panel button first, then try again.
- **Extra JS-driven input path.** The side panel's example loader and shape
  editor queue commands through browser-only JS helpers; the native window only
  has the in-canvas typed prompt.

Everything else — turtle rendering, collision detection, `WHEN`/`TOUCHING`,
shapes, and the fixed-tick dynaturtle simulation — runs identically to the
native window, just compiled to WASM instead of a native binary.

## Where this fits

- [`getting-started.md`](getting-started.md) — general onboarding, including
  the native window walkthrough this demo mirrors
- [`../examples/README.md`](../examples/README.md) — the example gallery the
  demo panel loads from
- [`reference-manual.md`](reference-manual.md) — frontend parity notes in
  context with the rest of the language reference
