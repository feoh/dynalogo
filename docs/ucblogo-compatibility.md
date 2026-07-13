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
- `CATCH "ERROR`/`ERROR` list contents for error codes 5, 9, 11, 25, and 35
  (`error_codes`), pinning the numeric code plus message wording for the
  "didn't output to", "You don't say what to do with", "has no value",
  "IFTRUE/IFFALSE without TEST", and custom `THROW "ERROR` cases

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

## Additive extension cases

The compatibility target is no UCBLogo semantic differences. Differences from
UCBLogo behavior should be treated as bugs or compatibility gaps unless they are
strictly additive DynaLOGO features:

- dynaturtle primitives such as `TELL`, `ASK`, `WHO`, velocity, collision, and
  `WHEN` are DynaLOGO extensions rather than UCBLogo surface
- browser/window runtime behavior is outside the classic UCBLogo model
- Atari/dynaturtle compatibility helpers may exist even when no direct UCBLogo
  counterpart exists

As the compatibility corpus grows, additive extension fixtures should remain
separate from UCBLogo conformance fixtures or be clearly marked extension-only.

Error-code behavior is pinned by the compatibility fixtures plus focused VM
regression tests. Current coded families include wrong-type input (code 4),
missing output (5), not-enough-inputs (6), unused values (9), missing variables
(11), unknown procedures (13), `IFTRUE`/`IFFALSE` without `TEST` (25), and
custom `THROW "ERROR` values (35).
