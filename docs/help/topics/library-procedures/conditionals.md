---
id: "library-conditionals"
title: "Library conditional procedures"
kind: "library-procedure"
category: "library-procedures"
names: ["CASE", "COND"]
signature: "CASE value clauses; COND clauses"
aliases: []
summary: "Select among multiple branches using library-level conditional helpers."
tags: ["conditionals", "library", "control"]
see_also: ["control-evaluation"]
status: "implemented"
---

`CASE` and `COND` provide multi-way branching in the startup library.

```logo
cond [[true [print "matched]]]
```
