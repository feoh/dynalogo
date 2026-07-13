---
id: "macros-editing"
title: "Macro and editor primitives"
kind: "primitive"
category: "workspace"
names: [".DEFMACRO", "MACROP", "MACRO?", "MACROEXPAND", "EDIT", "ED", "EDNS", "EDSH"]
signature: ".DEFMACRO name inputs body; EDIT name; EDSH"
aliases: []
summary: "Define macros and open text/shape editing surfaces where supported."
tags: ["macros", "editor", "procedures", "shapes"]
see_also: ["macros", "shapes", "workspace"]
status: "implemented"
---

Macro primitives expand Logo code before evaluation. Editor primitives use
the current frontend's editor support for procedures, names, and shapes.

```logo
print macrop "repeat
edit "square
edsh
```
