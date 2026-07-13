---
id: "looping"
title: "Library looping procedures"
kind: "library-procedure"
category: "library-procedures"
names: ["FOR", "WHILE", "UNTIL", "DO.WHILE"]
signature: "FOR control-list body; WHILE test body"
aliases: []
summary: "Loop using Logo library procedures loaded at startup."
tags: ["loops", "library", "control"]
see_also: ["control-evaluation"]
status: "implemented"
---

DynaLOGO loads several control helpers as Logo procedures at VM startup.
They are written in Logo rather than Rust primitives.

```logo
for [i 1 4] [print :i]
while [:x < 10] [make "x sum :x 1]
```
