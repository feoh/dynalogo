---
id: "dynamic-scope"
title: "Dynamic scope"
kind: "syntax"
category: "syntax"
summary: "Logo-style dynamic variable lookup through the current call stack."
tags: ["scope", "variables", "procedures"]
see_also: ["procedures", "workspace"]
status: "implemented"
---

DynaLOGO follows classic Logo dynamic scope: variable lookup searches the
current procedure call chain rather than lexical blocks.

```logo
make "size 40
print :size
```
