---
id: "turtle-motion"
title: "Turtle motion and position primitives"
kind: "primitive"
category: "turtle-graphics"
names: ["BACK", "BK", "LEFT", "LT", "RIGHT", "RT", "SETXY", "SETPOS", "SETHEADING", "SETH", "HOME", "CLEARSCREEN", "CS"]
signature: "BACK distance; LEFT degrees; SETXY x y; CLEARSCREEN"
aliases: []
summary: "Move, rotate, position, home, and clear selected turtles."
tags: ["turtle", "movement", "graphics", "position", "heading"]
see_also: ["fd", "turtle-pen-screen"]
status: "implemented"
---

Motion primitives change turtle position or heading. `CLEARSCREEN`/`CS`
clears trails and homes turtles without drawing an extra home line.

```logo
repeat 4 [fd 80 rt 90]
setxy 10 20
setheading 90
cs
```
