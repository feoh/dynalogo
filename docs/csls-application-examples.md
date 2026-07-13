# CSLS application example coverage

Audit date: 2026-07-13

This note tracks larger Computer Science Logo Style application examples and how the current deterministic fixture suite handles them.

## Imported / validated deterministic application-style examples

- Volume 2 Chapter 1, Data Files (`v2ch1/files.html`)
  - `crates/dynalogo-core/tests/csls_input/v2_ch01_file_streams.lgo`
  - Covers real file streams with `OPENWRITE`, `SETWRITE`, `OPENAPPEND`, `OPENREAD`, `SETREAD`, `READWORD`, `READLIST`, `EOFP`, `CLOSE`, and `DRIBBLE` using per-test scratch paths.
- Volume 2 Chapter 5, Program as Data (`v2ch5/prgdat.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch05_program_as_data.lgo`
  - Covers `TEXT`, two-input UCBLogo `DEFINE`, redefinition with `LPUT`/`BUTLAST TEXT`, and procedure execution after generated definitions.
- Volume 2 Chapter 8, Property Lists (`v2ch8/plist.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch08_property_lists.lgo`
  - Covers the family-tree property-list application: `FAMILY`, `MOTHER`, `FATHER`, `KIDS`, `SONS`, `GRANDCHILDREN`, `GRANDDAUGHTERS`, `SIBLINGS`, and `COUSINS`.
- Volume 2 Chapter 12, Macros (`v2ch12/macro.html`)
  - `crates/dynalogo-core/tests/csls/v2_ch12_macros.lgo`
  - Covers generated instruction lists, `.MACRO`, `MACROEXPAND`, backquote, comma unquote, comma-at splicing, and `STOP` in macro-expanded code.

## Excluded from deterministic fixtures for now

These examples are intentionally not yet imported as whole-program fixtures because the book behavior is interactive, graphics-driven, or too large for a clear non-interactive expected-output oracle without additional harness design:

- Volume 2 Chapter 2, file differences (`v2ch2/diff.html`): depends on comparing arbitrary external text files. A useful fixture should generate two scratch files and assert a stable diff transcript.
- Volume 2 Chapter 4, Solitaire (`v2ch4/solitaire.html`): interactive/game state example with user choices.
- Volume 2 Chapter 6, BASIC compiler (`v2ch6/basic.html`): large compiler application; should be imported only with a curated sample BASIC source and expected generated Logo/source output.
- Volume 2 Chapter 7, Pattern Matcher (`v2ch7/match.html`) and Chapter 9 Doctor (`v2ch9/doctor.html`): conversational/pattern-matching examples requiring scripted multi-turn input and transcript normalization.
- Volume 2 Chapter 11, Cryptographer's Helper (`v2ch11/crypto.html`): puzzle-assistance application with exploratory/user-guided output rather than one canonical solution transcript.
- Volume 2 Chapter 13, Fourier Series Plotter (`v2ch13/fourie.html`): graphical plotting output; should be validated through the graphics trace/raster harness rather than stdout.
- Volume 3 Chapter 5, language implementation (`v3ch5/langi.html`) and Chapter 6 AI (`v3ch6/ai.html`): larger interpreter/AI examples requiring separate curated input programs and stable transcripts.

## Validation

After importing the deterministic examples above and adding compatibility support needed by them, the full `cargo test` suite passed.
