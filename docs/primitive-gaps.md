# Primitive gap list (initial pass)

This is a first-pass gap list derived from the current implementation snapshot
and roadmap, not yet a full audit against the UCBLogo manual.

## Workspace management still missing

Implemented today:

- `DEFINEDP` / `DEFINED?`
- `PRIMITIVEP` / `PRIMITIVE?`
- `TEXT`
- `FULLTEXT`
- `COPYDEF`
- `PO`, `POALL`, `PONS`, `POPS`, `POTS`, `.PRIMITIVES`
- `ERASE` / `ER`, `ERN`, `ERNS`, `ERPS`, `ERALL`

Still missing from the nearby workspace roadmap bucket:

- `NODES`, `RECYCLE`
- `BURY`, `UNBURY`, `BURIEDP`
- editor-facing commands such as `EDIT`/`ED`, `EDNS`, `EDSH`

## File I/O is still absent

No user-facing file or stream primitives are implemented yet:

- `LOAD`, `SAVE`
- `OPENREAD`, `OPENWRITE`
- `READWORD`, `READCHAR`
- stream plumbing and `DRIBBLE`
- `EDIT` integration with `$EDITOR`

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

## Template/control follow-up remains

Current template and library-control support is solid, but roadmap follow-up
still remains for:

- full template forms
- `CASCADE`
- `TRANSFER`
- any additional UCBLogo-specific template edge cases found during audit

## Graphics/library gaps remain

Static turtle graphics are usable, but the remaining parity/polish bucket still
includes:

- `LABEL`
- `SETLABELHEIGHT`
- `FILL`
- multiple pens and related graphics polish

## Dynaturtle command surface is not yet exposed

The engine groundwork exists, but the classic dynaturtle-facing language layer
still needs user-visible commands such as:

- `TELL`, `ASK`, `EACH`, `WHO`
- `SETSPEED`, `SPEED`, `SETVELOCITY`
- `TOUCHING`
- `WHEN` event/demon surface
- `BOUNCE`, `WRAP`, `FENCE`, `WINDOW`, `SETSHAPE`

## Error parity remains incomplete

Recent work improved several important semantics:

- `not enough inputs to X`
- `You don't say what to do with X`
- `CATCH "ERROR`
- last-error reporting through `ERROR`

Still pending is a broader pass to match UCBLogo wording/numbering and any
remaining edge cases from the manual.
