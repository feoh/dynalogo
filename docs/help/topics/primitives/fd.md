---
id: fd
title: FORWARD / FD
kind: primitive
category: turtle-graphics
names: [FORWARD, FD]
signature: FORWARD distance
aliases: [FD]
summary: Move the selected turtle or turtles forward by a distance.
tags: [turtle, movement, graphics]
see_also: []
status: implemented
---

Move the selected turtle or turtles forward by `distance` turtle units. If the
pen is down, DynaLOGO draws a line as the turtle moves.

```logo
fd 100
forward 50
repeat 4 [fd 80 rt 90]
```
