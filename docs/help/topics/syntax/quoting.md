---
id: "quoting"
title: "Quoting and variable references"
kind: "syntax"
category: "syntax"
summary: "How quoted words, colon references, and bare words become Logo values or procedure calls."
tags: ["syntax", "words", "variables", "quoting"]
see_also: ["words", "workspace", "expressions"]
status: "implemented"
---

A leading quote creates a literal word. A leading colon reads a variable.
Bare words are parsed as known primitives/procedures when arity is known,
otherwise as literal bare words in data contexts.

```logo
make "size 50
print :size
print "size
```
