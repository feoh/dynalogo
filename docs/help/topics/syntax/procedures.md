---
id: "procedure-definitions"
title: "Procedures"
kind: "syntax"
category: "syntax"
summary: "User-defined TO ... END procedures with Logo-style dynamic inputs."
tags: ["procedures", "syntax", "workspace"]
see_also: ["workspace", "dynamic-scope"]
status: "implemented"
---

Define procedures with `TO`, input names, a body, and `END`. Call the
procedure by name after it is loaded or defined.

```logo
to square :size
  repeat 4 [fd :size rt 90]
end
square 80
```
