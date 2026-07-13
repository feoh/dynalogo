# CSLS application example coverage

Audit date: 2026-07-13

This note tracks larger Computer Science Logo Style application examples and how the current deterministic fixture suite handles them.

## Imported / validated deterministic application-style examples

- Volume 2 Chapter 1, Data Files (`v2ch1/files.html`)
  - `crates/dynalogo-core/tests/csls_input/v2_ch01_file_streams.lgo`
  - Covers real file streams with `OPENWRITE`, `SETWRITE`, `OPENAPPEND`, `OPENREAD`, `SETREAD`, `READWORD`, `READLIST`, `EOFP`, `CLOSE`, and `DRIBBLE` using per-test scratch paths.
- Volume 2 Chapter 2, File Differences (`v2ch2/diff.html`)
  - `crates/dynalogo-core/tests/csls_input/v2_ch02_file_diff.lgo`
  - Modeled deterministic fixture that creates two scratch files and asserts a stable diff-style transcript using the chapter's `<`/`>` report conventions.
- Volume 2 Chapter 4, Solitaire (`v2ch4/solitaire.html`)
  - `crates/dynalogo-core/tests/csls_input/v2_ch04_solitaire_scripted.lgo`
  - Modeled scripted-input fixture that pins the chapter's nested `CATCH`/`THROW` command-loop structure for deal/redisplay/help/give-up/exit commands.
- Volume 2 Chapter 5, Program as Data (`v2ch5/prgdat.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch05_program_as_data.lgo`
  - Covers `TEXT`, two-input UCBLogo `DEFINE`, redefinition with `LPUT`/`BUTLAST TEXT`, and procedure execution after generated definitions.
- Volume 2 Chapter 6, BASIC Compiler (`v2ch6/basic.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch06_basic_compiler.lgo`
  - Modeled deterministic compiler fixture that emits a curated tiny BASIC program as generated Logo procedures with `DEFINE` and pins the resulting `FULLTEXT` source.
- Volume 2 Chapter 7, Pattern Matcher (`v2ch7/match.html`) and Chapter 9, Doctor (`v2ch9/doctor.html`)
  - `crates/dynalogo-core/tests/csls_input/v2_ch07_ch09_doctor_scripted.lgo`
  - Modeled scripted multi-turn Doctor-style responder that pins pattern-response behavior and transcript normalization.
- Volume 2 Chapter 8, Property Lists (`v2ch8/plist.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch08_property_lists.lgo`
  - Covers the family-tree property-list application: `FAMILY`, `MOTHER`, `FATHER`, `KIDS`, `SONS`, `GRANDCHILDREN`, `GRANDDAUGHTERS`, `SIBLINGS`, and `COUSINS`.
- Volume 2 Chapter 11, Cryptographer's Helper (`v2ch11/crypto.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch11_crypto_ascii.lgo`
  - `crates/dynalogo-core/tests/csls/v2_ch11_crypto_helper.lgo`
  - Covers ASCII/CHAR helpers plus a deterministic substitution-cipher decoding transcript.
- Volume 2 Chapter 12, Macros (`v2ch12/macro.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch12_macros.lgo`
  - Covers generated instruction lists, `.MACRO`, `MACROEXPAND`, backquote, comma unquote, comma-at splicing, and `STOP` in macro-expanded code.
- Volume 2 Chapter 13, Fourier Series Plotter (`v2ch13/fourie.html`)
  - `crates/dynalogo-core/tests/csls_graphics/v2_ch13_fourier_plotter.lgo`
  - Modeled deterministic graphics fixture that validates a sampled sine/Fourier-style curve through the headless turtle trace harness.
- Volume 3 Chapter 5, Programming Language Implementation (`v3ch5/langi.html`) and Chapter 6, Artificial Intelligence (`v3ch6/ai.html`)
  - `crates/dynalogo-core/tests/csls/v3_ch05_ch06_language_ai_curated.lgo`
  - Curated deterministic fixture covering representative Pascal compiler and STUDENT mechanics: Logo programs as data, generated procedures via `DEFINE`, dynamic variable lookup, recursive list processing, and symbolic algebra rewrite rules.

## Excluded from deterministic fixtures for now

No audited application examples are currently excluded from deterministic coverage. Some imported fixtures are curated/model fixtures instead of full upstream whole-program ports when the book program is too large or exploratory for a stable CI oracle.

## Validation

After importing the deterministic examples above and adding compatibility support needed by them, the full `cargo test` suite passed.
