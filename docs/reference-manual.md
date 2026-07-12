# DynaLOGO Reference Manual

This manual describes the **currently implemented** user-facing behavior of
DynaLOGO.

It is intentionally practical rather than aspirational:

- it focuses on features you can run today
- it groups related primitives and behaviors in one place
- it calls out important limitations and differences where they matter

For a quicker first tour, start with
[`getting-started.md`](getting-started.md).

## 1. What DynaLOGO is

DynaLOGO is a Rust implementation of the Logo programming language with two
major themes:

- **classic Logo turtle graphics**
- **dynaturtles**: multiple turtles with velocity, collisions, and `WHEN`
  event handlers

The project aims at broad UCBLogo compatibility while also taking inspiration
from Atari LOGO and MicroWorlds for multi-turtle behavior.

## 2. Frontends

DynaLOGO currently has two main user frontends.

### Terminal REPL

Start it with:

```bash
cargo run -p dynalogo --bin dynalogo
```

Use it for:

- quick experiments
- one-off calculations
- loading example files from standard input
- defining and testing procedures

You can also run a short program directly:

```bash
cargo run -p dynalogo --bin dynalogo -- --eval 'print sum 2 3'
```

### Native window frontend

Start it with:

```bash
cargo run -p dynalogo --bin dynalogo-window
```

The window frontend provides:

- a centered Cartesian drawing canvas
- an input prompt and small command log
- continuous dynaturtle simulation ticking
- animated shape rendering for dynaturtle demos
- `TOOT`-driven sound feedback

For the current dynaturtle demos, the native window is the most complete way to
experience the system.

## 3. Core language conventions

### Words

Logo words can appear in several common forms:

- `hello`
- `"hello`
- `:name`

Important conventions:

- `"word` creates a literal word
- `:name` reads the variable named `name`
- many commands accept either word input or list input depending on context

### Numbers

DynaLOGO supports numeric input for arithmetic, comparisons, turtle movement,
and simulation configuration.

Examples:

```logo
print sum 2 3
print sqrt 81
fd 100
rt 45
```

### Lists

Lists are written in square brackets:

```logo
[a b c]
[fd 100 rt 90]
```

Lists are used for:

- data
- instruction lists
- control structure bodies
- active turtle selections such as `tell [0 1 2]`

### Procedures

User procedures are defined with `TO ... END`:

```logo
to square :size
  repeat 4 [fd :size rt 90]
end
```

DynaLOGO currently uses **dynamic scope** in the classic Logo style.

## 4. Evaluation and control

### Basic control primitives

Implemented control/evaluation primitives include:

- `OUTPUT`, `OP`, `STOP`
- `REPEAT`, `REPCOUNT`
- `IF`, `IFELSE`
- `RUN`, `RUNRESULT`
- `PARSE`, `RUNPARSE`, `APPLY`
- `TEST`, `IFTRUE`/`IFT`, `IFFALSE`/`IFF`
- `CATCH`, `THROW`, `ERROR`, `WAIT`

### Library control structures

These are loaded at VM startup as Logo procedures rather than Rust primitives:

- `FOR`
- `WHILE`
- `UNTIL`
- `DO.WHILE`
- `CASE`
- `COND`

### Status of PAUSE / CONTINUE

`PAUSE` enters an interactive pause loop and `CONTINUE` resumes execution.
Commands entered while paused run in the current dynamic environment, so they
can inspect or mutate local state before continuing. Entering `OUTPUT`, `STOP`,
or `THROW` while paused resumes the enclosing computation with that control
flow.

## 5. Data, arithmetic, and predicates

### Arithmetic and numeric utilities

Implemented arithmetic includes:

- `SUM`, `DIFFERENCE`, `PRODUCT`, `QUOTIENT`, `REMAINDER`
- `ABS`, `INT`, `ROUND`, `SQRT`
- `SIN`, `COS`, `TAN`
- `RANDOM`, `RERANDOM`

Infix arithmetic is also supported:

- `+`, `-`, `*`, `/`
- comparison operators such as `<`, `>`, `=`, `<=`, `>=`, `<>`

### Lists, words, and collection operations

Implemented collection primitives include:

- `FIRST`, `BUTFIRST`/`BF`, `LAST`, `BUTLAST`/`BL`
- `FPUT`, `LPUT`, `SENTENCE`/`SE`, `LIST`, `WORD`
- `COUNT`, `ITEM`
- `WHICH`, `BEFORE`, `INSERT`, `SORT`, `SUPERSORT`
- `EMPTYP`/`EMPTY?`, `EQUALP`/`EQUAL?`, `MEMBERP`/`MEMBER?`
- `WORDP`, `LISTP`, `NUMBERP`, `INTP`, `DECIMALP`

### Arrays and templates

Implemented array/template surface includes:

- `ARRAY`, `SETITEM`, `LISTTOARRAY`, `ARRAYTOLIST`
- `FOREACH`, `MAP`, `FILTER`, `REDUCE`

Template support is usable, but full UCBLogo parity still has open edges and
follow-up work.

## 6. Variables, workspace, and property lists

### Variables

Core variable primitives include:

- `MAKE`, `NAME`, `THING`, `LOCAL`
- `NAMEP`

Examples:

```logo
make "size 80
print :size
local "tmp
```

### Property lists

Implemented property-list primitives:

- `PPROP`
- `GPROP`
- `REMPROP`
- `PLIST`

### Workspace inspection and mutation

Implemented workspace-oriented commands include:

- `DEFINEDP`/`DEFINED?`
- `PRIMITIVEP`/`PRIMITIVE?`
- `TEXT`, `FULLTEXT`, `COPYDEF`, `DEFINE`
- `PO`, `POALL`, `PONS`, `POPS`, `POTS`, `.PRIMITIVES`
- `ERASE`/`ER`, `ERN`, `ERNS`, `ERPS`, `ERALL`

These are useful for inspecting procedures, variables, and the primitive set.

## 7. Console I/O

Implemented console-oriented commands:

- `PRINT`/`PR`
- `SHOW`
- `TYPE`
- `READLIST`/`RL`

`PRINT` adds a newline. `TYPE` writes output without forcing the same printed
representation style as `SHOW`.

## 8. Classic turtle graphics

Classic single-turtle-style commands are implemented and now operate through the
same TurtleStore-backed engine used by dynaturtles.

### Movement and heading

- `FORWARD`/`FD`
- `BACK`/`BK`
- `LEFT`/`LT`
- `RIGHT`/`RT`
- `SETXY`, `SETX`, `SETY`
- `SETPOS`
- `SETHEADING`/`SETH`
- `HOME`

### Pen and visibility

- `PENUP`/`PU`
- `PENDOWN`/`PD`
- `SETPENCOLOR`/`SETPC`
- `SETPENSIZE`
- `HIDETURTLE`/`HT`
- `SHOWTURTLE`/`ST`
- `SHOWNP`

### Position/state queries

- `POS`
- `HEADING`
- `XCOR`
- `YCOR`

### Screen/canvas behavior

- `CLEARSCREEN`/`CS`
- `INIT.TURTLE`
- `DOT`

The native window frontend renders line events and turtle state visually.

## 9. Dynaturtles

Dynaturtles are the main extension that differentiates DynaLOGO from a purely
classic Logo implementation.

### Active turtle selection

Implemented selection/control commands:

- `TELL`
- `ASK`
- `EACH`
- `WHO`

Typical usage:

```logo
tell [0 1 2]
ask 1 [fd 50 rt 90]
each [fd 10]
print who
```

Behavior summary:

- `TELL` sets the active turtle selection
- `ASK` temporarily runs a block as one turtle
- `EACH` iterates a block across the current active selection
- movement and pen commands now honor that active selection

### Velocity and continuous motion

Implemented dynaturtle motion primitives:

- `SETVELOCITY`
- `SETSPEED`

These primitives matter most in the native window, because that frontend runs a
continuous simulation tick.

### Shapes

Implemented shape primitive:

- `SETSHAPE`

Current user-facing rendered shapes in the native window include:

- `"turtle`
- `"dog`
- `"ship`
- `"rocket` (rendered with the same ship-style sprite)

The window frontend animates these shapes with simple sprite-like motion such
as flipper motion, leg swing, tail wagging, or thruster flicker.

### Collisions and predicates

Implemented dynaturtle collision primitives:

- `TOUCHING`
- `WHEN`

Supported `WHEN` condition forms currently include:

- `[touching a b]`
- `[edge]`
- `[edge t]`
- `[overcolor c]`

Example:

```logo
when [touching 0 1] [print "collided]
```

### Sound events

Implemented sound event primitive:

- `TOOT`

`TOOT` records a 4-byte sound event in the VM. The native window frontend uses
this to play a short bark-like sound and display a visible `TOOT!` flash.

## 10. Examples and demo gallery

See [`../examples/README.md`](../examples/README.md).

Current notable examples include:

### Classic examples

- `square.lgo`
- `flower.lgo`
- `spiral.lgo`

### Dynaturtle examples

- `shape_parade.lgo` — a simple shape-rendering showcase
- `dogs_in_the_park.lgo` — collision-driven barking with `WHEN` and `TOOT`

## 11. Known differences, limits, and honesty notes

This manual describes what is implemented now. Important current limitations
include:

### Not yet complete compared with the long-term roadmap

- full file/workspace parity is still incomplete
- macros are not at full parity
- richer UCBLogo error-message parity remains ongoing
- browser / WASM support is still future work

### UCBLogo / Atari compatibility is a target, not a guarantee everywhere

DynaLOGO intentionally follows UCBLogo-style syntax and also tracks Atari LOGO
semantics where useful, but users should assume that edge cases may still differ
until the remaining compatibility tasks are complete.

For current audit notes, see:

- [`primitive-gaps.md`](primitive-gaps.md)
- [`primitive-inventory.md`](primitive-inventory.md)
- [`atari-logo-validation.md`](atari-logo-validation.md)

### Native window vs terminal REPL

Both frontends share the same VM core, but the native window currently gives the
best dynaturtle experience because it:

- runs continuous simulation ticking
- renders animated shapes
- reacts to `TOOT`

The terminal frontend is still excellent for quick experiments, scripts, and
core language work.

## 12. Suggested reading order

If you are new to the project, read in this order:

1. [`getting-started.md`](getting-started.md)
2. this reference manual
3. [`../examples/README.md`](../examples/README.md)
4. [`primitive-inventory.md`](primitive-inventory.md)
5. [`primitive-gaps.md`](primitive-gaps.md)
