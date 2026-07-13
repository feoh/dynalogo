---
id: "infix"
title: "Infix operators"
kind: "syntax"
category: "syntax"
summary: "Arithmetic and comparison operators that can be written between expressions."
tags: ["syntax", "infix", "arithmetic", "comparison"]
see_also: ["arithmetic", "expressions"]
status: "implemented"
---

Infix operators are parsed with precedence so arithmetic reads naturally.
Multiplication and division bind tighter than addition and subtraction.

```logo
print 2 + 3 * 4
print 10 >= 5
```
