---
id: "arrays"
title: "Array primitives"
kind: "primitive"
category: "data"
names: ["ARRAY", "SETITEM", "LISTTOARRAY", "ARRAYTOLIST"]
signature: "ARRAY size; SETITEM index array value"
aliases: []
summary: "Create arrays, mutate indexed array slots, and convert between arrays and lists."
tags: ["arrays", "data", "indexed", "mutable"]
see_also: ["lists"]
status: "implemented"
---

Arrays are indexed mutable aggregate values. Convert from lists when you
want indexed storage, and convert back to inspect values as lists.

```logo
make "a array 3
setitem 1 :a "red
print arraytolist :a
```
