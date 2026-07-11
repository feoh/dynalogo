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
- `EDIT` / `ED` — shells out to `$EDITOR` on a contentslist (procedure names,
  or the 3-list `[[procs] [vars] [plists]]` form) and reloads the edited text
  on exit. Note: the fixed-arity table means the bare 0-input "re-edit last
  buffer" form must be written `(EDIT)` with parens; `EDIT`/`ED` alone always
  expects exactly one contents argument.

Still missing from the nearby workspace roadmap bucket:

- `NODES`, `RECYCLE`
- `BURY`, `UNBURY`, `BURIEDP`
- `EDNS`, `EDSH`

## File I/O still has follow-up gaps

Core file/stream support is now implemented:

- `LOAD`, `SAVE`
- `SETREAD`, `SETWRITE`
- `READCHAR`/`RC`
- `READLIST`/`RL` via the active read stream

Still missing from the wider file/device roadmap bucket:

- `OPENREAD`, `OPENWRITE`
- `READWORD`
- `DRIBBLE`

## Macro system is still absent

The roadmap still calls out these missing macro features:

- `.MACRO`
- `.DEFMACRO`
- `MACROP`
- `MACROEXPAND`

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
