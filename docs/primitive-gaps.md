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
- `PO`, `POALL`, `PONS`, `POPS`, `POTS`, `POPLS`, `.PRIMITIVES`
- `ERASE` / `ER`, `ERN`, `ERNS`, `ERPS`, `ERPL`, `ERALL`
- `BURY`, `UNBURY`, `BURIEDP`

Still missing from the nearby workspace roadmap bucket:

- `NODES`, `RECYCLE`
- the broader editor-family follow-ups such as `EDNS`, `EDSH`

## File and stream follow-up gaps

Implemented on the current integration line:

- `LOAD`, `SAVE`
- `SETREAD`, `SETWRITE`
- `OPENREAD`, `OPENWRITE`, `OPENAPPEND`, `CLOSE`
- `READER`, `WRITER`
- `DRIBBLE`, `NODRIBBLE`
- `READCHAR`, `READLIST`, `READWORD`
- `EDIT`/`ED` via `$EDITOR`

Still missing in the broader file/device area are Atari- and UCBLogo-adjacent
commands beyond this core stream surface.

## Macro system is still absent

The roadmap still calls out these missing macro features:

- `.MACRO`
- `.DEFMACRO`
- `MACROP`
- `MACROEXPAND`

## Template/control follow-up remains

Current template and library-control support now includes:

- full template forms across `APPLY`/`FOREACH`/`MAP`/`FILTER`/`REDUCE`
- `CASCADE`
- `CASCADE.2`
- `TRANSFER`
- preserved literal-word reserialization for instruction-list templates

Remaining work is mainly any additional UCBLogo-specific template edge cases
found during audit.

## Graphics/library gaps remain

Static turtle graphics are usable, but the remaining parity/polish bucket still
includes:

- `LABEL`
- `SETLABELHEIGHT`
- `FILL`
- multiple pens and related graphics polish

## Dynaturtle follow-up surface remains

The classic dynaturtle-facing language layer is now exposed with:

- `TELL`, `ASK`, `EACH`, `WHO`
- `SETSPEED`, `SETVELOCITY`, `SETSHAPE`
- `TOUCHING`, `WHEN`, `TOOT`

Still missing from the broader dynaturtle roadmap are:

- edge/window modes such as `BOUNCE`, `WRAP`, `FENCE`, `WINDOW`
- compatibility helpers such as a `SPEED` query primitive
- any remaining collision/event polish found during manual comparison

## Error parity remains incomplete

Recent work improved several important semantics:

- `not enough inputs to X`
- `You don't say what to do with X`
- `CATCH "ERROR`
- last-error reporting through `ERROR`

Still pending is a broader pass to match UCBLogo wording/numbering, plus any
remaining edge cases surfaced by the compatibility suite and manual audit.
