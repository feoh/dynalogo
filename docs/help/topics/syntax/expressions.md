---
id: expressions
title: Expressions and evaluation
kind: syntax
category: syntax
summary: Logo expressions are parsed by command arity, inputs, and infix operators.
tags: [syntax, evaluation, expressions, infix]
see_also: [lists]
status: implemented
---

DynaLOGO parses expressions using command arity and Logo evaluation rules.
Commands consume their required inputs, and infix arithmetic/comparison
operators can be used inside expressions.

```logo
print sum 2 3
print 2 + 3 * 4
if :x > 10 [print "big]
```
