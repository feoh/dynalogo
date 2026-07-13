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
- `SETC`, `SETBG`, `SETSP`, `CT`
- true `PX` reverse/XOR compositing in the software raster canvas; vector trace renderers preserve pen mode while raster snapshots validate pixel behavior

**Implemented on the current DynaLOGO surface**

- `ASK`, `TELL`, `EACH`, `WHO`
- `OVER`, `TOUCHING`, and `WHEN` event/collision conditions
- `SPEED`, `LABEL`, `FILL`, `FILLED`, and `SETSCRUNCH`/`SETSCR`
- `FS`, `SS`, and `TS` screen-mode commands that record the requested frontend screen mode in VM state
- `SETSHAPE`/`SHAPE`/`GETSH`/`PUTSH` registry behavior plus native/browser custom-outline rendering

**Still intentionally narrower than Atari LOGO**

- Atari's original full-screen native editor is not reproduced byte-for-byte. DynaLOGO uses a modern `$EDITOR`-driven flow for procedure/name/shape/source editing and records screen-mode/cursor requests for frontends instead of emulating Atari display-list behavior.

### 2. Words and lists

**Implemented now**

- `FIRST`, `BUTFIRST`/`BF`, `LAST`, `BUTLAST`/`BL`
- `FPUT`, `LPUT`, `LIST`, `WORD`, `COUNT`, `ITEM`, `WHICH`, `MEMBERP`
- `EMPTYP`, `EQUALP`

**Implemented now**

- `LISTP`, `WORDP`, `REALWORDP`
- `RANK`, `RANPICK`
- text/character helpers visible in Atari vocabulary such as `ASCII`, `CHAR`, `LOWERCASE`, `REV`

**Remaining Atari vocabulary work**

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

**Section status**

- No unimplemented appendix helper from this validation section remains in the current audited surface.

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

**Remaining Atari vocabulary work**

- any remaining Atari numeric helpers beyond the core predicate/math surface above

### 5. Defining and editing procedures

**Implemented now**

- `TO ... END`
- recursion and dynamic scope
- `TEXT`, `FULLTEXT`, `COPYDEF`, `DEFINE`
- `POTS`

**Editor/screen fidelity audit**

- `EDIT`/`ED`, `EDNS`, and `EDSH` are available through the current `$EDITOR`-driven text-edit flow. The edited buffer is evaluated on editor exit, and tests cover source-file editing, blank edit buffers, variable editing, and shape-registry editing.
- `PUTSH`/`GETSH`/`SHAPE` provide a shape registry, the browser demo has a shape-editor panel, and `EDSH` opens the existing `$EDITOR` flow on shape definitions rendered as `PUTSH` commands.
- This is an intentional modern frontend substitution rather than a byte-for-byte reproduction of Atari LOGO's native full-screen editor. The remaining fidelity limitation is documented here instead of tracked as an implementation blocker.

### 6. Flow of control and conditionals

**Implemented now**

- `REPEAT`, `IF`, `IFELSE`, `STOP`, `OUTPUT`
- `TEST`, `IFTRUE`, `IFFALSE`
- library-level `FOR`, `WHILE`, `UNTIL`, `DO.WHILE`, `CASE`, `COND`

**Implemented or intentionally scoped differently**

- DynaLOGO implements `WHEN` with `TOUCHING`, `OVER`, `OVERCOLOR`, and edge conditions; Atari-specific condition numbering and `POD`/`PODS` are not part of the current surface.
- `FOREVER` is implemented. Other named Atari example/event helpers such as `HALT.AT` remain outside the current compatibility target.

### 7. Logical operations

**Implemented now**

- `AND`, `OR`, `NOT`
- `TRUE`, `FALSE` semantics through words/truthiness

**Implemented now**

- additional type predicates including `REALWORDP`

### 8. The outside world

**Implemented now**

- `READLIST`/`RL`, `READCHAR`/`RC`, and `READWORD`/`RW`
- `WAIT`, `KEYP`, joystick/paddle input (`JOY`, `JOYB`, `PADDLE`, `PADDLEB`)
- sound/environment helpers (`TOOT`, `SETENV`, `TIMEOUT`)
- cursor/screen helpers (`SETCURSOR`, `TEXTSCREEN`, `SPLITSCREEN`, `FULLSCREEN`)

**Editor/screen fidelity audit**

- `TEXTSCREEN`/`TS`, `SPLITSCREEN`/`SS`, `FULLSCREEN`/`FS`, and `SETCURSOR`/`SETPOSN` validate arguments and store the requested text-screen mode/cursor state in the VM for frontends and tests.
- `CT` clears the current VM output buffer.
- Native/browser frontends do not emulate Atari-specific display-list memory, cursor drawing, or physical keyboard/joystick timing. Those hardware details remain outside the current compatibility target.

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

**Section status**

- No unimplemented workspace item from this validation section remains in the current surface.

### 10. Files

**Implemented from the Atari manual surface**

- `LOAD`, `SAVE`
- `SETREAD`, `SETWRITE`
- `OPENREAD`, `OPENWRITE`, `OPENAPPEND`, `CLOSE`
- `READER`, `WRITER`
- file/device oriented `RC`, `RL`, `RW`
- `DRIBBLE`, `NODRIBBLE`

**Still intentionally narrower than Atari LOGO**

- `ERF` and `CATALOG` use the host filesystem deterministically. Atari printer/device namespaces and device-specific behavior are not emulated.

### 11. Special primitives

**Remaining Atari-only special primitives**

- `.DEPOSIT`, `EXAMINE`, likely `.CALL`, and related low-level/system primitives from the Atari glossary/index
- assorted hardware-facing commands tied to Atari devices and memory

## Highest-priority Atari-derived implementation gaps

These are the most important Atari-manual-driven gaps currently not exposed in
DynaLOGO:

1. **Low-level Atari-only special primitives**
   - `.DEPOSIT`, `EXAMINE`, likely `.CALL`, and related memory/system primitives remain intentionally outside the portable DynaLOGO runtime unless a future Atari-emulation target needs them.
2. **Device-specific fidelity**
   - printer/device namespaces, physical joystick/paddle timing, and Atari display-list/cursor rendering are represented by portable VM state or host services rather than emulated hardware.
3. **Remaining manual wording/parsing audit**
   - continue growing compatibility fixtures for edge-case parser wording, instruction-list errors, and manual examples as needed.

## Notes

- This pass is **Atari-manual-first**: it uses the Atari table of contents and
  glossary/index to identify actual Atari feature surface.
- DynaLOGO should continue using UCBLogo as a major compatibility target, but
  Atari LOGO introduces additional runtime, hardware, collision/event, and
  workspace behaviors that need separate tracking.
- Follow-up work should keep linking new tasks back to this manual-driven
  validation effort so missing Atari-only features do not get lost inside the
  broader UCBLogo parity work.
