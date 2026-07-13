---
id: "shapes"
title: "Shape data"
kind: "data-type"
category: "data"
summary: "Named turtle shape data manipulated by SETSHAPE, PUTSH, GETSH, EDSH, and shape editors."
tags: ["shapes", "sprites", "editor", "dynaturtles"]
see_also: ["dynaturtles", "macros-editing"]
status: "implemented"
---

Shape data controls how turtles are drawn in graphical frontends. DynaLOGO
supports shape storage and editor workflows for native/window and browser
usage.

```logo
putsh "dog [triangle]
setshape 0 "dog
print getsh "dog
```
