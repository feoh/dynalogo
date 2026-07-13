# Computer Science Logo Style examples

These examples are adapted from Brian Harvey's *Computer Science Logo Style* (CSLS), with source URLs preserved below for volume/chapter provenance. They are included so DynaLOGO users can browse runnable programs by volume/chapter/topic.

Attribution: the original examples are by Brian Harvey in *Computer Science Logo Style*. These copies are credited and sourced CSLS examples, not original DynaLOGO programs. Some files include small harness lines or deterministic print calls so they can double as regression fixtures.

## Layout

- `volume1/` — Symbolic Computing examples.
- `volume2/` — Advanced Techniques examples.
- `volume3/` — Beyond Programming examples.
- `graphics/` — turtle/graphics examples with trace sidecars.
- `input/` — scripted or filesystem-backed examples with input/output sidecars.

For most examples, run the `.lgo` file from the terminal REPL:

```bash
cargo run -p dynalogo --bin dynalogo < examples/csls/volume1/v1_ch02_words_lists.lgo
```

Sidecar files are reference artifacts:

- `.out` — expected stdout from the corresponding `.lgo` file.
- `.err` — expected error text from an intentional error example.
- `.in` — scripted stdin consumed by input-oriented examples.
- `.trace` — expected headless graphics/turtle trace.

## Source provenance

### Volume 1: Symbolic Computing

- Chapter 2, Words and Lists: <https://people.eecs.berkeley.edu/~bh/v1ch2/words.html>
- Chapter 3, Variables and Procedures: <https://people.eecs.berkeley.edu/~bh/v1ch3/var.html>
- Chapter 4, Predicates and Conditionals: <https://people.eecs.berkeley.edu/~bh/v1ch4/cond.html>
- Chapter 5, Higher-Order Functions: <https://people.eecs.berkeley.edu/~bh/v1ch5/hof.html>
- Chapter 7, Introduction to Recursion: <https://people.eecs.berkeley.edu/~bh/v1ch7/rec.html>
- Chapter 8, Practical Recursion: <https://people.eecs.berkeley.edu/~bh/v1ch8/prac.html>
- Chapter 10, Turtle Geometry: <https://people.eecs.berkeley.edu/~bh/v1ch10/turtle.html>
- Chapter 11, Recursive Operations: <https://people.eecs.berkeley.edu/~bh/v1ch11/recursive.html>
- Chapter 15, Debugging: <https://people.eecs.berkeley.edu/~bh/v1ch15/debug.html>

### Volume 2: Advanced Techniques

- Chapter 1, Data Files: <https://people.eecs.berkeley.edu/~bh/v2ch1/files.html>
- Chapter 3, Nonlocal Exit: <https://people.eecs.berkeley.edu/~bh/v2ch3/exit.html>
- Chapter 5, Program as Data: <https://people.eecs.berkeley.edu/~bh/v2ch5/prgdat.html>
- Chapter 8, Property Lists: <https://people.eecs.berkeley.edu/~bh/v2ch8/plist.html>
- Chapter 10, Iteration and Control Structures: <https://people.eecs.berkeley.edu/~bh/v2ch10/iter.html>
- Chapter 11, Cryptographer's Helper / ASCII: <https://people.eecs.berkeley.edu/~bh/v2ch11/crypto.html>
- Chapter 12, Macros: <https://people.eecs.berkeley.edu/~bh/v2ch12/macro.html>

### Volume 3: Beyond Programming

- Chapter 2, Discrete Mathematics: <https://people.eecs.berkeley.edu/~bh/v3ch2/math.html>
- Chapter 3, Algorithms and Data Structures: <https://people.eecs.berkeley.edu/~bh/v3ch3/algs.html>
- Chapter 4, Programming Language Design: <https://people.eecs.berkeley.edu/~bh/v3ch4/langd.html>

See [`../../docs/csls-application-examples.md`](../../docs/csls-application-examples.md) for notes on larger CSLS applications that are not yet suitable as deterministic distribution examples.
