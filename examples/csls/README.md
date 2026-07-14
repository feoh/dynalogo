# Computer Science Logo Style examples

These examples are adapted from Brian Harvey's *Computer Science Logo Style*
(CSLS), with source URLs preserved below for volume/chapter provenance. They
are included so DynaLOGO users can browse programs by volume/chapter/topic,
including both deterministic DynaLOGO-compatible examples and full upstream
transcripts of the larger CSLS applications.

Attribution: the original examples are by Brian Harvey in *Computer Science
Logo Style*. These copies are credited and sourced CSLS examples, not original
DynaLOGO programs. Some files include small harness lines or deterministic
print calls so they can double as regression fixtures; the larger `*_full` or
chapter-specific application files preserve upstream UCBLogo source more
directly and may require compatibility work before they run end-to-end in
DynaLOGO.

## Layout

- `volume1/` — Symbolic Computing examples.
- `volume2/` — Advanced Techniques examples.
- `volume3/` — Beyond Programming examples.
- `graphics/` — turtle/graphics examples.
- `input/` — scripted or filesystem-backed examples with input/output sidecars.

For deterministic examples that target the current DynaLOGO feature set, run
the `.lgo` file from the terminal REPL:

```bash
cargo run -p dynalogo --bin dynalogo < examples/csls/volume1/v1_ch02_words_lists.lgo
```

Full upstream transcripts are included for source study and compatibility
tracking. They are credited and linked, but some intentionally preserve
UCBLogo primitives, interactive loops, graphics behavior, or file semantics
that DynaLOGO may not fully implement yet.

Sidecar files are reference artifacts:

- `.out` — expected stdout from the corresponding `.lgo` file.
- `.err` — expected error text from an intentional error example.
- `.in` — scripted stdin consumed by input-oriented examples.

## Source provenance

### Volume 1: Symbolic Computing

- Chapter 2, Words and Lists:
  <https://people.eecs.berkeley.edu/~bh/v1ch2/words.html>
- Chapter 3, Variables and Procedures:
  <https://people.eecs.berkeley.edu/~bh/v1ch3/var.html>
- Chapter 4, Predicates and Conditionals:
  <https://people.eecs.berkeley.edu/~bh/v1ch4/cond.html>
- Chapter 5, Higher-Order Functions:
  <https://people.eecs.berkeley.edu/~bh/v1ch5/hof.html>
- Chapter 7, Introduction to Recursion:
  <https://people.eecs.berkeley.edu/~bh/v1ch7/rec.html>
- Chapter 8, Practical Recursion:
  <https://people.eecs.berkeley.edu/~bh/v1ch8/prac.html>
- Chapter 10, Turtle Geometry:
  <https://people.eecs.berkeley.edu/~bh/v1ch10/turtle.html>
- Chapter 11, Recursive Operations:
  <https://people.eecs.berkeley.edu/~bh/v1ch11/recursive.html>
- Chapter 15, Debugging:
  <https://people.eecs.berkeley.edu/~bh/v1ch15/debug.html>

### Volume 2: Advanced Techniques

- Chapter 1, Data Files:
  <https://people.eecs.berkeley.edu/~bh/v2ch1/files.html>
- Chapter 2, File Differences:
  <https://people.eecs.berkeley.edu/~bh/v2ch2/diff.html>
- Chapter 3, Nonlocal Exit:
  <https://people.eecs.berkeley.edu/~bh/v2ch3/exit.html>
- Chapter 4, Solitaire:
  <https://people.eecs.berkeley.edu/~bh/v2ch4/solitaire.html>
- Chapter 5, Program as Data:
  <https://people.eecs.berkeley.edu/~bh/v2ch5/prgdat.html>
- Chapter 6, BASIC Compiler:
  <https://people.eecs.berkeley.edu/~bh/v2ch6/basic.html>
- Chapter 7, Pattern Matcher:
  <https://people.eecs.berkeley.edu/~bh/v2ch7/match.html>
- Chapter 8, Property Lists:
  <https://people.eecs.berkeley.edu/~bh/v2ch8/plist.html>
- Chapter 9, Doctor:
  <https://people.eecs.berkeley.edu/~bh/v2ch9/doctor.html>
- Chapter 10, Iteration and Control Structures:
  <https://people.eecs.berkeley.edu/~bh/v2ch10/iter.html>
- Chapter 11, Cryptographer's Helper / ASCII:
  <https://people.eecs.berkeley.edu/~bh/v2ch11/crypto.html>
- Chapter 12, Macros:
  <https://people.eecs.berkeley.edu/~bh/v2ch12/macro.html>
- Chapter 13, Fourier Series Plotter:
  <https://people.eecs.berkeley.edu/~bh/v2ch13/fourie.html>

### Volume 3: Beyond Programming

- Chapter 2, Discrete Mathematics:
  <https://people.eecs.berkeley.edu/~bh/v3ch2/math.html>
- Chapter 3, Algorithms and Data Structures:
  <https://people.eecs.berkeley.edu/~bh/v3ch3/algs.html>
- Chapter 4, Programming Language Design:
  <https://people.eecs.berkeley.edu/~bh/v3ch4/langd.html>
- Chapter 5, Programming Language Implementation:
  <https://people.eecs.berkeley.edu/~bh/v3ch5/langi.html>
- Chapter 6, Artificial Intelligence:
  <https://people.eecs.berkeley.edu/~bh/v3ch6/ai.html>

## Full application and graphics transcripts

The examples directory also includes full upstream transcripts for CSLS examples
that were previously represented only by small deterministic stand-ins:

- `graphics/v1_ch10_turtle_geometry.lgo` — Volume 1 Chapter 10 turtle-geometry
  progression, including the graphics-heavy spin/squiggle examples and
  recursive trees.
- `graphics/v2_ch13_fourier_plotter.lgo` — Volume 2 Chapter 13 Fourier series
  plotter.
- `volume2/v2_ch01_format_full.lgo` — Volume 2 Chapter 1 file-formatting
  program, from the upstream `format.lg` listing.
- `volume2/v2_ch02_file_diff.lgo` — Volume 2 Chapter 2 file-difference
  program.
- `volume2/v2_ch04_solitaire.lgo` — Volume 2 Chapter 4 solitaire game.
- `volume2/v2_ch06_basic_compiler.lgo` — Volume 2 Chapter 6 BASIC compiler,
  from the upstream `basic.lg` listing.
- `volume2/v2_ch07_pattern_matcher.lgo` — Volume 2 Chapter 7 pattern matcher.
- `volume2/v2_ch09_doctor.lgo` — Volume 2 Chapter 9 Doctor/ELIZA-style
  program, from the upstream `doctormatch.lg` listing.
- `volume2/v2_ch11_crypto_helper_full.lgo` — Volume 2 Chapter 11 full
  cryptographer's helper.
- `volume3/v3_ch02_discrete_math_full.lgo` — Volume 3 Chapter 2 full
  discrete-math/logic program, from the upstream `math.lg` listing.
- `volume3/v3_ch03_algorithms_full.lgo` — Volume 3 Chapter 3 full algorithms
  program, from the upstream `algs.lg` listing.
- `volume3/v3_ch04_language_design_pascal.lgo` — Volume 3 Chapter 4
  Pascal/language-design listing, from the upstream `pascal.lg` listing.
- `volume3/v3_ch05_language_implementation.lgo` — Volume 3 Chapter 5
  Pascal/language-implementation example, from the upstream `pascal.lg`
  listing.
- `volume3/v3_ch06_artificial_intelligence.lgo` — Volume 3 Chapter 6 STUDENT
  algebra word-problem solver, from the upstream `student.lg` listing.

See [`../../docs/csls-application-examples.md`](../../docs/csls-application-examples.md)
for notes on the difference between deterministic test fixtures and full
upstream application examples.
