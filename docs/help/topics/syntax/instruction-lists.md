---
id: "instruction-lists"
title: "Instruction lists"
kind: "syntax"
category: "syntax"
summary: "Lists of Logo instructions evaluated by control primitives such as REPEAT, IF, RUN, and ASK."
tags: ["syntax", "lists", "control"]
see_also: ["lists", "control-evaluation"]
status: "implemented"
---

Instruction lists are list literals that contain code to evaluate later.
Control and dynaturtle primitives use them for bodies.

```logo
repeat 4 [fd 50 rt 90]
ask [0 1] [fd 20]
```
