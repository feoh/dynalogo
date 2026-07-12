# WASM and Browser Embedding Guide

This guide is for developers embedding or deploying DynaLOGO in the browser.
It complements the user-facing [`browser-demo.md`](browser-demo.md) page.

## What currently ships

DynaLOGO already includes:

- a `wasm32-unknown-unknown` build of `dynalogo-window`
- `web/index.html` and `web/mq_js_bundle.js` as the demo shell
- `.github/workflows/pages.yml` for GitHub Pages deployment
- browser-side command queue plumbing in `dynalogo-window.rs`

The browser demo is therefore an application delivery target, not just an
experimental build.

## Build prerequisites

Install the target once:

```bash
rustup target add wasm32-unknown-unknown
```

Then build:

```bash
cargo build -p dynalogo --bin dynalogo-window --target wasm32-unknown-unknown
```

For a release-style build matching Pages more closely:

```bash
cargo build --release -p dynalogo --bin dynalogo-window \
  --target wasm32-unknown-unknown
```

The emitted artifact is:

- `target/wasm32-unknown-unknown/{debug|release}/dynalogo-window.wasm`

## Local serving pattern

Do not open the demo via `file://`.
Browsers will block the `.wasm` fetch and example-file fetches.

A minimal local site assembly looks like:

```bash
mkdir -p /tmp/dynalogo-web/examples
cp web/index.html web/mq_js_bundle.js /tmp/dynalogo-web/
cp target/wasm32-unknown-unknown/release/dynalogo-window.wasm /tmp/dynalogo-web/
cp examples/*.lgo /tmp/dynalogo-web/examples/
cd /tmp/dynalogo-web && python3 -m http.server 8080
```

Then open `http://localhost:8080`.

## Embedding model

The existing shell embeds DynaLOGO by:

1. loading the macroquad-generated JS bundle
2. letting that bundle load `dynalogo-window.wasm`
3. exposing an in-page UI that pushes Logo source into
   `window.__dynalogoCommands`
4. letting the Rust frontend drain and execute those commands once per frame

That means browser embedding is primarily an **input/output integration** task,
not a second interpreter implementation.

## Reusing the shell in another page

At minimum, an embedding page needs:

- the JS bundle
- the `.wasm` artifact in the expected relative location
- a `<canvas>` for macroquad
- optional UI that writes strings into `window.__dynalogoCommands`
- optional log sink matching the existing `repl-log` textarea pattern

If you build custom controls, keep them text-oriented and feed the same command
queue. Avoid creating browser-only semantic branches when the VM can already run
plain Logo source.

## GitHub Pages deployment

The current Pages workflow:

- builds the WASM binary
- assembles a site artifact from `web/`, the `.wasm`, and example files
- publishes it through GitHub Pages

Use it as the reference deployment path before inventing a separate browser
pipeline. If you change the shell layout, make the same change locally and in
`.github/workflows/pages.yml`.

## Current browser limitations

These are real platform limits, not missing docs:

- **No filesystem-backed primitives** in a browser runtime. `LOAD`, `SAVE`,
  `OPENREAD`, `OPENWRITE`, `OPENAPPEND`, and `DRIBBLE` depend on `std::fs`.
- **Audio may require a user gesture** before `TOOT` is audible.
- **Browser-only input glue** exists, but execution still happens through the
  same VM path as native input.
- **Performance/debugging signals differ** from native because you are running
  through the browser event loop and WASM runtime.

## Practical validation checklist

When touching browser/WASM support, the useful baseline is:

```bash
cargo build -p dynalogo --bin dynalogo-window --target wasm32-unknown-unknown
cargo check -p dynalogo-core --target wasm32-unknown-unknown
cargo test --workspace -q
cargo clippy --workspace --all-targets -- -D warnings
```

If you change shell behavior, also validate that:

- the page loads over HTTP
- the example loader still works
- the REPL log updates
- browser-specific docs still match reality

## Related docs

- [`browser-demo.md`](browser-demo.md) — user-facing local demo walkthrough
- [`release-process.md`](release-process.md) — native release packaging, not
  browser deployment
- [`developer-guide.md`](developer-guide.md) — contributor-focused internals
