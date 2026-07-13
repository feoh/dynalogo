---
id: "turtle-ids"
title: "Turtle IDs and selections"
kind: "data-type"
category: "data"
summary: "Numeric turtle IDs and lists used by TELL, ASK, EACH, and dynaturtle commands."
tags: ["turtles", "dynaturtles", "ids", "selection"]
see_also: ["dynaturtles"]
status: "implemented"
---

Dynaturtle commands select turtles by numeric ID or lists of IDs. `WHO`
reports the current selection.

```logo
tell [0 1 2]
ask [0 1] [fd 10]
print who
```
