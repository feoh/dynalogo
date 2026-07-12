# Debugging and Errors

This guide collects the most practical ways to understand failures in DynaLOGO.
It is aimed at both advanced users and contributors.

## First debugging steps

When something goes wrong, start small:

1. reproduce it with a minimal Logo snippet
2. run it in the terminal frontend first if the window/browser adds noise
3. check whether the failure is a parse issue, runtime error, or frontend-only
   issue

Useful one-shot command pattern:

```bash
cargo run -p dynalogo --bin dynalogo -- --eval 'print sum 2 3'
```

Or for a slightly larger repro:

```bash
cargo run -p dynalogo --bin dynalogo < examples/square.lgo
```

## Understanding `CATCH` / `ERROR`

DynaLOGO carries forward UCBLogo-style `CATCH "ERROR` behavior.

If you wrap code like:

```logo
catch "error [print first []]
print error
```

then `ERROR` reports a structured list describing the last caught error.
That list is the main debugging interface for Logo-level error handling.

For compatibility details and the current coded error map, see:

- [`ucblogo-compatibility.md`](ucblogo-compatibility.md)
- [`ucblogo-error-audit.md`](ucblogo-error-audit.md)

## Common error families

### “I don't know how to X”

This means the VM could not find a procedure or primitive named `X`.
Common causes:

- typo in a primitive name
- procedure not yet defined
- assuming a UCBLogo/Atari primitive exists when DynaLOGO has not implemented
  it yet

### “not enough inputs to X”

A primitive or procedure was called with too few inputs.
Reduce the expression to one line and verify the arity in the reference docs.

### “You don't say what to do with X”

A value-producing expression was evaluated where no consumer used the value.
This often happens when mixing command-style and operation-style usage.

### “X has no value”

A variable lookup (`:name` or `THING`) failed because the variable was never
initialized in the current dynamic environment.

### “X doesn't like Y as input”

This is the UCBLogo-style wrong-type / invalid-input idiom being rolled out
across the codebase. It usually means the primitive recognized the input shape
but rejected its type or emptiness.

## Frontend-specific debugging

### Native window

Use the native window when debugging:

- dynaturtle motion
- `TOOT` behavior
- collision-triggered `WHEN` logic
- turtle rendering and event replay

If visual behavior seems wrong, reduce the program until you can tell whether
it is:

- a VM/turtle-state issue
- an event-stream issue (`Line`, `Label`, `Fill`, `Clear`)
- a pure frontend rendering issue

### Browser/WASM

Browser debugging has extra failure modes:

- missing HTTP serving (`file://` cannot load the demo correctly)
- blocked audio until user gesture
- filesystem-backed primitives failing by design in-browser

See [`browser-demo.md`](browser-demo.md) and
[`wasm-and-browser.md`](wasm-and-browser.md).

## Dynaturtle troubleshooting

If a dynaturtle demo is not behaving as expected, check these in order:

1. **Active turtle selection** — `TELL`, `ASK`, `EACH`, `WHO`
2. **Velocity setup** — `SETVELOCITY`, `SETSPEED`, `SPEED`
3. **Shape/visibility** — `SETSHAPE`, `HT`, `ST`, `SHOWNP`
4. **Edge mode** — `BOUNCE`, `WRAP`, `FENCE`, `WINDOW`
5. **Collision predicates** — `TOUCHING`, `WHEN`

A large share of “nothing happened” bugs are really selection or state bugs.

## Workspace/editor troubleshooting

For editor-driven commands:

- `EDIT`/`ED` require `$EDITOR` or `Vm::set_editor_command(...)`
- `EDNS` uses the same editor flow for visible global variables
- `EDSH` is not interactive yet; use `PUTSH`/`GETSH` directly or the browser
  shape editor to create custom outlines

In browser/WASM frontends, filesystem/editor-backed flows are not available in
any meaningful way.

## Performance troubleshooting

For performance-sensitive work:

- prefer the native frontend or direct tests/benches over browser impressions
- look for repeated instruction-list execution, high turtle counts, or heavy
  collision setups
- use the existing benchmark/perf work as precedent before changing core loops

If you are changing runtime performance, validate with normal correctness checks
first, then benchmark separately.

## Contributor debugging workflow

When working on the implementation itself, the standard loop is:

```bash
cargo fmt --check
cargo test --workspace -q
cargo clippy --workspace --all-targets -- -D warnings
```

Then, if the change is compatibility-sensitive:

- add or update a fixture in `crates/dynalogo-core/tests/ucblogo/`
- update the compatibility docs if behavior changed intentionally

## Related docs

- [`reference-manual.md`](reference-manual.md)
- [`ucblogo-compatibility.md`](ucblogo-compatibility.md)
- [`ucblogo-error-audit.md`](ucblogo-error-audit.md)
- [`developer-guide.md`](developer-guide.md)
