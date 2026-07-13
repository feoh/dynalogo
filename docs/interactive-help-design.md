# Interactive help design

## Goals

DynaLOGO should provide an interactive help facility that is useful from the
terminal REPL, native window, and browser build. Users should be able to ask for
help on:

- every primitive and alias
- library-level procedures such as `FOR`, `WHILE`, `CASE`, and `COND`
- data types: words, numbers, lists, arrays, property lists, booleans, turtle
  IDs, and shape data
- syntax and evaluation rules: quoting, variable references, infix operators,
  procedure definitions, dynamic scope, templates, macros, and instruction
  lists
- frontend commands and behavior such as command history, command editing,
  clean exit aliases, and browser limitations
- compatibility notes where DynaLOGO intentionally differs from UCBLogo, Atari
  LOGO, native filesystem behavior, or browser/WASM behavior

The help content must not drift from the docs. The canonical help topics should
feed both runtime help and generated documentation.

## User interface

Initial command surface:

```logo
help
help "fd
help "lists
help "templates
apropos "shape
```

Recommended aliases:

- `HELP` / `HELPON` for exact topic help
- `APROPOS` for keyword/category search
- `?` is intentionally **not** proposed initially because it already has Logo
  template meaning

Expected behavior:

- `HELP` with no input lists major categories and a few starter examples.
- `HELP "topic` prints one concise topic, examples, aliases, and see-also links.
- `APROPOS "word` lists matching topics by ID, title, aliases, and tags.
- Unknown topics show close suggestions instead of a generic error.
- Output is plain text so it works in terminal, native window log, and browser
  without filesystem access.

## Source-of-truth architecture

Use canonical topic files as the source. The reference manual and runtime help
index should be generated or validated from those files.

Proposed layout:

```text
docs/help/topics/
  primitives/fd.md
  primitives/make.md
  data/lists.md
  syntax/templates.md
  frontends/window-input.md
scripts/build_help_docs.rb
crates/dynalogo-core/src/generated_help.rs
```

Each topic file should be Markdown with YAML front matter:

````markdown
---
id: fd
title: FORWARD / FD
kind: primitive
category: turtle-graphics
names: [FORWARD, FD]
signature: FORWARD distance
aliases: [FD]
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

The generator should emit:

1. a runtime data file embedded in `dynalogo-core`, preferably a generated Rust
   table or compact static JSON string included with `include_str!`
2. documentation sections for `docs/reference-manual.md` or a generated
   `docs/help-reference.md` that the reference manual links to
3. validation reports for missing topics, duplicate aliases, broken `see_also`
   links, and topic IDs that do not match implemented primitives

The generated runtime help must be available at compile time. Do not read help
from the filesystem at runtime; that would fail in browser/WASM builds and make
release packages fragile.

## Coverage model

Topic types:

- `primitive`: Rust VM primitive or alias
- `library-procedure`: Logo procedure loaded by the VM at startup
- `data-type`: values and structures users can manipulate
- `syntax`: expression/evaluation grammar and special forms
- `frontend`: terminal/window/browser usage
- `compatibility`: UCBLogo/Atari/browser/native differences
- `concept`: higher-level narrative topic such as dynaturtles or macros

Minimum fields for every topic:

- `id`
- `title`
- `kind`
- `category`
- `summary` or first paragraph body
- body text

Additional fields for command-like topics:

- `names`
- `signature`
- `aliases`
- `examples`
- `see_also`

## Runtime implementation outline

1. Add a `help` module in `dynalogo-core` with:
   - `HelpTopic`
   - `HelpIndex`
   - exact lookup by canonical ID/name/alias
   - case-insensitive matching
   - simple search for `APROPOS`
   - close suggestions for unknown topics
2. Add VM primitives:
   - `HELP` / `HELPON`
   - `APROPOS`
3. Format help as plain text lines through the existing output path.
4. Add fixture coverage for exact lookup, alias lookup, search, unknown-topic
   suggestions, and browser-safe no-filesystem behavior.
5. Update terminal/window startup text to mention `HELP` after the primitives
   exist.

## Validation and CI

The generator/validator should fail when:

- a primitive or alias exposed by `.PRIMITIVES` has no primitive-topic lookup
  coverage
- a topic references a missing `see_also` ID
- two topics claim the same ID or alias without an explicit shared alias rule
- generated runtime help or generated docs are stale
- a topic lacks examples where examples are required for command-like topics

Useful commands once implemented:

```bash
ruby scripts/build_help_docs.rb --check
cargo test --workspace -q
cargo clippy --workspace --all-targets -- -D warnings
```

## Task breakdown

Tracked under Witan project
`wp-dynalogo-command-editing-and-interactive-help-a319d0`:

- `tk-design-source-of-truth-interactive-help-system-0b294b` — this design
- `tk-implement-source-of-truth-interactive-help-facil-d45278` — help epic
- `tk-define-canonical-help-topic-schema-and-source-la-25de0b` — topic schema
  and canonical source layout
- `tk-generate-reference-docs-and-runtime-help-data-fr-c980ee` — generator,
  runtime data, and drift checks
- `tk-add-core-help-primitives-and-topic-lookup-a6e043` — VM primitives and
  lookup/search implementation
- `tk-cover-primitives-data-types-expression-syntax-an-87d08e` — content
  coverage for primitives, data types, syntax, and language features
- `tk-integrate-interactive-help-with-terminal-and-win-5541da` — frontend
  integration and onboarding text

## Resolved by the schema task

[`help/topic-schema.md`](help/topic-schema.md) resolves the initial source-layout
and schema decisions:

- Generate a separate `docs/help-reference.md` first, then link it from the
  reference manual.
- Parse examples from fenced `logo` code blocks in the Markdown body.
- Make `HELP` default to compact topic output; full-topic output can be added
  later.
- Make `APROPOS` initially search IDs, titles, names, aliases, summaries,
  categories, and tags; full body-text search can be added later.
