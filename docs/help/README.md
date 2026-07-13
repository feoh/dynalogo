# Interactive help source

This directory is the canonical source tree for DynaLOGO's planned interactive
`HELP` / `APROPOS` facility.

The goal is to write help content once and use it in two places:

1. generated or validated user documentation
2. compile-time embedded runtime help data for terminal, native window,
   release-package, and browser/WASM builds

Runtime help must not read these files from the filesystem. The generator in
`scripts/build_help_docs.rb` parses this tree and emits:

- `docs/help-reference.md`
- `crates/dynalogo-core/src/generated_help.rs`

Run `ruby scripts/build_help_docs.rb --check` to verify generated outputs are
current.

## Source layout

```text
docs/help/
  README.md
  topic-schema.md
  topic.schema.json
  topics/
    _template.md
    README.md
    primitives/
    library-procedures/
    data/
    syntax/
    concepts/
    frontends/
    compatibility/
```

Topic file paths are organizational only. The stable topic identity is the
front matter `id` field.

## Topic categories

Use these top-level category directories unless a later schema revision expands
the taxonomy:

- `primitives/` — Rust VM primitives and their aliases
- `library-procedures/` — Logo procedures loaded by the VM at startup
- `data/` — user-visible value types and data structures
- `syntax/` — expression syntax, parsing, evaluation, templates, and macros
- `concepts/` — larger feature areas such as dynaturtles or workspaces
- `frontends/` — terminal, native window, and browser usage
- `compatibility/` — UCBLogo, Atari LOGO, native/browser, and intentional
  divergence notes

## Current status

This tree currently defines the schema and representative starter topics. It is
not yet wired into the runtime. The next implementation tasks are tracked in
Witan under `wp-dynalogo-command-editing-and-interactive-help-a319d0`.
