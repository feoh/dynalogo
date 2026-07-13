---
id: "control-evaluation"
title: "Control and evaluation primitives"
kind: "primitive"
category: "control"
names: ["OUTPUT", "OP", "STOP", "REPEAT", "IF", "IFELSE", "RUN", "RUNRESULT", "PARSE", "RUNPARSE", "APPLY", "FOREACH", "MAP", "FILTER", "REDUCE", "CASCADE", "CASCADE.2", "TRANSFER", "REPCOUNT", "TEST", "IFTRUE", "IFT", "IFFALSE", "IFF", "WAIT", "CATCH", "THROW", "ERROR", "PAUSE", "CONTINUE"]
signature: "REPEAT count instructions; IF condition instructions; RUN instructions"
aliases: []
summary: "Control evaluation, run instruction lists, process collections, and handle errors or pauses."
tags: ["control", "evaluation", "lists", "errors", "pause"]
see_also: ["instruction-lists", "templates", "procedures"]
status: "implemented"
---

Control primitives evaluate instruction lists, branch on conditions,
process collections, and manage non-local control flow such as errors,
pauses, `OUTPUT`, and `STOP`.

```logo
repeat 4 [fd 50 rt 90]
if equalp 2 sum 1 1 [print "yes]
print map "first [[a b] [c d]]
catch "oops [throw "oops "handled]
```
