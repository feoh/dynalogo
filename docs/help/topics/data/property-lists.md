---
id: "property-lists"
title: "Property lists"
kind: "data-type"
category: "data"
summary: "Named key/value bags managed with PPROP, GPROP, REMPROP, and PLIST."
tags: ["properties", "workspace", "data"]
see_also: ["workspace"]
status: "implemented"
---

Property lists attach keyed values to a word. They are useful for storing
metadata without creating many separate variable names.

```logo
pprop "sprite "color "blue
print gprop "sprite "color
print plist "sprite
```
