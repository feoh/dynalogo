---
id: "dynaturtles"
title: "Dynaturtle primitives"
kind: "primitive"
category: "dynaturtles"
names: ["TELL", "ASK", "EACH", "WHO", "SETVELOCITY", "SETSPEED", "SPEED", "SETSHAPE", "SHAPE", "PUTSH", "GETSH", "BOUNCE", "WRAP", "FENCE", "WINDOW", "TOUCHING", "OVER", "WHEN", "TOOT"]
signature: "TELL turtle-or-list; ASK turtles instructions; WHEN condition instructions"
aliases: []
summary: "Select, animate, shape, collide, and coordinate multiple turtles."
tags: ["dynaturtles", "turtles", "sprites", "collision", "shapes", "sound"]
see_also: ["turtle-ids", "shapes", "fd"]
status: "implemented"
---

Dynaturtle primitives extend classic Logo with multiple turtles, velocity,
shapes, collision checks, edge modes, per-turtle instructions, and sound
events.

```logo
tell [0 1]
ask [0 1] [fd 20]
setshape 0 "dog
when [touching 0 1] [toot 1 2 3 4]
```
