# CSLS application example coverage

Audit date: 2026-07-13

This note tracks larger Computer Science Logo Style application examples and how
DynaLOGO separates two use cases:

1. deterministic fixtures under `crates/dynalogo-core/tests/`, which keep CI
   stable and target the current DynaLOGO compatibility surface; and
2. full credited source transcripts under `examples/csls/`, which preserve the
   upstream CSLS application and graphics programs for browsing, study, and
   future compatibility work.

## Full upstream transcripts in `examples/csls/`

The examples directory now includes the complex and graphics-heavy CSLS examples
that were previously represented only by small deterministic stand-ins:

- Volume 1 Chapter 10, Turtle Geometry (`v1ch10/turtle.html`)
  - `examples/csls/graphics/v1_ch10_turtle_geometry.lgo`
  - Full turtle-geometry progression including the graphics-heavy
    squiggle/squaggle/squoggle spin examples, polygon/star examples, fingers,
    and recursive trees.
- Volume 2 Chapter 1, Data Files (`v2ch1/files.html`)
  - `examples/csls/volume2/v2_ch01_format_full.lgo`
  - Full upstream `format.lg` file-formatting program listing.
- Volume 2 Chapter 2, File Differences (`v2ch2/diff.html`)
  - `examples/csls/volume2/v2_ch02_file_diff.lgo`
  - Full file-difference algorithm with credited source URL and scratch-file
    adaptation for the chapter's sample input files.
- Volume 2 Chapter 4, Solitaire (`v2ch4/solitaire.html`)
  - `examples/csls/volume2/v2_ch04_solitaire.lgo`
  - Full solitaire card-game program from the upstream companion listing.
- Volume 2 Chapter 6, BASIC Compiler (`v2ch6/basic.html`)
  - `examples/csls/volume2/v2_ch06_basic_compiler.lgo`
  - Full upstream `basic.lg` BASIC-to-Logo compiler listing.
- Volume 2 Chapter 7, Pattern Matcher (`v2ch7/match.html`)
  - `examples/csls/volume2/v2_ch07_pattern_matcher.lgo`
  - Full pattern-matcher transcript, including the complete engine and chapter
    demonstrations.
- Volume 2 Chapter 9, Doctor (`v2ch9/doctor.html`)
  - `examples/csls/volume2/v2_ch09_doctor.lgo`
  - Full upstream `doctormatch.lg` interactive Doctor/ELIZA-style program.
- Volume 2 Chapter 11, Cryptographer's Helper (`v2ch11/crypto.html`)
  - `examples/csls/volume2/v2_ch11_crypto_helper_full.lgo`
  - Full cryptogram-solving helper transcript, separate from the smaller
    deterministic helper example.
- Volume 2 Chapter 13, Fourier Series Plotter (`v2ch13/fourie.html`)
  - `examples/csls/graphics/v2_ch13_fourier_plotter.lgo`
  - Full Fourier-series plotter transcript, replacing the prior sampled sine
    wave stand-in.
- Volume 3 Chapter 2, Discrete Mathematics (`v3ch2/math.html`)
  - `examples/csls/volume3/v3_ch02_discrete_math_full.lgo`
  - Full upstream `math.lg` logic/discrete-math listing.
- Volume 3 Chapter 3, Algorithms and Data Structures (`v3ch3/algs.html`)
  - `examples/csls/volume3/v3_ch03_algorithms_full.lgo`
  - Full upstream `algs.lg` algorithms/data-structures listing.
- Volume 3 Chapter 4, Programming Language Design (`v3ch4/langd.html`)
  - `examples/csls/volume3/v3_ch04_language_design_pascal.lgo`
  - Full upstream `pascal.lg` Pascal/language-design listing.
- Volume 3 Chapter 5, Programming Language Implementation (`v3ch5/langi.html`)
  - `examples/csls/volume3/v3_ch05_language_implementation.lgo`
  - Full upstream `pascal.lg` Pascal/language-implementation listing.
- Volume 3 Chapter 6, Artificial Intelligence (`v3ch6/ai.html`)
  - `examples/csls/volume3/v3_ch06_artificial_intelligence.lgo`
  - Full upstream `student.lg` STUDENT algebra word-problem solver listing.

These files are credited to Brian Harvey and include source URLs. Some preserve
upstream UCBLogo behavior, interactivity, or file/graphics semantics that may
not yet run end-to-end in DynaLOGO.

## Deterministic application-style fixtures

The test fixture suite remains deliberately smaller and deterministic:

- Volume 2 Chapter 1, Data Files (`v2ch1/files.html`)
  - `crates/dynalogo-core/tests/csls_input/v2_ch01_file_streams.lgo`
  - Covers real file streams with `OPENWRITE`, `SETWRITE`, `OPENAPPEND`,
    `OPENREAD`, `SETREAD`, `READWORD`, `READLIST`, `EOFP`, `CLOSE`, and
    `DRIBBLE` using per-test scratch paths.
- Volume 2 Chapter 2, File Differences (`v2ch2/diff.html`)
  - `crates/dynalogo-core/tests/csls_input/v2_ch02_file_diff.lgo`
  - Modeled deterministic fixture that creates two scratch files and asserts a
    stable diff-style transcript using the chapter's `<`/`>` report
    conventions.
- Volume 2 Chapter 4, Solitaire (`v2ch4/solitaire.html`)
  - `crates/dynalogo-core/tests/csls_input/v2_ch04_solitaire_scripted.lgo`
  - Scripted-input fixture that pins the chapter's nested `CATCH`/`THROW`
    command-loop structure for deal/redisplay/help/give-up/exit commands.
- Volume 2 Chapter 5, Program as Data (`v2ch5/prgdat.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch05_program_as_data.lgo`
  - Covers `TEXT`, two-input UCBLogo `DEFINE`, redefinition with `LPUT` and
    `BUTLAST TEXT`, and procedure execution after generated definitions.
- Volume 2 Chapter 6, BASIC Compiler (`v2ch6/basic.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch06_basic_compiler.lgo`
  - Deterministic compiler fixture that emits a curated tiny BASIC program as
    generated Logo procedures with `DEFINE` and pins the resulting `FULLTEXT`
    source.
- Volume 2 Chapter 7, Pattern Matcher (`v2ch7/match.html`) and Chapter 9,
  Doctor (`v2ch9/doctor.html`)
  - `crates/dynalogo-core/tests/csls_input/v2_ch07_ch09_doctor_scripted.lgo`
  - Scripted multi-turn Doctor-style responder that pins pattern-response
    behavior and transcript normalization.
- Volume 2 Chapter 8, Property Lists (`v2ch8/plist.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch08_property_lists.lgo`
  - Covers the family-tree property-list application: `FAMILY`, `MOTHER`,
    `FATHER`, `KIDS`, `SONS`, `GRANDCHILDREN`, `GRANDDAUGHTERS`, `SIBLINGS`,
    and `COUSINS`.
- Volume 2 Chapter 11, Cryptographer's Helper (`v2ch11/crypto.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch11_crypto_ascii.lgo`
  - `crates/dynalogo-core/tests/csls/v2_ch11_crypto_helper.lgo`
  - Covers ASCII/CHAR helpers plus a deterministic substitution-cipher decoding
    transcript.
- Volume 2 Chapter 12, Macros (`v2ch12/macro.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch12_macros.lgo`
  - Covers generated instruction lists, `.MACRO`, `MACROEXPAND`, backquote,
    comma unquote, comma-at splicing, and `STOP` in macro-expanded code.
- Volume 2 Chapter 13, Fourier Series Plotter (`v2ch13/fourie.html`)
  - `crates/dynalogo-core/tests/csls_graphics/v2_ch13_fourier_plotter.lgo`
  - Modeled deterministic graphics fixture that validates a sampled
    sine/Fourier-style curve through the headless turtle trace harness.
- Volume 3 Chapter 5, Programming Language Implementation (`v3ch5/langi.html`)
  and Chapter 6, Artificial Intelligence (`v3ch6/ai.html`)
  - `crates/dynalogo-core/tests/csls/v3_ch05_ch06_language_ai_curated.lgo`
  - Curated deterministic fixture covering representative Pascal compiler and
    STUDENT mechanics: Logo programs as data, generated procedures via
    `DEFINE`, dynamic variable lookup, recursive list processing, and symbolic
    algebra rewrite rules.

## Validation

The deterministic fixture suite should remain the CI oracle. Full upstream
transcripts in `examples/csls/` are intentionally broader: when they do not run
end-to-end yet, they identify concrete DynaLOGO/UCBLogo compatibility gaps
rather than being reduced to simplified examples.
