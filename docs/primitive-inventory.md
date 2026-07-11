# Primitive inventory snapshot

This document is a snapshot of the primitives and library-level procedures that
are currently implemented in DynaLOGO. It is intended as a working baseline for
manual comparison against the UCBLogo reference.

## Core control and evaluation

- `OUTPUT`, `OP`, `STOP`
- `REPEAT`, `REPCOUNT`
- `IF`, `IFELSE`, `RUN`, `RUNRESULT`
- `PARSE`, `RUNPARSE`, `APPLY`
- `TEST`, `IFTRUE`/`IFT`, `IFFALSE`/`IFF`
- `CATCH`, `THROW`, `ERROR`, `WAIT`
- `PAUSE`, `CONTINUE` currently exist but still return "not implemented yet"

## Library control structures

Loaded at VM startup as Logo procedures rather than Rust primitives:

- `FOR`
- `WHILE`
- `UNTIL`
- `DO.WHILE`
- `CASE`
- `COND`

## Templates and collection processing

- `FOREACH`
- `MAP`
- `FILTER`
- `REDUCE`

## Words, lists, arrays, and property lists

- `FIRST`, `BUTFIRST`/`BF`, `LAST`, `BUTLAST`/`BL`
- `FPUT`, `LPUT`, `SENTENCE`/`SE`, `LIST`, `WORD`
- `COUNT`, `ITEM`, `WHICH`, `EMPTYP`/`EMPTY?`, `EQUALP`/`EQUAL?`, `MEMBERP`/`MEMBER?`
- `BEFORE`, `INSERT`, `SORT`, `SUPERSORT`
- `WORDP`, `LISTP`, `NUMBERP`, `INTP`, `DECIMALP`
- `ARRAY`, `SETITEM`, `LISTTOARRAY`, `ARRAYTOLIST`
- `PPROP`, `GPROP`, `REMPROP`, `PLIST`

## Variables and workspace predicates

- `MAKE`, `NAME`, `THING`, `LOCAL`
- `NAMEP`
- `DEFINEDP`/`DEFINED?`
- `PRIMITIVEP`/`PRIMITIVE?`
- `TEXT`, `FULLTEXT`, `COPYDEF`, `DEFINE`
- `PO`, `POALL`, `PONS`, `POPS`, `POTS`, `.PRIMITIVES`
- `ERASE`/`ER`, `ERN`, `ERNS`, `ERPS`, `ERALL`

## Arithmetic, comparison, and logic

- `SUM`, `DIFFERENCE`, `PRODUCT`, `QUOTIENT`, `REMAINDER`
- `ABS`, `INT`, `ROUND`, `SQRT`, `SIN`, `COS`, `TAN`, `RANDOM`, `RERANDOM`
- Infix operators: `+`, `-`, `*`, `/`, `<`, `>`, `=`, `<=`, `>=`, `<>`
- `AND`, `OR`, `NOT`

## Console I/O

- `PRINT`/`PR`
- `SHOW`
- `TYPE`
- `READLIST`/`RL`

## Turtle graphics

- `FORWARD`/`FD`, `BACK`/`BK`
- `LEFT`/`LT`, `RIGHT`/`RT`
- `SETXY`, `SETX`, `SETY`, `SETPOS`, `SETHEADING`/`SETH`
- `HOME`, `CLEARSCREEN`/`CS`, `INIT.TURTLE`
- `PENUP`/`PU`, `PENDOWN`/`PD`
- `SETPENCOLOR`/`SETPC`, `SETPENSIZE`
- `HIDETURTLE`/`HT`, `SHOWTURTLE`/`ST`, `SHOWNP`
- `POS`, `HEADING`, `XCOR`, `YCOR`

## Notes

Not yet implemented from nearby roadmap/workspace tasks include remaining
workspace-management pieces such as bury/unbury behavior, file I/O (`LOAD`,
`SAVE`, streams), macros, richer graphics primitives like `LABEL`/`FILL`, and
the dynaturtle-specific runtime commands such as `TELL`, `ASK`, velocity, and
collision/event APIs. See also [primitive-gaps.md](primitive-gaps.md) for the
current first-pass gap list.
