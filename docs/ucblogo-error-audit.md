# UCBLogo error wording and code-map audit

This is a snapshot audit of `crates/dynalogo-core/src/vm.rs` error reporting as
of this commit, done in support of the broader UCBLogo error-parity effort
tracked by `tk-full-ucblogo-error-parity-follow-through-from-re-0e7947`. It
maps every place the VM raises a `VmError`, notes whether it already carries a
UCBLogo-style numeric error code, and flags wording that should be revisited
against real UCBLogo output before the parity task is considered done.

No live UCBLogo/Berkeley Logo binary was available in this environment, so
nothing below has been re-verified against a running reference interpreter.
Anything not already covered by the compatibility harness
(`docs/ucblogo-compatibility.md`) should still be treated as unverified.

## 1. Error codes already assigned (`infer_error_info` / `error_with_code`)

These are the only codes the VM currently attaches to `ERROR`/`CATCH "ERROR`
results. Each now has a dedicated unit test in `crates/dynalogo-core/src/vm.rs`
(`catch_error_records_*`) pinning both the code and the message text, so a
future wording change will show up as a test failure instead of silently
drifting:

| Code | Message pattern | Trigger | Test |
| --- | --- | --- | --- |
| 5 | `X didn't output to Y` | a value-producing call site received `NoValue` | `catch_error_records_missing_output_code_and_context` |
| 6 | `not enough inputs to X` | arity check failed for primitive/procedure `X` | `catch_error_records_unknown_procedure_code_and_context` (13), pre-existing `catch_error_populates_structured_error_list_and_consumes_it` (6) |
| 9 | `You don't say what to do with X` | a top-level/RUN expression produced an unused value | `catch_error_records_unused_value_code_and_context` |
| 11 | `X has no value` | unbound variable lookup via `:X` or `THING` | `catch_error_records_missing_variable_code_and_context` |
| 13 | `I don't know how to X` | call to an undefined procedure name | `catch_error_records_unknown_procedure_code_and_context` |
| 25 | `IFTRUE/IFFALSE without TEST` | `IFTRUE`/`IFFALSE` called before `TEST` ran in the frame | `catch_error_records_iftrue_without_test_code_and_context` |
| 35 | value of a `(THROW "ERROR value)` | user-level `THROW "ERROR` | `catch_error_records_custom_throw_error_code_and_message` |

These seven map onto real UCBLogo's numbered error scheme and the wording
matches idioms documented in prior work (see the "Improve UCBLogo-style error
reporting" history on this repo). Treat this table, plus its tests, as the
current source of truth for the code map.

## 2. Sites still producing DynaLOGO-only wording (no numeric code)

The remaining ~100 `VmError::new(...)` call sites in `vm.rs` do not go through
`error_with_code` and are not recognized by `infer_error_info`, so `ERROR`
reports them with code `0` and whatever ad hoc string was passed in. They fall
into a few buckets:

### 2a. Datum type-mismatch messages (highest-value follow-up bucket)

Real UCBLogo's canonical wrong-type-input message is the `X doesn't like Y as
input` idiom (e.g. `FIRST doesn't like [] as input`). Most of dynalogo's
current wording instead says `Y is not a Z`, which reads differently even
though the situation is the same case UCBLogo assigns error code 4 (`DATUM
doesn't like INPUT as input`). These are the primary candidates for the next
wording pass:

- `vm.rs:4052` — `"{value} is not a number"` (used by all arithmetic/number
  inputs, e.g. `number_input`)
- `vm.rs:4058` — `"{value} is not a variable name"`
- `vm.rs:4071` — `"{value} is not a property-list key"`
- `vm.rs:4093`, `4108`, `4123`, `4133` — variable-name-list/input-name/
  input-list/body-line-list checks for `TO`/`DEFINE`
- `vm.rs:1536` — `"{X} is not a list"`
- `vm.rs:1610` — `"{X} is not a sort tree"`
- `vm.rs:1446` — `"FPUT second input must be a list"`
- `vm.rs:1457` — `"LPUT second input must be a list"`
- `vm.rs:2711` — `"SETITEM second input must be an array"`
- `vm.rs:2730` — `"ARRAYTOLIST input must be an array"`
- `vm.rs:3055` — `"template must be a word or a list"`
- `vm.rs:4194` — `"{name} input must be a list"`
- `vm.rs:4244`/`4247`/`4249` — `"SETPOS requires a two-number list"`

### 2b. Empty-datum / out-of-range messages

Also part of UCBLogo's code-4 family (or a distinct "index out of range"
case), currently phrased as ad hoc English rather than the classic idiom:

- `vm.rs:1353`/`1356` — `"FIRST of empty list"` / `"FIRST of empty array"`
- `vm.rs:1403`/`1409` — `"LAST of empty list"` / `"LAST of empty array"`
- `vm.rs:4722`/`4730` — `"FIRST of empty word"` / `"LAST of empty word"`
- `vm.rs:1563`/`1571`/`4231` — `"RANPICK of empty list/array/word"`
- `vm.rs:1522`/`1525`/`4740`/`4745` — `"ITEM index out of range"`
- `vm.rs:1691` — `"ASCII of empty word"`
- `vm.rs:2904` — `"REDUCE cannot reduce an empty list"`

### 2c. Internal/defensive invariants (likely not user-reachable)

These guard bytecode invariants the compiler should already uphold; they are
unlikely to ever surface from valid Logo source, so they are low priority for
UCBLogo wording parity:

- `vm.rs:740` — `"instruction pointer ran past end of chunk"`
- `vm.rs:780`/`783` — `"infix operator missing right/left input"`
- `vm.rs:3995` — `"not enough inputs: expected {argc}"` (note: this duplicates
  the intent of the code-6 `"not enough inputs to X"` message at `vm.rs:4005`
  but with different wording and no procedure name — worth reconciling if it
  ever turns out to be reachable)

### 2d. File/stream/editor plumbing

Wraps `std::io::Error` text or describes editor/file-handle state
(`vm.rs:240`, `518`, `525`, `1663`, `1683`, `1800`, `1820`, `1831`, `1875`,
`1904`, `1920`, `1922`, `1935`, `1937`, `1957`, `1959`, `2476`, `2555`, `2558`,
`2565`, `2569`, `4440`). UCBLogo's own file-error wording is itself
OS-dependent, so these are lower priority than the pure-language cases above.

### 2e. DynaLOGO-extension-specific wording (intentionally out of scope)

Not UCBLogo surface at all, so no UCBLogo idiom applies:
`vm.rs:3751` (`TELL`), `vm.rs:3872`/`3875`/`3891` (`WHEN`), and the Atari/
dynaturtle-adjacent primitives referenced in `docs/primitive-gaps.md`.

## 3. Recommendation

This task should stay a precursor to the larger parity work, not a closer:
seven codes are now mapped and regression-tested, but roughly 90 other error
sites (bucket 2a/2b above) still use ad hoc wording with no error code, and
none of it has been checked against a live UCBLogo interpreter. The highest
-value next slice is converting bucket 2a/2b to the `X doesn't like Y as
input` idiom (UCBLogo error code 4) and giving each a code-tagged regression
test the same way this pass did for codes 5/9/11/25/35.
