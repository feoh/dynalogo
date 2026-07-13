---
id: "booleans"
title: "Booleans"
kind: "data-type"
category: "data"
summary: "`true` and `false` values produced by predicates and consumed by conditionals."
tags: ["booleans", "predicates", "conditionals"]
see_also: ["logic-predicates", "control-evaluation"]
status: "implemented"
---

Predicates output boolean values. Conditionals accept booleans from direct
predicate calls or expressions.

```logo
print equalp 2 sum 1 1
if true [print "yes]
```
