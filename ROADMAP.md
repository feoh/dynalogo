# DynaLOGO Roadmap

This is a long-term compatibility roadmap, not a list of currently assigned
work. The current implementation snapshot and concrete limitations are
maintained in the README and compatibility documentation.

## v0.1 — "The turtle crawls" (core language + static turtle graphics)

- Lexer/reader: UCBLogo tokenization — words, `[lists]`, `(parens)`,
  `"quoted` words, `:dots`, `;comments`, `~` continuation, infix operators
- Parser to instruction lists (homoiconic), arity-driven expression grouping
- Bytecode compiler with per-procedure/list chunk cache, invalidated on
  redefinition
- Stack VM: dynamic scoping, tail-call optimization, `OUTPUT`/`STOP`
- `TO … END` procedure definition
- Control: `REPEAT IF IFELSE RUN REPCOUNT`
- Words/lists: `FIRST BUTFIRST LAST BUTLAST FPUT LPUT SENTENCE LIST WORD
  COUNT ITEM EMPTYP EQUALP MEMBERP`
- Arithmetic (prefix + infix), comparison, logic; `PRINT SHOW TYPE READLIST`
- Variables: `MAKE NAME THING LOCAL` and `:x`
- `TurtleBackend` trait + headless test harness
- Native window with static turtle graphics: `FORWARD BACK LEFT RIGHT SETXY
  SETPOS SETHEADING HOME CLEARSCREEN PENUP PENDOWN SETPENCOLOR SETPENSIZE
  HIDETURTLE SHOWTURTLE POS HEADING XCOR YCOR`
- Terminal REPL

## v0.2 — Dynaturtles

- Fixed 60 Hz simulation tick decoupled from rendering; interpolated drawing
- Simulation/VM thread + REPL channel — typing never freezes moving turtles
- Multiple turtles, struct-of-arrays storage: `TELL ASK EACH WHO`
- Velocity: `SETSPEED SPEED SETVELOCITY`, continuous motion
- Collision: spatial-hash broad phase; turtle/turtle, turtle/edge, and
  pen-line contact; `TOUCHING`
- `WHEN` demons with per-tick fuel budget; demon management
- Edge modes: `BOUNCE WRAP FENCE WINDOW`; `SETSHAPE`

## v0.3 — Rich language core

- Templates: `MAP FILTER REDUCE FOREACH APPLY` with `?` placeholders,
  anonymous procedures, `PARSE RUNPARSE RUNRESULT`
- `TEST IFTRUE IFFALSE`; `CATCH THROW ERROR`; `PAUSE CONTINUE WAIT`
- Property lists and arrays
- Library control structures: `FOR WHILE UNTIL DO.WHILE CASE COND`
- UCBLogo-accurate error messages

## v0.4 — Workspace & I/O

- Workspace management: `POALL PO POPS ERASE ERALL BURY UNBURY DEFINE TEXT
  FULLTEXT COPYDEF`
- File I/O: `LOAD SAVE OPENREAD OPENWRITE READWORD READCHAR`, streams,
  `DRIBBLE`
- `EDIT` via `$EDITOR`
- User docs + classic example programs (spaceship, bouncing ball, orbit)

## v0.5 — Macros & performance

- `.MACRO .DEFMACRO MACROP MACROEXPAND`
- Template/control edge-case audit after full template forms + `CASCADE`/`CASCADE.2`/`TRANSFER`
- Performance pass: interning audit, bytecode peephole, benchmarks —
  **1,000 colliding dynaturtles at 60 Hz**

## v0.6 — Web/WASM

- Core compiles to `wasm32-unknown-unknown`; cooperative sim scheduling
- Browser build with REPL panel; GitHub Pages demo

## v1.0 — Full UCBLogo library parity + polish

- Audit of all ~400 UCBLogo primitives against the manual
- Compatibility test suite run against UCBLogo behavior
- Remaining Atari/UCBLogo surface: low-level Atari-only special primitives such
  as `.DEPOSIT`, `EXAMINE`, `.CALL`, and related hardware/memory hooks remain
  documented as intentionally outside the portable runtime unless a future
  Atari-emulation target needs them
- Remaining graphics/editor fidelity: true per-pixel `PX` reverse/XOR
  compositing is implemented in the software raster canvas; Atari-native
  editor/screen behavior has been audited and documented as a modern
  `$EDITOR`/frontend-state substitution rather than a byte-for-byte Atari
  display-list emulation target
- Dynaturtle polish: visual/audio snapshot coverage and additional concrete
  compatibility cases
