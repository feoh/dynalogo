---
id: "console-files"
title: "Console and file I/O primitives"
kind: "primitive"
category: "io"
names: ["PRINT", "PR", "SHOW", "TYPE", "CT", "LOAD", "SAVE", "SETREAD", "SETWRITE", "OPENREAD", "OPENWRITE", "OPENAPPEND", "CLOSE", "READER", "WRITER", "DRIBBLE", "NODRIBBLE", "ERF", "CATALOG", "READCHAR", "RC", "READLIST", "RL", "READWORD", "RW", "READLINE", "EOFP"]
signature: "PRINT value; CT; LOAD filename; ERF filename; CATALOG [directory]; READWORD; READLINE"
aliases: []
summary: "Print output, load/save Logo files, manage reader/writer streams, and read input."
tags: ["print", "console", "files", "input", "output"]
see_also: ["browser-filesystem", "window-input"]
status: "implemented"
---

Console primitives write text to the current frontend output path. `CT` clears
the current text output buffer. `LOAD` reads Logo source from a file and
evaluates it. `SAVE` writes the visible workspace procedures as Logo source.
`CATALOG` lists files in a directory, and `ERF` erases a file. File primitives
are available in native builds; browser builds have filesystem limits documented
separately.

```logo
print "hello
show [a b c]
catalog
load "examples/square.lgo
save "my-workspace.lgo
```
