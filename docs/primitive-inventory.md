# Primitive inventory snapshot

This document is a snapshot of the primitives and library-level procedures that
are currently implemented in DynaLOGO. It is intended as a working baseline for
manual comparison against the UCBLogo reference.

## Core control and evaluation

- `OUTPUT`, `OP`, `STOP`
- `REPEAT`, `REPCOUNT`
- `IF`, `IFELSE`, `RUN`, `RUNRESULT`
- `PARSE`, `RUNPARSE`, `APPLY`
- `CASCADE`, `CASCADE.2`, `TRANSFER`
- `TEST`, `IFTRUE`/`IFT`, `IFFALSE`/`IFF`
- `CATCH`, `THROW`, `ERROR`, `WAIT`
- `PAUSE`, `CONTINUE` with interactive pause-loop / resume semantics

## Library control structures

Loaded at VM startup as Logo procedures rather than Rust primitives:

- `FOR`
- `WHILE`
- `UNTIL`
- `DO.WHILE`
- `CASE`
- `COND`

## Templates and collection processing

- `APPLY`
- `FOREACH`
- `MAP`
- `FILTER`
- `REDUCE`
- `CASCADE`, `CASCADE.2`, `TRANSFER`
- implicit-slot templates (`?`, `?1`, `?2`, ...)
- named-slot templates (`[:x :y] [...]`)
- procedure-name templates
- `PARSE`, `RUNPARSE`, `RUNRESULT`

## Words, lists, arrays, and property lists

- `FIRST`, `BUTFIRST`/`BF`, `LAST`, `BUTLAST`/`BL`
- `FPUT`, `LPUT`, `SENTENCE`/`SE`, `LIST`, `WORD`
- `COUNT`, `ITEM`, `RANK`, `RANPICK`, `WHICH`
- `EMPTYP`/`EMPTY?`, `EQUALP`/`EQUAL?`, `MEMBERP`/`MEMBER?`
- `BEFORE`, `INSERT`, `SORT`, `SUPERSORT`, `ASCII`, `CHAR`, `LOWERCASE`, `REV`
- `WORDP`, `REALWORDP`, `LISTP`, `NUMBERP`, `INTP`, `DECIMALP`, `EVENP`, `DIVISORP`
- `ARRAY`, `SETITEM`, `LISTTOARRAY`, `ARRAYTOLIST`
- `PPROP`, `GPROP`, `REMPROP`, `PLIST`

## Variables and workspace predicates

- `MAKE`, `NAME`, `THING`, `LOCAL`
- `EDIT`/`ED`
- `NAMEP`
- `DEFINEDP`/`DEFINED?`
- `PRIMITIVEP`/`PRIMITIVE?`
- `TEXT`, `FULLTEXT`, `COPYDEF`, `DEFINE`, `.DEFMACRO`
- `MACROP`/`MACRO?`, `MACROEXPAND`
- `PO`, `POALL`, `PONS`, `POPS`, `POTS`, `POPLS`, `.PRIMITIVES`
- `ERASE`/`ER`, `ERN`, `ERNS`, `ERPS`, `ERPL`, `ERALL`
- `BURY`, `UNBURY`, `BURIEDP`

## Arithmetic, comparison, and logic

- `SUM`, `DIFFERENCE`, `PRODUCT`, `QUOTIENT`, `REMAINDER`
- `ABS`, `INT`, `ROUND`, `SQRT`, `SIN`, `COS`, `TAN`, `RANDOM`, `RERANDOM`, `FACTORIAL`
- Infix operators: `+`, `-`, `*`, `/`, `<`, `>`, `=`, `<=`, `>=`, `<>`
- `AND`, `OR`, `NOT`

## Console I/O and outside-world helpers

- `PRINT`/`PR`
- `SHOW`
- `TYPE`
- `LOAD`, `SAVE`
- `SETREAD`, `SETWRITE`
- `OPENREAD`, `OPENWRITE`, `OPENAPPEND`, `CLOSE`
- `READER`, `WRITER`
- `DRIBBLE`, `NODRIBBLE`
- `READCHAR`/`RC`, `READLIST`/`RL`, `READWORD`/`RW`
- `KEYP`
- `JOY`, `JOYB`, `PADDLE`, `PADDLEB`
- `TIMEOUT`
- `TEXTSCREEN`/`TS`, `SPLITSCREEN`/`SS`, `FULLSCREEN`/`FS`
- `SETCURSOR`, `SETENV`

## Turtle graphics and dynaturtles

- `FORWARD`/`FD`, `BACK`/`BK`
- `LEFT`/`LT`, `RIGHT`/`RT`
- `SETXY`, `SETX`, `SETY`, `SETPOS`, `SETHEADING`/`SETH`
- `HOME`, `CLEARSCREEN`/`CS`, `INIT.TURTLE`
- `PENUP`/`PU`, `PENDOWN`/`PD`
- `SETPENCOLOR`/`SETPC`, `SETPENSIZE`
- `SETLABELHEIGHT`, `LABEL`, `FILL`
- `HIDETURTLE`/`HT`, `SHOWTURTLE`/`ST`, `SHOWNP`
- `POS`, `HEADING`, `XCOR`, `YCOR`
- `TELL`, `ASK`, `EACH`, `WHO`
- `SETVELOCITY`, `SETSPEED`, `SETSHAPE`
- `TOUCHING`, `WHEN`, `TOOT`

## Notes

Not yet implemented from nearby roadmap/workspace tasks include remaining
workspace-management pieces such as `NODES`/`RECYCLE`, richer graphics
primitives like `FILL`, and dynaturtle follow-ups such as edge/window modes
and additional compatibility polish. See also
[primitive-gaps.md](primitive-gaps.md) for the current gap list.
