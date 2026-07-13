---
id: "console-files"
title: "Console and file I/O primitives"
kind: "primitive"
category: "io"
names: ["PRINT", "PR", "SHOW", "TYPE", "LOAD", "SAVE", "SETREAD", "SETWRITE", "OPENREAD", "OPENWRITE", "OPENAPPEND", "CLOSE", "READER", "WRITER", "DRIBBLE", "NODRIBBLE", "READCHAR", "RC", "READLIST", "RL", "READWORD", "RW"]
signature: "PRINT value; LOAD filename; OPENREAD filename; READWORD"
aliases: []
summary: "Print output, load/save Logo files, manage reader/writer streams, and read input."
tags: ["print", "console", "files", "input", "output"]
see_also: ["browser-filesystem", "window-input"]
status: "implemented"
---

Console primitives write text to the current frontend output path. File
primitives are available in native builds; browser builds have filesystem
limits documented separately.

```logo
print "hello
show [a b c]
load "examples/square.lgo
```
