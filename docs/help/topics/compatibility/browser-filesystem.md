---
id: "browser-filesystem"
title: "Browser filesystem limits"
kind: "compatibility"
category: "compatibility"
summary: "Browser/WASM builds cannot use native filesystem-backed primitives."
tags: ["browser", "wasm", "filesystem", "compatibility"]
see_also: ["window-input"]
status: "documented-limit"
compatibility: ["browser-limit"]
examples_required: false
---

The browser build runs the same VM core as native DynaLOGO, but browser/WASM
execution does not provide a native filesystem. Primitives such as `LOAD`,
`SAVE`, `OPENREAD`, `OPENWRITE`, `OPENAPPEND`, and `DRIBBLE` may fail or be
unavailable in the browser demo even when they work in the terminal or native
window.
