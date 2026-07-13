# CSLS/UCBLogo conformance specification

This specification is the phase-exit plan for the `dynalogo conformance`
workflow project. It turns the discovery gaps in
[`csls-conformance-discovery.md`](csls-conformance-discovery.md) into concrete
acceptance criteria for the implementation phase.

## Compatibility target

DynaLOGO's conformance target is **zero UCBLogo semantic differences** for the
Computer Science Logo Style (CSLS) corpus. A behavior difference from UCBLogo is
a bug or unsupported gap unless it is an explicitly additive DynaLOGO extension.

Additive extension examples, such as dynaturtles, may keep their own fixtures,
but they must not be used to relax UCBLogo-compatible CSLS expectations.

## Corpus and fixture policy

- Keep UCBLogo/CSLS-compatible fixtures separate from DynaLOGO-only extension
  fixtures.
- Every imported CSLS fixture must retain source attribution in a nearby comment
  or sidecar note: CSLS volume, chapter/section, procedure or example name, and
  whether the committed expected output came from UCBLogo, the book text, or a
  documented manual derivation.
- Prefer original book syntax. If a fixture must be temporarily adapted to pass,
  mark the adaptation in the fixture and track it as blocked follow-up work; the
  final conformance corpus should not depend on DynaLOGO-specific rewrites.
- Keep committed `.out`/`.err` files as the required CI oracle. Live UCBLogo
  comparison is optional and should be used when `$UCBLOGO_BIN`, `ucblogo`, or
  `logo` is available, but CI must still be useful without that binary.
- Import examples in layers: pure deterministic output first, then scripted
  interactive input and filesystem examples, then deterministic turtle/graphics
  examples, and finally full applications/distribution examples.

## Implementation-phase task criteria

### Parser compatibility for CSLS expression forms

Task: `tk-expand-parser-compatibility-for-csls-expression--b5cfcf`

Acceptance criteria:

- Parenthesized expression grouping such as `(:x + 1)` is accepted where
  UCBLogo accepts it, without regressing existing parenthesized procedure calls
  such as `(sentence ...)` and `(list ...)`.
- Compact subtraction and negative-number forms already covered by the current
  fixture set remain passing.
- `TO` headers support the CSLS/UCBLogo procedure-input forms needed by the
  corpus, including optional/default/rest inputs when encountered.
- New regression fixtures cover every parser form added, including at least one
  fixture copied from or directly modeled on CSLS text.

### UCBLogo-compatible template iteration semantics

Task: `tk-implement-ucblogo-compatible-template-iteration--25505c`

Acceptance criteria:

- `MAP`, `FOREACH`, `FILTER`, `REDUCE`, `APPLY`, `CASCADE`, `CASCADE.2`, and
  `TRANSFER` keep current fixture behavior while matching UCBLogo for the CSLS
  template cases.
- Multi-list `MAP`/`FOREACH` bind `?1`, `?2`, ... to parallel input elements
  with UCBLogo-compatible arity and length behavior.
- Word inputs, procedure-name templates, explicit-slot templates, implicit-slot
  templates, reporter/effect templates, and `#`/index binding are covered.
- `MAP.SE` is implemented or otherwise explicitly handled as the UCBLogo
  primitive expected by CSLS examples.

### Missing CSLS/UCBLogo primitives and aliases

Task: `tk-add-missing-csls-ucblogo-primitives-and-aliases-d9ab29`

Acceptance criteria:

- Implement or alias each primitive needed by imported CSLS examples before the
  examples land in the conformance corpus.
- The first required batch is: `LOCALMAKE`, `MEMBER` as a tail reporter,
  `FIND`, `MAP.SE`, `QUEUE`, `PUSH`, `POP`, `BOUNDP`, `SUBSTRINGP`,
  `PROCEDURES`, `EOFP`, and cursor-position helpers such as `SETPOSN` where
  applicable.
- Each primitive has focused Rust or fixture coverage for UCBLogo-compatible
  success behavior and representative error behavior.
- `docs/primitive-inventory.md` is updated in the same change that adds or
  removes primitives from this list.

### Full CSLS example corpus support

Task: `tk-support-full-computer-science-logo-style-example-9ce399`

Acceptance criteria:

- The current curated fixture set remains passing.
- The corpus is expanded chapter-by-chapter with attribution and canonical
  output/error files.
- Examples that require not-yet-implemented parser/template/primitive/file/input
  or graphics behavior are not silently adapted; they are either blocked by the
  appropriate implementation task or marked with an explicit temporary
  adaptation note.
- Full application examples are promoted to distribution examples only after
  their dependencies have deterministic test coverage.

### Mutable list/cell operations used by CSLS data structures

Task: `tk-support-mutable-list-cell-operations-used-by-csl-47a17b`

Acceptance criteria:

- Mutable list or cell operations required by CSLS examples behave like UCBLogo
  for aliasing and mutation visibility.
- Tests include both direct primitive behavior and at least one CSLS data
  structure example that depends on mutation.
- Interactions with existing immutable list operations (`FPUT`, `LPUT`,
  `SENTENCE`, `LIST`, `ITEM`, `SETITEM`) are documented and regression-tested.

### Filesystem and input harnesses

Tasks:

- `tk-complete-filesystem-input-primitives-needed-by-c-ec449a`
- `tk-build-a-scripted-interactive-input-harness-for-c-c7dc73`

Acceptance criteria:

- File examples run inside a deterministic temporary workspace and do not depend
  on the developer's current directory or host files.
- Interactive examples can be driven by scripted input in tests, including EOF
  and empty-input cases.
- `LOAD`, `SAVE`, stream primitives, `READWORD`, `READLIST`, `READCHAR`, `KEYP`,
  and `EOFP` are covered as needed by CSLS examples.
- Test output normalization is documented so expected files remain stable across
  platforms.

Implementation note: scripted interactive examples live under
`crates/dynalogo-core/tests/csls_input/` as `.lgo` sources paired with `.in`
input scripts and `.out` stdout oracles. The integration harness uses
`Vm::set_scripted_input` so `READWORD`, `READLIST`, `READCHAR`, and `EOFP`
execute deterministically without reading from the host terminal.

### Deterministic graphics/turtle assertions

Task: `tk-add-deterministic-graphics-turtle-assertions-for-csls-geometry-examples-dcd34d`

Acceptance criteria:

- Geometry examples assert deterministic turtle state and drawing events rather
  than relying on visual inspection.
- Tests pin positions, headings, pen state, color/size where relevant, and a
  normalized event trace for drawn segments.
- Graphics assertions remain backend-independent and run in headless CI.

Implementation note: graphics-only CSLS fixtures live under
`crates/dynalogo-core/tests/csls_graphics/` and are paired with `.trace` files
checked by `crates/dynalogo-core/tests/csls_graphics.rs`. The trace oracle pins
the final turtle state plus normalized clear/line/label/fill events so geometry
examples can run in CI without a rendering backend.

### Macro and quasiquote compatibility

Task: `tk-implement-csls-macro-quasiquote-compatibility-7e2466`

Acceptance criteria:

- CSLS macro examples using `.DEFMACRO`, `MACROEXPAND`, quasiquote-like list
  construction, and program-as-data idioms run with UCBLogo-compatible expansion
  semantics.
- Macro expansion preserves literal words that shadow primitive names and keeps
  source reserialization stable enough for `TEXT`/`PO` examples.
- Fixtures cover both expansion output and end-to-end execution of expanded
  code.

## Project phase exit gate

The spec phase is complete when this document is committed, the project is
advanced to implementation, and implementation work follows the task order above
unless a task is deliberately split or blocked with a recorded reason.
