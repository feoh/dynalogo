---
id: "macros-editing"
title: "Macro and editor primitives"
kind: "primitive"
category: "workspace"
names: [".DEFMACRO", "MACROP", "MACRO?", "MACROEXPAND", "EDIT", "ED", "EDNS", "EDSH"]
signature: ".DEFMACRO name inputs body; EDIT [name-or-file]; EDSH"
aliases: []
summary: "Define macros and open text/shape editing surfaces where supported."
tags: ["macros", "editor", "procedures", "shapes"]
see_also: ["macros", "shapes", "workspace"]
status: "implemented"
---

Macro primitives expand Logo code before evaluation. Editor primitives use the
current frontend's editor support for procedures, names, shapes, and source
files. `EDIT "square` edits a workspace procedure named `square`; `EDIT
"program.lgo` opens that source file in the system editor and evaluates it when
the editor exits. `EDIT` with no input opens a blank source buffer unless there
is an existing workspace edit session to revisit.

```logo
print macrop "repeat
edit "square
edit "program.lgo
edsh
```
