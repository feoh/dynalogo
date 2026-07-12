# Primitive gap list

This is a current implementation gap list derived from the roadmap, Atari LOGO
validation notes, UCBLogo compatibility audit, and the closed-task verification
passes. It focuses on user-visible gaps that are still intentionally unfinished
or not fully verifiable in this environment.

## Workspace management still missing

Implemented today:

- `DEFINEDP` / `DEFINED?`
- `PRIMITIVEP` / `PRIMITIVE?`
- `TEXT`
- `FULLTEXT`
- `COPYDEF`
- `PO`, `POALL`, `PONS`, `POPS`, `POTS`, `POPLS`, `.PRIMITIVES`
- `ERASE` / `ER`, `ERN`, `ERNS`, `ERPS`, `ERPL`, `ERALL`
- `BURY`, `UNBURY`, `BURIEDP`
- `NODES`, `RECYCLE` (honest-limits implementation: `NODES` reports live
  workspace object counts rather than allocator statistics since DynaLOGO has
  no fixed node pool; `RECYCLE` is a documented no-op since Rust reclaims
  memory automatically and there is nothing for it to manually free)

The nearby workspace roadmap bucket is now mostly covered. Shape data is
implemented through `PUTSH`/`GETSH`/`SHAPE`, the browser demo has a shape-editor
panel, and `EDSH` opens the existing `$EDITOR` flow on shape definitions by
rendering them as editable `PUTSH` commands.

## File and stream status

Implemented on the current integration line:

- `LOAD`, `SAVE`
- `SETREAD`, `SETWRITE`
- `OPENREAD`, `OPENWRITE`, `OPENAPPEND`, `CLOSE`
- `READER`, `WRITER`
- `DRIBBLE`, `NODRIBBLE`
- `READCHAR`, `READLIST`, `READWORD`
- `EDIT`/`ED` via `$EDITOR`

The remaining limitation here is frontend/platform support, not missing native
stream primitives from the currently targeted surface. This whole file/stream
surface depends on `std::fs`, so it only works
in the native frontends. In the browser (WASM) build there is no filesystem,
so these primitives error instead of doing anything useful there. See
[`browser-demo.md`](browser-demo.md).

## Macro system

Implemented:

- `.MACRO` / `END` — like `TO`, but marks the procedure as a macro. The
  macro's own body runs like a normal procedure call (its own frame, must
  `OUTPUT` an instruction list). That list is then evaluated in place of the
  call, after the macro's frame is popped, so it runs in the caller's
  dynamic scope rather than the macro's own. If the macro call is used in an
  expression position, the expansion is evaluated like `RUNRESULT`
  (a trailing bare expression, or `OUTPUT`, supplies the value); if used as
  a command, it's evaluated like `RUN` (so e.g. `STOP` inside the expansion
  stops the calling procedure).
- `.DEFMACRO name paramsList bodyLinesList` — like `DEFINE`, but builds a
  macro from data instead of parsing `.MACRO`/`END` text.
- `MACROP` / `MACRO?` — true if the input names a currently defined macro.
- `MACROEXPAND instructionlist` — given a list whose first word names a
  macro, calls that macro with the remaining items as its inputs and
  outputs the one-step expansion *without* running it.
- `COPYDEF` preserves the macro/procedure distinction of the source.

Not yet covered: full syntactic (compile-time) macro expansion — macros are
expanded at call time here, not spliced into the surrounding bytecode chunk,
so a macro invocation is always dispatched dynamically rather than inlined.

## Template/control status

Current template and library-control support now includes:

- full template forms across `APPLY`/`FOREACH`/`MAP`/`FILTER`/`REDUCE`
- `CASCADE`
- `CASCADE.2`
- `TRANSFER`
- preserved literal-word reserialization for instruction-list templates

No concrete template/control gap is currently tracked from this file. Future
work should be driven by specific UCBLogo compatibility cases rather than this
broad bucket.

## Graphics/library status

Static turtle graphics are usable, and the current integration line includes
`LABEL` / `SETLABELHEIGHT`, `FILL` / `FILLED` seed-event helpers with native
software flood-fill rendering, multi-pen color selection (`PN` / `SETPN` /
`PC` / `SETPC`), and pen-mode state/reporting (`PEN`, `PE`, `PX`). The known
remaining rendering limitation is true per-pixel XOR compositing for `PX`:
DynaLOGO tracks reverse-pen state but renders reverse segments like `PD` unless
a future backend adds real raster inversion.

## Dynaturtle status

The classic dynaturtle-facing language layer is now exposed with:

- `TELL`, `ASK`, `EACH`, `WHO`
- `SETSPEED`, `SETVELOCITY`, `SETSHAPE`, `SPEED`, `SHAPE`
- `PUTSH`, `GETSH` as an initial shape registry layer
- `BOUNCE`, `WRAP`, `FENCE`, `WINDOW`
- `TOUCHING`, `WHEN`, `TOOT`

No concrete dynaturtle primitive is missing from the current documented surface
in this file. Future collision/event work should be tracked by specific failing
manual-comparison cases rather than the earlier broad "polish" bucket.

## Error parity remains incomplete

Recent work improved several important semantics:

- `not enough inputs to X`
- `You don't say what to do with X`
- `CATCH "ERROR`
- last-error reporting through `ERROR`
- `X doesn't like Y as input` (error code 4) for `FIRST`/`LAST`/`FPUT`/`LPUT`/
  `RANPICK` wrong-type and empty-collection inputs, plus empty-word `FIRST`/
  `RANPICK`/`ASCII` cases

Still pending is a broader pass to match UCBLogo wording/numbering, plus any
remaining edge cases surfaced by the compatibility suite and manual audit. The
shared numeric-input family now threads primitive-name context through its
callers, `SETITEM` index failures use the same code-4 wording, and `REDUCE` on
an empty list now reports `reduce doesn't like [] as input`. See
[`ucblogo-error-audit.md`](ucblogo-error-audit.md) for the remaining
site-by-site wording gaps.
