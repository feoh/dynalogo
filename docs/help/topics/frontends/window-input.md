---
id: "window-input"
title: "Native window input"
kind: "frontend"
category: "frontends"
summary: "The native window prompt supports editing, history, exit commands, and text scaling."
tags: ["window", "input", "history", "font-size"]
see_also: ["expressions"]
status: "implemented"
---

The native window prompt accepts Logo commands while the graphics canvas remains
visible. Use Enter to submit a command.

Editing controls:

- Left/Right move within the current command.
- Home/End jump to the start or end.
- Backspace deletes before the cursor.
- Delete deletes at the cursor.
- Up/Down browse command history.
- Ctrl+Plus/Ctrl+Equals and Ctrl+Minus scale input and output text.
- `exit`, `quit`, `bye`, and Ctrl+Q close the window.

```logo
repeat 36 [fd 120 rt 170]
```
