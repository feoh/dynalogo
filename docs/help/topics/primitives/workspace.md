---
id: "workspace"
title: "Variables, workspace, and property-list primitives"
kind: "primitive"
category: "workspace"
names: ["MAKE", "NAME", "THING", "LOCAL", "NAMEP", "DEFINEDP", "DEFINED?", "PRIMITIVEP", "PRIMITIVE?", "TEXT", "FULLTEXT", "COPYDEF", "DEFINE", "PO", "POALL", "PONS", "POPS", "POTS", "POPLS", ".PRIMITIVES", "ERASE", "ER", "ERN", "ERNS", "ERPS", "ERPL", "ERALL", "NODES", "RECYCLE", "BURY", "UNBURY", "BURIEDP", "PPROP", "GPROP", "REMPROP", "PLIST"]
signature: "MAKE name value; THING name; DEFINE name text; PPROP plist key value"
aliases: []
summary: "Create variables, inspect definitions, manage procedure text, and use property lists."
tags: ["workspace", "variables", "procedures", "properties", "definitions"]
see_also: ["procedures", "dynamic-scope", "property-lists"]
status: "implemented"
---

Workspace primitives manage names, variables, procedures, buried names,
property lists, and introspection. `MAKE` assigns a value, while `THING`
retrieves one by name.

```logo
make "size 80
print thing "size
print primitivep "fd
pprop "sprite "color "red
print plist "sprite
```
