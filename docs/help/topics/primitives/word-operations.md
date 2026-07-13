---
id: "word-operations"
title: "Word primitives"
kind: "primitive"
category: "data"
names: ["WORD", "ASCII", "CHAR", "LOWERCASE", "REV"]
signature: "WORD a b; ASCII word; CHAR code"
aliases: []
summary: "Construct and transform Logo words and characters."
tags: ["words", "text", "characters"]
see_also: ["words", "list-operations"]
status: "implemented"
---

Words are atomic text values. These primitives concatenate words, convert
between character codes and one-character words, lowercase text, and
reverse text.

```logo
print word "dy "nalogo
print char 65
print lowercase "HELLO
print rev "logo
```
