---
id: "turtle-pen-screen"
title: "Turtle pen, drawing, and query primitives"
kind: "primitive"
category: "turtle-graphics"
names: ["PENUP", "PU", "PENDOWN", "PD", "PE", "PX", "PEN", "PN", "SETPN", "PC", "SETPENCOLOR", "SETPC", "SETPENSIZE", "SETSCRUNCH", "SETSCR", "SETLABELHEIGHT", "LABEL", "FILL", "FILLED", "HIDETURTLE", "HT", "SHOWTURTLE", "ST", "POS", "HEADING", "XCOR", "YCOR"]
signature: "PENUP; PENDOWN; SETPENCOLOR color; LABEL text; POS"
aliases: []
summary: "Control pen modes, colors, labels, fills, turtle visibility, and turtle state queries."
tags: ["turtle", "pen", "drawing", "color", "label", "fill"]
see_also: ["fd", "turtle-motion"]
status: "implemented"
---

Pen and drawing primitives control whether movement draws, how lines are
styled, labels and fills, visibility, and queries for turtle state.

```logo
penup
setpc 2
pendown
label "hello
print pos
```
