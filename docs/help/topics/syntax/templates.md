---
id: "templates"
title: "Templates"
kind: "syntax"
category: "syntax"
summary: "Template variables such as ?, ?1, ?IN, and ?OUT used by higher-order primitives."
tags: ["templates", "higher-order", "syntax"]
see_also: ["control-evaluation", "lists"]
status: "implemented"
---

Higher-order commands such as `MAP`, `FILTER`, `REDUCE`, `CASCADE`, and
`TRANSFER` use Logo template variables to refer to each input value.

```logo
print map [? * ?] [1 2 3]
print filter [? > 2] [1 2 3]
```
