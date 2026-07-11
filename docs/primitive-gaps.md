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

- `DEFINE`
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
