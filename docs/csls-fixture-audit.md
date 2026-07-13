# CSLS fixture audit

Audit date: 2026-07-13

This audit checked the committed CSLS fixtures under:

- `crates/dynalogo-core/tests/csls/`
- `crates/dynalogo-core/tests/csls_graphics/`
- `crates/dynalogo-core/tests/csls_input/`

## Restored original source forms

The following DynaLOGO-specific adaptations were removed after parser/template/control compatibility support was available:

- `v1_ch05_higher_order_functions.lgo`
  - Restored original omitted-step `FOR` controls: `for [number 4 7]` and `for [i 7 5]`.
  - Restored original compact arithmetic in `range`: `:to-:from` and `1+last ?`.
- `v3_ch02_combinatorics.lgo`
  - Restored original `fact` template: `[# * ?]`.
  - Restored original infix permutation expression: `(fact :n)/(fact (:n-:r))`.
- `v3_ch04_language_design.lgo`
  - Restored original recursive factorial body: `output :n * fact :n-1`.
  - Restored original Hanoi recursive calls using `:number-1`.

During the audit, restoring omitted-step `FOR` exposed a real control-library grouping bug. `FOR` now parenthesizes `ITEM` calls in its comparisons and `__FORLOOP` invocations so the original CSLS forms run correctly.

## Non-verbatim fixtures retained intentionally

Most fixtures in `tests/csls/` are direct or near-verbatim executable examples from the cited CSLS chapter URLs. Two sidecar fixtures remain intentionally modeled rather than verbatim:

- `crates/dynalogo-core/tests/csls_graphics/v1_ch10_square_label_fill.lgo`
  - Source URL: <https://people.eecs.berkeley.edu/~bh/v1ch10/turtle.html>
  - Rationale: compact deterministic drawing modeled on the chapter's turtle-geometry examples, suitable for asserting headless turtle state/events without a window.
- `crates/dynalogo-core/tests/csls_input/v2_ch01_scripted_read.lgo`
  - Source URL: <https://people.eecs.berkeley.edu/~bh/v2ch1/files.html>
  - Rationale: deterministic scripted input shape that exercises `READWORD`, `READLIST`, `READCHAR`, and `EOFP` without depending on an interactive terminal.

## Validation

`cargo test --test csls_examples` passed after the source restorations. The full suite was last run immediately before this audit task and passed; rerun the full suite before final delivery if more code changes follow.
