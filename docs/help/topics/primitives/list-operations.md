---
id: "list-operations"
title: "List and sequence primitives"
kind: "primitive"
category: "data"
names: ["FIRST", "BUTFIRST", "BF", "LAST", "BUTLAST", "BL", "FPUT", "LPUT", "SENTENCE", "SE", "LIST", "COUNT", "ITEM", "RANK", "RANPICK"]
signature: "FIRST value; LIST a b; ITEM index aggregate; COUNT value"
aliases: []
summary: "Build, inspect, and select items from lists, words, and other Logo aggregates."
tags: ["lists", "sequences", "aggregate", "selection"]
see_also: ["lists", "word-operations", "arrays"]
status: "implemented"
---

These primitives operate on Logo aggregates. Many work on both lists and
words, matching classic Logo conventions.

```logo
print first [red green blue]
print butfirst "logo
print sentence [a b] [c d]
print item 2 [red green blue]
```
