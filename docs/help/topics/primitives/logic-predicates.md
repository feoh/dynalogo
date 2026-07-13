---
id: "logic-predicates"
title: "Logic and predicate primitives"
kind: "primitive"
category: "data"
names: ["AND", "OR", "NOT", "EQUALP", "EQUAL?", "EMPTYP", "EMPTY?", "MEMBERP", "MEMBER?", "WORDP", "REALWORDP", "LISTP", "NUMBERP", "INTP", "DECIMALP", "EVENP"]
signature: "EQUALP a b; MEMBERP value aggregate; NUMBERP value"
aliases: []
summary: "Test boolean conditions, equality, membership, and value types."
tags: ["predicate", "boolean", "types", "equality", "membership"]
see_also: ["booleans", "words", "lists", "numbers"]
status: "implemented"
---

Predicates output `true` or `false`. They are useful in `IF`, `IFELSE`,
`TEST`, and higher-level library procedures.

```logo
print equalp 3 sum 1 2
print memberp "b [a b c]
if numberp 42 [print "number]
```
