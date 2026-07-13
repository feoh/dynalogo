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
- `v2_ch05_program_as_data.lgo`
  - Restored two-input UCBLogo `DEFINE "name [[inputs] body...]` forms and `TEXT`-driven redefinition examples.
  - Note: calls to newly `DEFINE`d procedures are wrapped in `RUN [...]` in this whole-file fixture so DynaLOGO's batch parser can resolve arity after runtime definition; interactive CSLS sessions do not need that wrapper.
- `v2_ch08_property_lists.lgo`
  - Restored the book's family-tree property-list procedures using data-first `FOREACH`, `LOCALMAKE`, `CATCH`, `MAP.SE`, `FILTER`, and `REMOVE`.
- `v2_ch10_iteration_templates.lgo`
  - Restored data-first `FOREACH` argument order and the omitted `MAP.SE` contrast example.
- `v3_ch04_language_design.lgo`
  - Restored original recursive factorial body: `output :n * fact :n-1`.
  - Restored original Hanoi recursive calls using `:number-1`.

During the audit, restoring omitted-step `FOR`, UCBLogo `DEFINE`/`TEXT`, and family-tree examples exposed real compatibility gaps. `FOR` now parenthesizes `ITEM` calls in its comparisons and `__FORLOOP` invocations, `TEXT` now returns UCBLogo-style `[[inputs] body...]` while `FULLTEXT` retains full source lines, `DEFINE` accepts both UCBLogo two-input text lists and the existing three-input form, and `REMOVE` is available for CSLS list/word filtering.

## Non-verbatim fixtures retained intentionally

Most fixtures in `tests/csls/` are direct or near-verbatim executable examples from the cited CSLS chapter URLs. Two sidecar fixtures remain intentionally modeled rather than verbatim:

- `crates/dynalogo-core/tests/csls_graphics/v1_ch10_square_label_fill.lgo`
  - Source URL: <https://people.eecs.berkeley.edu/~bh/v1ch10/turtle.html>
  - Rationale: compact deterministic drawing modeled on the chapter's turtle-geometry examples, suitable for asserting headless turtle state/events without a window.
- `crates/dynalogo-core/tests/csls_input/v2_ch01_scripted_read.lgo`
  - Source URL: <https://people.eecs.berkeley.edu/~bh/v2ch1/files.html>
  - Rationale: deterministic scripted input shape that exercises `READWORD`, `READLIST`, `READCHAR`, and `EOFP` without depending on an interactive terminal.

## Validation

`cargo test --test csls_examples` passed after the source restorations. The full `cargo test` suite also passed after the additional fixture rewrites and compatibility fixes.
