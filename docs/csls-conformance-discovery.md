# CSLS conformance discovery

This note captures the discovery baseline for the `dynalogo conformance`
workflow project. The goal is to make the Computer Science Logo Style (CSLS)
examples a first-class compatibility corpus for full UCBLogo compatibility.
Differences from UCBLogo behavior are treated as bugs or gaps unless they are
strictly additive DynaLOGO features, such as dynaturtle support.

## Current baseline

- The repository has an initial CSLS integration harness at
  `crates/dynalogo-core/tests/csls_examples.rs`.
- The current CSLS fixture set contains 16 `.lgo` files under
  `crates/dynalogo-core/tests/csls/`: 15 expected-output fixtures and 1
  expected-error fixture.
- The covered examples span selected chapters from CSLS volumes 1-3: words and
  lists, variables and procedures, predicates, higher-order functions,
  recursion, program-as-data, property lists, iteration/templates, ASCII
  helpers, combinatorics, tree algorithms, and language design.
- `cargo test -p dynalogo-core --test csls_examples -- --nocapture` passes for
  this baseline.
- `cargo test --workspace -q` passes for the current workspace snapshot.

## Compatibility surface already present

The current implementation is sufficient for the imported baseline fixtures and
includes these CSLS-relevant behaviors:

- procedure definitions and dynamic-scope variable lookup
- compact subtraction/negative expression forms such as `:n-1`
- greedy parenthesized calls for variadic-style primitives already handled by
  the VM, such as `(sentence ...)` and `(list ...)`
- UCBLogo-style `PRINT` rendering for list values without outer brackets
- templates with `?`, `?1`, `?2`, and `#` in the single-list and cascade cases
  currently represented in fixtures
- core higher-order primitives `FOREACH`, `MAP`, `FILTER`, `REDUCE`, `APPLY`,
  `CASCADE`, `CASCADE.2`, and `TRANSFER`
- workspace inspection primitives used by program-as-data examples (`TEXT`,
  `PO`, etc.)
- property-list primitives used by imported examples (`PPROP`, `GPROP`,
  `REMPROP`, `PLIST`)

## Gaps that still block the full CSLS corpus

These gaps are observable from the ready Witan tasks and local probes against
current VM behavior.

### Parser/expression compatibility

- Parenthesized expression grouping such as `(:x + 1)` is not accepted. Current
  parenthesized parsing expects a procedure-name call after `(`.
- Optional/default/rest procedure inputs in `TO` headers are not supported.
- The initial fixture set still represents a curated subset, so a later audit
  must compare fixture source against the original CSLS examples and remove
  adaptations.

### Template iteration compatibility

- Multi-list `MAP`/`FOREACH` does not bind `?1`, `?2`, ... across parallel
  inputs yet.
- `MAP.SE` is not implemented.
- Word inputs to `MAP` work for simple procedure-name templates, but the full
  UCBLogo template matrix still needs audit coverage for words,
  effects-vs-reporters, `?REST`, and multi-input cases.

### Missing CSLS/UCBLogo helpers

Missing primitives and aliases observed from task requirements and local probes
include:

- `LOCALMAKE`
- `MEMBER` as a tail reporter
- `FIND`
- `MAP.SE`
- `QUEUE`, `PUSH`, and `POP`
- `BOUNDP`
- `SUBSTRINGP`
- `PROCEDURES`
- `EOFP`
- cursor-position helpers such as `SETPOSN` where applicable

Library control structures exist in this implementation, but CSLS-specific
calling patterns should still be verified while importing chapters.

### Full-corpus harnesses

- File/input examples need a deterministic filesystem and input harness before
  they can be imported as stable tests.
- Graphics-heavy examples need deterministic turtle-state assertions rather
  than visual-only checks.
- Complete application chapters should be ported only after parser, templates,
  primitives, filesystem/input, and graphics harness gaps are closed.

## Recommended phase exit criteria

Discovery is complete enough to move to spec when the next phase focuses on:

- defining acceptance criteria for each ready task above;
- deciding how to represent original CSLS source attribution and any
  unavoidable adaptations;
- expanding the fixture harness in layers: pure output examples first, then
  input/filesystem examples, graphics examples, and finally full applications;
- keeping live-UCBLogo comparison optional while documenting whether committed
  outputs are derived from UCBLogo or CSLS text;
- keeping DynaLOGO additive-extension fixtures separate from the conformance
  corpus, or clearly marking them as extension-only tests rather than
  acceptable UCBLogo semantic differences.
