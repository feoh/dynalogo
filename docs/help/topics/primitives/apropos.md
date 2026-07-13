---
id: "apropos"
title: "APROPOS"
kind: "primitive"
category: "help"
names: ["APROPOS"]
signature: "APROPOS keyword"
aliases: []
summary: "Search help topic IDs, names, aliases, summaries, categories, and tags."
tags: ["help", "search", "documentation"]
see_also: ["help"]
status: "implemented"
---

`APROPOS` searches the embedded help index for a word or phrase and prints
matching topic IDs with short summaries.

```logo
apropos "turtle
apropos "window
```
