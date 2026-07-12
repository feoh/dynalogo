# Atari LOGO validation (initial manual pass)

This document tracks DynaLOGO against the **Atari LOGO Reference Manual** as an
explicit compatibility target of its own, not merely as a comparison filtered
through UCBLogo.

Reference manual:

- <https://archive.org/details/AtariLOGOReferenceManual>

Working source used for this pass:

- table of contents (manual pages 3–6)
- glossary/index pages around manual pages 197–205 extracted from the PDF

## Validation scope

The Atari manual organizes features into these areas:

- Getting Started / Logo grammar
- Chapter 1: Turtle Graphics
- Chapter 2: Words and Lists
- Chapter 3: Variables
- Chapter 4: Arithmetic Operations
- Chapter 5: Defining and Editing Procedures
- Chapter 6: Flow of Control and Conditionals
- Chapter 7: Logical Operations
- Chapter 8: The Outside World
- Chapter 9: Workspace Management
- Chapter 10: Files
- Chapter 11: Special Primitives
- Appendix E: Parsing
- Appendix G/H: Vocabulary / glossary

## Syntax and grammar status

### Already aligned or mostly aligned

- `TO ... END` procedure definition
- quoted words, colon variables, list brackets
- infix arithmetic/comparison operators
- parentheses for grouped / greedy calls
- `REPEAT`, `IF`, `IFELSE`, `RUN`, `RUNRESULT`
- dynamic variables via `MAKE`, `THING`, `:name`

### Still needing explicit Atari validation or follow-up

- Atari distinction between commands and operations in all edge cases
- full parsing appendix behavior: delimiters, infix procedures, minus-sign rules
- Atari-specific wording and semantics around instruction lists and line parsing
- backslash/literal-character edge cases from the manual

## Feature-by-feature status by manual area

### 1. Turtle graphics

**Implemented now**

- `FORWARD`/`FD`, `BACK`/`BK`
- `LEFT`/`LT`, `RIGHT`/`RT`
- `HOME`, `POS`, `HEADING`, `XCOR`, `YCOR`
- `PENUP`/`PU`, `PENDOWN`/`PD`
- `SETPOS`, `SETXY`, `SETX`, `SETY`, `SETHEADING`/`SETH`
- `HIDETURTLE`/`HT`, `SHOWTURTLE`/`ST`, `SHOWNP`
- `SETPENCOLOR`/`SETPC`, `SETPENSIZE`
- `CLEARSCREEN`/`CS`
- multi-pen state: `PEN`, `PE`, `PX`, `PN`, `SETPN`, Atari-style `SETPC pennumber colornumber`
  (`PX`'s reverse/XOR pixel compositing is not implemented; see the
  reference manual's Pen and visibility section)

**Present in Atari manual but still missing or incomplete in DynaLOGO**

- `ASK`, `TELL`, `EACH`, `WHO`
- `OVER`, `TOUCHING`, `WHEN`, event/collision table semantics
- `SETSH`/`SETSHAPE`, `SHAPE`, `GETSH`, `PUTSH`, turtle shape editor behavior
- `SETSP` (Atari-style alias for `SETSPEED`)
- `SPEED`
- background/turtle color split: `SETBG`, `SETC`
- Atari screen-mode commands such as `FS`, `SS`, `TS`, `CT`
- graphics extras like `LABEL`, `FILL`, `SETSCR`

### 2. Words and lists

**Implemented now**

- `FIRST`, `BUTFIRST`/`BF`, `LAST`, `BUTLAST`/`BL`
- `FPUT`, `LPUT`, `LIST`, `WORD`, `COUNT`, `ITEM`, `WHICH`, `MEMBERP`
- `EMPTYP`, `EQUALP`

**Implemented now**

- `LISTP`, `WORDP`, `REALWORDP`
- `RANK`, `RANPICK`
- text/character helpers visible in Atari vocabulary such as `ASCII`, `CHAR`, `LOWERCASE`, `REV`

**Missing from Atari vocabulary**

- assorted Atari examples/helpers referenced in the glossary that are not yet audited individually

### 2b. Atari Useful Tools appendix

**Implemented now**

- `ABS` (now primitive-backed)
- `BEFORE`, `INSERT`, `SORT`, `SUPERSORT`
- `COPYDEF`
- `DEFINE`
- `DOT`
- `FOREVER`
- `INIT.TURTLE`
- `TEXT`
- `WHICH`

**Still missing from the appendix examples/tools**

- `READLINE` as used by Atari's file-backed TEXT example

### 3. Variables

**Implemented now**

- `MAKE`, `THING`, `LOCAL`

**Implemented now**

- `NAMEP`

**Implemented now**

- broader Atari variable/workspace listing and erase helpers now covered in part by
  `PONS`, `ERN`, and `ERNS`

### 4. Arithmetic operations

**Implemented now**

- `SUM`, `DIFFERENCE`, `PRODUCT`, `QUOTIENT`, `REMAINDER`
- infix `+ - * / < > = <= >= <>`

**Implemented now**

- `ABS`, `INT`, `ROUND`, `SQRT`, `RANDOM`, `RERANDOM`
- `SIN`, `COS`, `TAN`
- `INTP`, `DECIMALP`, `NUMBERP`
- `EVENP`, `DIVISORP`, `FACTORIAL`

**Missing from Atari vocabulary**

- any remaining Atari numeric helpers beyond the core predicate/math surface above

### 5. Defining and editing procedures

**Implemented now**

- `TO ... END`
- recursion and dynamic scope
- `TEXT`, `FULLTEXT`, `COPYDEF`, `DEFINE`
- `POTS`

**Partial / follow-up status**

- `EDIT`/`ED` and `EDNS` are available through the current `$EDITOR`-driven
  text-edit flow
- `EDSH` currently reports that the shape registry/editor work is not yet
  implemented
- Atari editor/screen behavior itself is still not reproduced

### 6. Flow of control and conditionals

**Implemented now**

- `REPEAT`, `IF`, `IFELSE`, `STOP`, `OUTPUT`
- `TEST`, `IFTRUE`, `IFFALSE`
- library-level `FOR`, `WHILE`, `UNTIL`, `DO.WHILE`, `CASE`, `COND`

**Present in Atari manual but still missing or only partial**

- full Atari `WHEN` demon/event model
- Atari collision/event condition numbering and `POD`/`PODS`
- some named example/event helpers from the glossary (`FOREVER`, `HALT.AT`, etc.) are not implemented

### 7. Logical operations

**Implemented now**

- `AND`, `OR`, `NOT`
- `TRUE`, `FALSE` semantics through words/truthiness

**Implemented now**

- additional type predicates including `REALWORDP`

### 8. The outside world

**Implemented now**

- `READLIST`/`RL` exists, but is not connected to a real stream yet
- `WAIT`

**Present in Atari manual but missing**

- `RC` / read-char behavior
- `KEYP`
- joystick/paddle input: `JOY`, `JOYB`, `PADDLE`, `PADDLEB`
- sound: `TOOT`, `SETENV`, `TIMEOUT`
- cursor/screen helpers such as `SETCURSOR`, `TEXTSCREEN`, `SPLITSCREEN`, `FULLSCREEN`

### 9. Workspace management

**Implemented now**

- `DEFINEDP` / `DEFINED?`
- `PRIMITIVEP` / `PRIMITIVE?`
- `TEXT`, `FULLTEXT`, `COPYDEF`
- `PO`, `POALL`, `POPS`, `PONS`, `POTS`, `.PRIMITIVES`
- `ERASE` / `ER`, `ERALL`, `ERN`, `ERNS`, `ERPS`
- `BURY`, `UNBURY`, `BURIEDP`
- `DEFINE`
- `NODES`, `RECYCLE` (honest-limits: `NODES` reports live workspace object
  counts rather than allocator statistics, since DynaLOGO has no fixed node
  pool; `RECYCLE` is a documented no-op since Rust reclaims memory
  automatically)

**Present in Atari manual but missing**

- none remaining in this section

### 10. Files

**Implemented from the Atari manual surface**

- `LOAD`, `SAVE`
- `SETREAD`, `SETWRITE`
- `OPENREAD`, `OPENWRITE`, `OPENAPPEND`, `CLOSE`
- `READER`, `WRITER`
- file/device oriented `RC`, `RL`, `RW`
- `DRIBBLE`, `NODRIBBLE`

**Still missing**

- `ERF`, `CATALOG`, printer/file device handling

### 11. Special primitives

**Present in Atari manual but missing**

- `.DEPOSIT`, `EXAMINE`
- likely `.CALL` and related low-level/system primitives from the Atari glossary/index
- assorted hardware-facing commands tied to Atari devices and memory

## Highest-priority Atari-derived implementation gaps

These are the most important Atari-manual-driven gaps currently not exposed in
DynaLOGO:

1. **Turtle addressing and event primitives**
   - `ASK`, `TELL`, `EACH`, `WHO`, `OVER`, `TOUCHING`, `WHEN`
2. **Remaining workspace management work**
   - editor integration (`NODES`, `RECYCLE`, and bury/unbury are now implemented)
3. **File and device follow-through**
   - printer/catalog/device-specific surface beyond `LOAD`/`SAVE`/streams
4. **Remaining Atari type/text helper audit**
   - verify any adjacent helper surface beyond `REALWORDP`, `RANK`, `RANPICK`, `EVENP`, `DIVISORP`, `FACTORIAL`, `ASCII`, `CHAR`, `LOWERCASE`, and `REV`
5. **Graphics/screen extras**
   - `SETBG`, `SETC`, `SETX`, `SETY`, `SHAPE`, `SETSH`, `GETSH`, `PUTSH`, `SETSP`
   - `PX`'s reverse/XOR pixel compositing (needs a persistent raster canvas the current vector event-replay renderers don't have)
6. **Remaining Atari outside-world features**
   - any deeper printer/device hooks beyond `KEYP`, joystick/paddle input, `TOOT`, `SETENV`, and cursor/text-screen primitives already integrated

## Notes

- This pass is **Atari-manual-first**: it uses the Atari table of contents and
  glossary/index to identify actual Atari feature surface.
- DynaLOGO should continue using UCBLogo as a major compatibility target, but
  Atari LOGO introduces additional runtime, hardware, collision/event, and
  workspace behaviors that need separate tracking.
- Follow-up work should keep linking new tasks back to this manual-driven
  validation effort so missing Atari-only features do not get lost inside the
  broader UCBLogo parity work.
