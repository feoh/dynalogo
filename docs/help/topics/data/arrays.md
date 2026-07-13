---
id: "array-values"
title: "Array values"
kind: "data-type"
category: "data"
summary: "Mutable indexed aggregate values created by ARRAY and converted to or from lists."
tags: ["arrays", "data", "mutable"]
see_also: ["arrays", "lists"]
status: "implemented"
---

Arrays are useful when a program needs mutable indexed storage rather than
list construction.

```logo
make "a array 2
setitem 1 :a "cat
print arraytolist :a
```
