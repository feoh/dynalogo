---
id: "arithmetic"
title: "Arithmetic and numeric primitives"
kind: "primitive"
category: "math"
names: ["SUM", "+", "DIFFERENCE", "-", "PRODUCT", "*", "QUOTIENT", "/", "REMAINDER", "ABS", "INT", "ROUND", "SQRT", "SIN", "COS", "TAN", "RANDOM", "RERANDOM", "FACTORIAL", "DIVISORP"]
signature: "SUM a b; DIFFERENCE a b; PRODUCT a b; QUOTIENT a b"
aliases: []
summary: "Compute numeric results, trigonometry, random values, and numeric divisibility helpers."
tags: ["math", "arithmetic", "numbers", "random", "trigonometry"]
see_also: ["numbers", "logic-predicates"]
status: "implemented"
---

DynaLOGO numbers are floating-point values. Arithmetic primitives consume
numeric inputs and output numeric results. The infix operators `+`, `-`,
`*`, and `/` are aliases for the corresponding arithmetic operations.

```logo
print sum 2 3
print 2 + 3 * 4
print sqrt 81
print random 10
```
