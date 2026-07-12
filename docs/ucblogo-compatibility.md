# UCBLogo compatibility suite

This repository now includes a small compatibility corpus under
`crates/dynalogo-core/tests/ucblogo/` plus a dedicated harness in
`crates/dynalogo-core/tests/ucblogo_compatibility.rs`.

## What it covers

The suite currently tracks three categories of behavior that should match
classic UCBLogo output on the current integration line:

- core control and expression evaluation
- `CATCH "ERROR` / `ERROR` behavior
- workspace procedure source via `TEXT`

It also includes an explicit **DynaLOGO-only** fixture for dynaturtle turtle
selection (`TELL`/`SETPOS`/`POS`) so intentional extensions are documented in
the same corpus.

## How it works

For fixtures marked as UCBLogo-compatible, the harness always checks DynaLOGO
against committed canonical `.out` files.

When a real UCBLogo executable is available, the same test also runs the
fixture against UCBLogo and verifies that the live output matches the committed
reference output.

The harness looks for UCBLogo in this order:

1. `$UCBLOGO_BIN`
2. `ucblogo` on `PATH`
3. `logo` on `PATH`

If no executable is found, the live-UCBLogo comparison is skipped and only the
committed canonical output is checked.

## Running it

DynaLOGO-only / canonical checks:

```bash
cargo test -p dynalogo-core --test ucblogo_compatibility
```

With a live UCBLogo binary available:

```bash
UCBLOGO_BIN=/path/to/logo \
  cargo test -p dynalogo-core --test ucblogo_compatibility -- --nocapture
```

## Intentional divergences

The suite currently documents these intentional differences from UCBLogo:

- dynaturtle primitives such as `TELL`, `ASK`, `WHO`, velocity, collision, and
  `WHEN` are DynaLOGO extensions rather than UCBLogo surface
- browser/window runtime behavior is outside the classic UCBLogo model
- Atari/dynaturtle compatibility helpers may exist even when no direct UCBLogo
  counterpart exists

As the compatibility corpus grows, additional divergences should be added here
instead of being left implicit in test code.
