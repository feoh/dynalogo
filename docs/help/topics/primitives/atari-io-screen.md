---
id: "atari-io-screen"
title: "Atari-style input and screen primitives"
kind: "primitive"
category: "frontends"
names: ["KEYP", "JOY", "JOYB", "PADDLE", "PADDLEB", "TIMEOUT", "TEXTSCREEN", "TS", "SPLITSCREEN", "SS", "FULLSCREEN", "FS", "SETCURSOR", "SETENV"]
signature: "KEYP; TEXTSCREEN; SPLITSCREEN; SETCURSOR row column"
aliases: []
summary: "Access compatibility input probes and switch text/graphics screen modes."
tags: ["atari", "input", "screen", "text", "graphics", "compatibility"]
see_also: ["window-input"]
status: "implemented"
---

These commands preserve Atari LOGO-inspired names for keyboard, joystick,
paddle, timeout, cursor, and screen-mode behavior. Some hardware-specific
probes are compatibility surfaces in modern frontends.

```logo
splitscreen
textscreen
print keyp
```
