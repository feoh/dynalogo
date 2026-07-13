# Help topic schema

Help topics are Markdown files with YAML front matter. The front matter is the
machine-readable contract used by future generators and validators. The Markdown
body is the canonical prose shown in docs and, in compact form, at runtime.

A machine-readable front matter schema lives at
[`topic.schema.json`](topic.schema.json).

## Topic file shape

````markdown
---
id: fd
title: FORWARD / FD
kind: primitive
category: turtle-graphics
names: [FORWARD, FD]
signature: FORWARD distance
aliases: [FD]
summary: Move the selected turtle or turtles forward.
tags: [turtle, movement, graphics]
see_also: [bk, lt, rt, setxy]
status: implemented
---

Move the selected turtle or turtles forward by `distance` turtle units.

```logo
fd 100
forward 50
```
````

## Required fields

Every topic must define:

- `id` — stable lowercase lookup ID. Use kebab-case or primitive spelling such
  as `fd`, `runresult`, or `do.while`.
- `title` — human-readable heading.
- `kind` — one of the topic kinds below.
- `category` — user-facing grouping, such as `turtle-graphics` or `data`.
- `summary` — one-sentence compact help text.
- `status` — one of `implemented`, `partial`, `planned`, or
  `documented-limit`.

The Markdown body is also required and should explain behavior, examples, and
important caveats.

## Topic kinds

- `primitive` — built-in Rust VM primitives and aliases.
- `library-procedure` — Logo procedures loaded by the VM at startup.
- `data-type` — words, numbers, lists, arrays, property lists, booleans,
  turtle IDs, and shape data.
- `syntax` — evaluation rules, special syntax, templates, macros, procedures,
  and expressions.
- `frontend` — terminal, native window, browser, and release-package operation.
- `compatibility` — UCBLogo, Atari LOGO, native/browser differences, and
  intentional divergences.
- `concept` — higher-level narrative topics that span several commands.

## Command-like fields

Primitive and library-procedure topics must define:

- `names` — all callable names covered by this topic, including the canonical
  name.
- `signature` — user-facing call shape. Use Logo names, not Rust helper names.
- `aliases` — alternate callable names. Empty array is allowed.

A future validator should ensure every name in `names` resolves to this topic and
that aliases do not collide across topics unless explicitly allowed.

## Optional fields

- `tags` — search keywords for `APROPOS`.
- `see_also` — topic IDs related to this topic.
- `since` — first DynaLOGO version that shipped the behavior, when known.
- `compatibility` — short compatibility labels such as `ucblogo`,
  `atari-logo`, or `browser-limit`.
- `examples_required` — overrides default example requirements.

Examples should live in fenced `logo` code blocks in the Markdown body. That
keeps examples readable in docs while still allowing a generator to extract them
for runtime help or future validation.

## ID and alias rules

- IDs are lowercase and stable.
- Prefer the shortest commonly used primitive alias for primitive IDs when the
  alias is canonical in Logo practice (`fd`, `bk`, `lt`, `rt`, `cs`).
- Use the long descriptive concept for non-command topics (`lists`,
  `templates`, `window-input`).
- Store callable names in uppercase in `names` and `aliases`.
- Runtime lookup must be case-insensitive.
- `HELP "FD`, `HELP "fd`, and `HELP "forward` should all resolve to the same
  `fd` topic.

## Runtime formatting expectations

The first runtime implementation should format plain text:

1. title
2. signature, for command-like topics
3. aliases, if any
4. summary
5. body text, compacted to fit REPL/window output
6. examples
7. see-also IDs

Do not rely on terminal control sequences, Markdown rendering, or filesystem
access at runtime. Browser/WASM builds must use the same embedded data as native
builds.

## Generator and validation contract

The future generator should fail when:

- a topic file lacks required front matter
- a topic has an invalid `kind`, `status`, or non-lowercase `id`
- a command-like topic lacks `names` or `signature`
- two topics claim the same `id`, callable name, or alias
- `see_also` references a missing topic ID
- a primitive/library-procedure topic lacks at least one `logo` example unless
  `examples_required: false` is set
- generated runtime help or generated docs are stale
- implemented primitive inventory entries lack primitive-topic lookup coverage

## Resolved design choices

The schema task resolves these choices from
[`../interactive-help-design.md`](../interactive-help-design.md):

- Generate a separate `docs/help-reference.md` first, then link to it from the
  reference manual. This avoids rewriting the entire reference manual while the
  help corpus is being built out.
- Parse examples from fenced `logo` code blocks in the Markdown body.
- Make `HELP` default to compact topic output. Long-form/full-topic output can be
  added later without changing the source schema.
- Make `APROPOS` initially search IDs, titles, names, aliases, summaries,
  categories, and tags. Full body-text search can be added later.
- Validate that every implemented primitive name exposed by `.PRIMITIVES` is
  covered by at least one primitive help topic.
