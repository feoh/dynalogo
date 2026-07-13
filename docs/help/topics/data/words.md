---
id: "words"
title: "Words"
kind: "data-type"
category: "data"
summary: "Atomic text values used for names, symbols, booleans, and quoted inputs."
tags: ["words", "symbols", "text"]
see_also: ["word-operations", "quoting"]
status: "implemented"
---

A word is an atomic value. A leading quote creates a literal word; a colon
reads the value stored under a word name.

```logo
print "hello
make "name "DynaLOGO
print :name
```
