---
id: "macros"
title: "Macros"
kind: "syntax"
category: "syntax"
summary: "Macro definitions and expansion for programs that transform Logo code."
tags: ["macros", "syntax", "expansion"]
see_also: ["macros-editing", "procedures"]
status: "implemented"
---

Macros are advanced workspace entries that expand into Logo code before the
resulting expression is evaluated.

```logo
print macrop "example
print macroexpand [example 1 2]
```
