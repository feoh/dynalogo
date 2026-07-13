---
id: "help"
title: "HELP / HELPON"
kind: "primitive"
category: "help"
names: ["HELP", "HELPON"]
signature: "HELP [topic]"
aliases: ["HELPON"]
summary: "Show interactive help topics from DynaLOGO's embedded help index."
tags: ["help", "documentation", "topics"]
see_also: ["apropos"]
status: "implemented"
---

`HELP` with no input lists available help categories and starter examples.
`HELP` with a topic word prints that topic. `HELPON` is an exact-topic form
that requires one input.

```logo
help
help "fd
helpon "lists
```
