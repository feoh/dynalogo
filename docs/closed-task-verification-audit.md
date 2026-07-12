# Closed-task verification audit inventory

This audit was created because prior completion claims need independent verification. It covers every closed Witan task returned for `https://github.com/feoh/dynalogo` at audit start.

- Audit project: `wp-dynalogo-closed-task-verification-audit-e69839`
- Parent audit epic: `tk-audit-closed-dynalogo-task-completion-claims-0d7c5a`
- Source branch/worktree: `pi/closed-task-verification-audit` from `origin/pi/ongoing-integration` (`ee33838`)
- Closed tasks inventoried: 126

## Verification subtasks

| Category | Witan task | Count | Scope |
|---|---:|---:|---|
| inventory | `tk-inventory-closed-tasks-and-expected-artifacts-65532a` | 126 | Map closed tasks to expected artifact categories. |
| vm/runtime/tests | `tk-verify-vm-primitive-and-runtime-task-artifacts-01a755` | 75 | Interpreter/runtime primitives, compiler/parser, UCBLogo/Atari behavior, fixtures/benchmarks. |
| frontend/wasm/release | `tk-verify-frontend-browser-wasm-release-artifacts-c25913` | 16 | Native/browser frontend, WASM shell, workflows, release automation, frontend tests. |
| docs/examples | `tk-verify-documentation-and-examples-task-artifacts-32620d` | 25 | User/developer docs, inventories/gaps, example programs. |
| cross-cutting | `tk-run-cross-cutting-validation-and-publish-audit-r-673919` | all | Full validation and final report. |

## Initial artifact smoke evidence

At inventory time, these claimed artifact classes are present on the integrated branch:

- Docs: `docs/getting-started.md`, `docs/reference-manual.md`,
  `docs/developer-guide.md`, `docs/wasm-and-browser.md`,
  `docs/debugging-and-errors.md`, `docs/browser-demo.md`,
  `docs/release-process.md`, `docs/releasing.md`, `docs/versioning.md`,
  `docs/ucblogo-compatibility.md`; the former UCBLogo error-audit document was
  retired after its remaining actionable wording gaps were covered by
  code/tests.
- Examples: `examples/dogs_in_the_park.lgo`, `examples/shape_parade.lgo`, `examples/spaceship_thrust.lgo`, `examples/bouncing_ball.lgo`, `examples/orbit_simulation.lgo`, `examples/pong_demons.lgo`, plus v0.1 examples.
- Workflows: `.github/workflows/ci.yml`, `pages.yml`, `publish.yml`, `release.yml`, `changelog.yml`
- Web artifacts: `web/index.html`, `web/mq_js_bundle.js`
- UCBLogo fixtures: `crates/dynalogo-core/tests/ucblogo/*.lgo`, `*.out`, and `ucblogo_compatibility.rs`
- Core fixtures: `crates/dynalogo-core/tests/fixtures/` contains arithmetic, control, templates, macros, negative_literals, catch_error, and unused-value cases.

This is not the final verdict; category subtasks must validate each claim in detail.

## Verification findings so far

### Confirmed artifacts and checks

- **Closed-task inventory:** 126 closed tasks were captured and categorized in the table above.
- **Core VM/runtime artifacts exist:** representative functions for macros, templates, `CASCADE.2`, `TRANSFER`, file/editor flows, `NODES`/`RECYCLE`, Atari graphics helpers, dynaturtle edge modes, shape registry, and code-4 numeric errors are present in `crates/dynalogo-core/src/vm.rs`.
- **Core VM/runtime tests exist:** regression tests cover macros/templates/negative literals, `events_since_clear`, Atari fill/pen/SETSCRUNCH, workspace lifecycle/editor flows, UCBLogo error-code behavior, shape registry, edge modes, speed, and item/numeric code-4 errors.
- **Frontend/browser/WASM artifacts exist:** `crates/dynalogo/src/bin/dynalogo-window.rs`, `web/index.html`, `web/mq_js_bundle.js`, `.github/workflows/{ci,pages,publish,release,changelog}.yml`, and browser/demo docs are present.
- **Frontend helper tests exist:** `dynalogo-window` has 25 unit tests covering coordinate transforms, heading vectors, sprite selection, custom shape point parsing, input queue behavior, browser-command filtering, and log retention. `web/shape_editor_test.js` now adds direct coverage for the browser shape-editor JavaScript.
- **Docs/examples artifacts exist:** all expected docs and example programs are present; relative Markdown links in docs/README/examples checked out with zero missing local targets.
- **Example smoke run:** every `examples/*.lgo` program executed successfully through the CLI with a 15-second timeout.
- **Validation commands:** after audit fixes, `ruby scripts/validate_workflows.rb`, `node web/shape_editor_test.js`, `cargo fmt --check`, `cargo test --workspace -q`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build -p dynalogo --bin dynalogo-window --target wasm32-unknown-unknown` have been run successfully. The workspace now reports 205 core VM tests plus the 25 frontend helper tests.

### Audit findings that required immediate fixes

1. **Formatting drift was real.** `cargo fmt --check` failed on `crates/dynalogo/src/bin/dynalogo-window.rs` and `crates/dynalogo-core/src/vm.rs`.
   - Corrective Witan task: `tk-fix-formatting-drift-found-by-closed-task-audit-bcb929`
   - Fix commit: `c505e52`
2. **Shape-editor/EDSH status wording was stale.** After `PUTSH`/`GETSH`/`SHAPE`, custom rendering, and the browser shape-editor panel landed, docs and the `EDSH` placeholder still said the shape registry/editor work was not implemented.
   - Corrective Witan task: `tk-fix-stale-edsh-and-shape-editor-wording-after-br-361223`
   - Fix commit: `48eefe3`
3. **Logo-level EDSH was still a placeholder.** The audit follow-up replaced it with a real `$EDITOR`-backed shape-definition editor.
   - Corrective Witan task: `tk-decide-and-implement-real-logo-level-edsh-flow-3ce826`
   - Fix commit: `18e0029`
4. **Numeric-input error parity was incomplete.** Direct non-arithmetic numeric call sites still had generic `is not a number` behavior or ad hoc range wording.
   - Corrective Witan task: `tk-finish-documented-remaining-ucblogo-error-parity-770844`
   - Fix commit: `e4a568e`

### Audit findings converted into follow-up tasks

These are not immediate artifact-existence failures, but the closed-task audit found that completion claims were broader than the current evidence/test coverage supports:

- `tk-finish-documented-remaining-ucblogo-error-parity-770844` — **fixed during audit**: all direct numeric-input call sites now use named code-4 handling, `SETITEM` bad-index wording is converted, and docs were updated. Remaining `REDUCE` empty-list wording still awaits live-UCBLogo verification.
- `tk-add-automated-browser-shape-editor-ui-tests-64f134` — **fixed during audit**: added `web/shape_editor_test.js`, a dependency-free Node test that extracts the actual inline shape-editor functions and verifies sample loading plus queued `PUTSH`/`SETSHAPE` commands. Fix commit: `9252d4a`.
- `tk-decide-and-implement-real-logo-level-edsh-flow-3ce826` — **fixed during audit**: `EDSH` now opens the existing editor flow on shape definitions rendered as `PUTSH` commands, with regression coverage. Fix commit: `18e0029`.
- `tk-add-workflow-yaml-validation-to-release-artifact-8c618c` — **fixed during audit**: added `scripts/validate_workflows.rb`, documented it, and wired it into CI for basic workflow YAML/structure validation. Fix commit: `15fde01`.

### Limitations of this audit pass

- Live UCBLogo comparison could not be executed because no `ucblogo`/`logo` binary is available in this environment. The committed compatibility fixtures are still valuable, but they are not a fresh live oracle run.
- Native GUI/audio behavior can be smoke-built and partially unit-tested through extracted pure helpers, but this environment does not perform visual/audio snapshot testing.
- GitHub Actions workflows are now checked by `scripts/validate_workflows.rb` for local YAML parsing and basic workflow/job/step structure, but they still have not been executed on GitHub in this audit worktree.

## Final audit outcome

The audit did **not** support the original blanket completion claim without qualifications. It found real issues, task-tracked them, and fixed the ones that were actionable in this pass:

- formatting drift (`c505e52`)
- stale EDSH/shape-editor status wording (`48eefe3`)
- incomplete numeric-input code-4 conversion and `SETITEM` range wording (`e4a568e`)
- missing automated browser shape-editor JavaScript test coverage (`9252d4a`)
- Logo-level `EDSH` still being a placeholder (`18e0029`)
- missing local workflow YAML/structure validation (`15fde01`)

The three exploratory subagents launched for VM/frontend/docs audit were not available for final reconciliation (`get_subagent_result` reported their handles had been cleaned up), so this report relies on direct Witan, git, source, and command-output evidence gathered in this worktree.

Final direct validation commands run successfully after all fixes:

```bash
ruby scripts/validate_workflows.rb
node web/shape_editor_test.js
cargo fmt --check
cargo test --workspace -q
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p dynalogo --bin dynalogo-window --target wasm32-unknown-unknown
```

Remaining limitations are documented above: no live UCBLogo binary was available, no visual/audio snapshot tests were run, and GitHub Actions were locally validated but not executed remotely from this worktree.

## Closed task inventory

| # | Category | Type | Slug | Title | Project |
|---:|---|---|---|---|---|
| 1 | docs/examples | epic | `tk-comprehensive-documentation-for-dynalogo-f9e150` | Comprehensive documentation for DynaLOGO | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle` |
| 2 | docs/examples | epic | `tk-comprehensive-documentation-for-dynalogo-3df005` | Comprehensive documentation for DynaLOGO | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-352144` |
| 3 | vm/runtime/tests | task | `tk-propagate-ucblogo-code-4-wording-through-number--8747b7` | Propagate UCBLogo code-4 wording through number_input-family call sites | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 4 | frontend/wasm/release | task | `tk-interactive-turtle-shape-editor-ui-5fdb5b` | Interactive turtle shape editor UI | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 5 | frontend/wasm/release | task | `tk-custom-shape-rendering-in-native-and-wasm-fronte-e8f020` | Custom shape rendering in native and WASM frontends | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 6 | vm/runtime/tests | task | `tk-verify-and-convert-remaining-item-index-range-uc-30ef50` | Verify and convert remaining ITEM/index-range UCBLogo errors | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 7 | docs/examples | task | `tk-full-ucblogo-error-parity-follow-through-from-re-0e7947` | Full UCBLogo error parity follow-through from README | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 8 | vm/runtime/tests | task | `tk-shape-registry-and-putsh-getsh-shape-primitives-eb7b6e` | Shape registry and PUTSH/GETSH/SHAPE primitives | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 9 | vm/runtime/tests | epic | `tk-add-a-turtle-shape-sprite-editor-43eb1d` | Add a turtle shape/sprite editor | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 10 | vm/runtime/tests | task | `tk-verify-and-convert-remaining-empty-word-and-inde-8cb540` | Verify and convert remaining empty-word and index-range UCBLogo errors | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 11 | docs/examples | task | `tk-debugging-and-error-reference-guide-141f3d` | Debugging and error reference guide | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 12 | docs/examples | epic | `tk-comprehensive-documentation-for-dynalogo-879a85` | Comprehensive documentation for DynaLOGO | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 13 | docs/examples | task | `tk-wasm-build-and-browser-embedding-guide-19115a` | WASM build and browser embedding guide | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 14 | docs/examples | task | `tk-developer-guide-for-vm-internals-and-primitive-e-0fad2b` | Developer guide for VM internals and primitive extension | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 15 | vm/runtime/tests | task | `tk-workspace-editor-parity-follow-up-edns-and-edsh-b66729` | Workspace editor parity follow-up EDNS and EDSH | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 16 | docs/examples | task | `tk-workspace-file-parity-polish-from-readme-follow--031862` | Workspace/file parity polish from README follow-on | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 17 | vm/runtime/tests | task | `tk-finish-primitive-level-ucblogo-error-wording-par-8411dc` | Finish primitive-level UCBLogo error wording parity | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 18 | frontend/wasm/release | epic | `tk-create-github-actions-for-packaging-and-releases-b2b85d` | Create Github Actions for packaging and releases | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 19 | frontend/wasm/release | epic | `tk-write-a-thorough-test-suite-for-dynalogos-graphi-a3d9e2` | Write a thorough test suite for DynaLOGOs graphical interface | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 20 | frontend/wasm/release | task | `tk-frontend-turtleevent-replay-and-clear-state-test-4eed19` | Frontend TurtleEvent replay and clear-state tests | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 21 | frontend/wasm/release | task | `tk-versioning-and-changelog-automation-for-releases-be1d75` | Versioning and changelog automation for releases | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 22 | frontend/wasm/release | task | `tk-publish-dynalogo-crates-via-github-actions-098f7b` | Publish dynalogo crates via GitHub Actions | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 23 | vm/runtime/tests | task | `tk-expand-ucblogo-error-compatibility-fixture-cover-783589` | Expand UCBLogo error compatibility fixture coverage | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 24 | frontend/wasm/release | task | `tk-build-native-github-release-artifacts-for-dynalo-a7ac83` | Build native GitHub release artifacts for dynalogo | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 25 | frontend/wasm/release | task | `tk-frontend-sprite-and-coordinate-transform-tests-09399d` | Frontend sprite and coordinate-transform tests | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 26 | frontend/wasm/release | task | `tk-frontend-input-queue-and-repl-log-tests-35dc8a` | Frontend input queue and REPL log tests | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 27 | vm/runtime/tests | task | `tk-audit-ucblogo-error-wording-and-code-map-8dc9bd` | Audit UCBLogo error wording and code map | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 28 | vm/runtime/tests | task | `tk-workspace-lifecycle-primitives-nodes-and-recycle-17a6ff` | Workspace lifecycle primitives NODES and RECYCLE | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 29 | vm/runtime/tests | task | `tk-full-fill-rendering-parity-follow-up-after-fille-67e81f` | Full fill rendering parity follow-up after FILLED helper | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 30 | vm/runtime/tests | task | `tk-setscrunch-coordinate-scaling-follow-up-after-br-86a616` | SETSCRUNCH coordinate scaling follow-up after branch merge | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 31 | vm/runtime/tests | task | `tk-atari-graphics-parity-follow-up-filled-multi-pen-ab896b` | Atari graphics parity follow-up: FILLED/multi-pen/SETSCRUNCH | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 32 | docs/examples | task | `tk-browser-wasm-polish-follow-through-from-readme-9f8db9` | Browser/WASM polish follow-through from README | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 33 | docs/examples | task | `tk-browser-specific-onboarding-and-docs-polish-3d7a1c` | Browser-specific onboarding and docs polish | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 34 | docs/examples | task | `tk-browser-example-gallery-and-loader-polish-124c48` | Browser example gallery and loader polish | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 35 | vm/runtime/tests | task | `tk-pen-mode-semantics-follow-up-after-multi-pen-por-525290` | Pen-mode semantics follow-up after multi-pen port | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 36 | docs/examples | task | `tk-additional-dynaturtle-compatibility-surface-from-3d9758` | Additional dynaturtle compatibility surface from README | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 37 | frontend/wasm/release | task | `tk-browser-demo-html-shell-and-pages-deploy-d9c067` | Browser demo HTML shell and Pages deploy | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 38 | vm/runtime/tests | task | `tk-dynaturtle-edge-mode-primitives-and-speed-report-f5eb29` | Dynaturtle edge-mode primitives and SPEED reporter | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 39 | inventory/other | chore | `tk-merge-useful-unmerged-branch-stack-into-main-7bc709` | Merge useful unmerged branch stack into main | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 40 | rollup/epic | epic | `tk-e3-v0-3-rich-language-core-314b6a` | E3: v0.3 — Rich language core | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 41 | vm/runtime/tests | epic | `tk-e7-v1-0-full-ucblogo-library-parity-polish-88af31` | E7: v1.0 — Full UCBLogo library parity + polish | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 42 | vm/runtime/tests | task | `tk-ucblogo-compatibility-test-suite-b7a611` | UCBLogo compatibility test suite | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 43 | docs/examples | task | `tk-primitive-audit-vs-ucblogo-manual-6b8dd7` | Primitive audit vs UCBLogo manual | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 44 | docs/examples | task | `tk-syntax-and-feature-validation-against-atari-logo-755e63` | Syntax and feature validation against Atari LOGO reference manual | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 45 | vm/runtime/tests | feature | `tk-test-iftrue-iffalse-catch-throw-error-pause-cont-45ab38` | TEST/IFTRUE/IFFALSE, CATCH/THROW/ERROR, PAUSE/CONTINUE/WAIT | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 46 | vm/runtime/tests | epic | `tk-e5-v0-5-macros-performance-b93a48` | E5: v0.5 — Macros & performance | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 47 | vm/runtime/tests | feature | `tk-ucblogo-accurate-error-messages-84944c` | UCBLogo-accurate error messages | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 48 | vm/runtime/tests | epic | `tk-e4-v0-4-workspace-i-o-ec0c10` | E4: v0.4 — Workspace & I/O | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 49 | docs/examples | task | `tk-user-docs-classic-example-programs-ad840d` | User docs + classic example programs | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 50 | vm/runtime/tests | feature | `tk-workspace-management-primitives-99dfd2` | Workspace management primitives | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 51 | frontend/wasm/release | epic | `tk-e6-v0-6-web-wasm-target-b20535` | E6: v0.6 — Web/WASM target | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 52 | frontend/wasm/release | feature | `tk-browser-frontend-github-pages-demo-027562` | Browser frontend + GitHub Pages demo | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 53 | frontend/wasm/release | feature | `tk-wasm-build-core-cooperative-sim-scheduling-8cb1bd` | WASM build: core + cooperative sim scheduling | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 54 | docs/examples | epic | `tk-extensive-user-documentation-0913b3` | Extensive user documentation | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 55 | docs/examples | task | `tk-reference-manual-84dbf9` | Reference manual | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 56 | docs/examples | task | `tk-getting-started-tutorial-491432` | Getting Started Tutorial | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 57 | docs/examples | feature | `tk-dynaturtle-polish-sprites-sound-demo-gallery-6455cc` | Dynaturtle polish: sprites, sound, demo gallery | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 58 | vm/runtime/tests | task | `tk-performance-pass-benchmarks-1-000-turtles-60-hz-141536` | Performance pass + benchmarks (1,000 turtles @ 60 Hz) | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 59 | vm/runtime/tests | task | `tk-end-to-end-1-000-turtle-60hz-benchmark-final-per-0fd56b` | End-to-end 1,000-turtle @ 60Hz benchmark + final perf pass | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 60 | vm/runtime/tests | task | `tk-classic-turtle-motion-pen-primitives-don-t-targe-4fd689` | Classic turtle motion/pen primitives don't target the TurtleStore active selection | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 61 | vm/runtime/tests | bug | `tk-parser-should-handle-adjacent-negative-literals--55a6aa` | Parser should handle adjacent negative literals in instruction lists | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 62 | vm/runtime/tests | feature | `tk-macros-macro-defmacro-macrop-macroexpand-05b774` | Macros: .MACRO/.DEFMACRO/MACROP/MACROEXPAND | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 63 | docs/examples | feature | `tk-dynaturtle-example-dogs-in-a-park-with-bark-on-c-373988` | Dynaturtle example: dogs in a park with bark-on-collision | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 64 | vm/runtime/tests | bug | `tk-template-source-reserialization-should-preserve--ed7f20` | Template source reserialization should preserve literal words that shadow primitive names | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 65 | vm/runtime/tests | feature | `tk-implement-cascade-2-and-transfer-template-follow-b59dc3` | Implement CASCADE.2 and TRANSFER template follow-ups | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 66 | vm/runtime/tests | feature | `tk-full-template-forms-cascade-transfer-c185c8` | Full template forms + CASCADE/TRANSFER | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 67 | inventory/other | feature | `tk-edit-via-editor-eeb485` | EDIT via $EDITOR | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 68 | vm/runtime/tests | feature | `tk-file-i-o-load-save-streams-149e9f` | File I/O: LOAD/SAVE + streams | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 69 | vm/runtime/tests | feature | `tk-remaining-graphics-label-fill-multiple-pens-cbf245` | Remaining graphics: LABEL/FILL/multiple pens | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 70 | vm/runtime/tests | feature | `tk-graphics-follow-up-atari-multi-pen-and-pen-mode--5b1b0f` | Graphics follow-up: Atari multi-pen and pen-mode semantics | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 71 | vm/runtime/tests | feature | `tk-graphics-follow-up-setscrunch-coordinate-scaling-0eb248` | Graphics follow-up: SETSCRUNCH coordinate scaling | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 72 | vm/runtime/tests | feature | `tk-graphics-follow-up-filled-native-flood-fill-sema-05e06e` | Graphics follow-up: FILLED + native flood-fill semantics | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 73 | vm/runtime/tests | feature | `tk-finish-atari-when-collision-event-table-semantic-78afcf` | Finish Atari WHEN collision/event table semantics | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 74 | vm/runtime/tests | feature | `tk-implement-atari-file-device-and-outside-world-pr-0690b7` | Implement Atari file, device, and outside-world primitives | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 75 | vm/runtime/tests | feature | `tk-implement-atari-device-and-outside-world-helpers-4aff74` | Implement Atari device and outside-world helpers | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 76 | vm/runtime/tests | feature | `tk-implement-atari-file-stream-primitives-158a9e` | Implement Atari file stream primitives | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 77 | vm/runtime/tests | feature | `tk-implement-atari-turtle-addressing-and-event-prim-cf422f` | Implement Atari turtle addressing and event primitives | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 78 | vm/runtime/tests | feature | `tk-complete-remaining-atari-helper-predicates-and-l-a2058f` | Complete remaining Atari helper predicates and list/math utilities | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 79 | vm/runtime/tests | feature | `tk-implement-atari-dot-helper-primitive-a2ff79` | Implement Atari DOT helper primitive | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 80 | vm/runtime/tests | feature | `tk-implement-atari-list-sorting-helper-procedures-a51621` | Implement Atari list-sorting helper procedures | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 81 | docs/examples | feature | `tk-implement-initial-atari-useful-tools-library-pro-206d22` | Implement initial Atari Useful Tools library procedures | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 82 | vm/runtime/tests | feature | `tk-implement-atari-init-turtle-helper-9bae48` | Implement Atari INIT.TURTLE helper | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 83 | vm/runtime/tests | feature | `tk-implement-simple-atari-turtle-state-primitives-53251b` | Implement simple Atari turtle state primitives | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 84 | vm/runtime/tests | feature | `tk-implement-atari-define-primitive-5d1331` | Implement Atari DEFINE primitive | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 85 | vm/runtime/tests | feature | `tk-implement-atari-workspace-listing-and-erase-prim-5edcbb` | Implement Atari workspace listing and erase primitives | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 86 | vm/runtime/tests | feature | `tk-implement-atari-type-and-math-predicate-surface-52e963` | Implement Atari type and math predicate surface | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 87 | docs/examples | task | `tk-draft-initial-primitive-gap-list-from-current-in-b75f06` | Draft initial primitive gap list from current inventory | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 88 | vm/runtime/tests | feature | `tk-add-copydef-workspace-primitive-6b8724` | Add COPYDEF workspace primitive | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 89 | vm/runtime/tests | task | `tk-add-text-fulltext-workspace-inspection-primitive-8e54c5` | Add TEXT/FULLTEXT workspace inspection primitives | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 90 | docs/examples | task | `tk-create-current-primitive-inventory-snapshot-8e7f58` | Create current primitive inventory snapshot | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 91 | vm/runtime/tests | task | `tk-support-expected-error-logo-fixtures-in-integrat-7efeb2` | Support expected-error Logo fixtures in integration tests | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 92 | vm/runtime/tests | task | `tk-add-definedp-and-primitivep-workspace-predicates-17471c` | Add DEFINEDP and PRIMITIVEP workspace predicates | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 93 | vm/runtime/tests | task | `tk-detect-unused-values-in-statement-context-d72d29` | Detect unused values in statement context | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 94 | vm/runtime/tests | task | `tk-add-initial-file-based-logo-integration-test-har-b513b5` | Add initial file-based Logo integration test harness | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 95 | docs/examples | task | `tk-add-quickstart-docs-and-v0-1-example-programs-289d49` | Add quickstart docs and v0.1 example programs | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 96 | vm/runtime/tests | task | `tk-improve-parser-and-vm-error-propagation-semantic-c3a33c` | Improve parser and VM error propagation semantics | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 97 | vm/runtime/tests | bug | `tk-make-should-update-local-bindings-and-procedure--10b413` | MAKE should update local bindings and procedure args | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 98 | vm/runtime/tests | feature | `tk-library-control-structures-for-while-until-do-wh-a452ce` | Library control structures: FOR/WHILE/UNTIL/DO.WHILE/CASE/COND | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 99 | vm/runtime/tests | feature | `tk-property-lists-arrays-5805e0` | Property lists + arrays | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 100 | vm/runtime/tests | feature | `tk-templates-map-filter-reduce-foreach-apply-placeh-8a3452` | Templates: MAP/FILTER/REDUCE/FOREACH/APPLY + ? placeholders | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 101 | vm/runtime/tests | epic | `tk-e2-v0-2-dynaturtles-d7a04c` | E2: v0.2 — Dynaturtles | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 102 | vm/runtime/tests | feature | `tk-sim-vm-thread-repl-command-channel-fc104e` | Sim/VM thread + REPL command channel | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 103 | inventory/other | feature | `tk-when-demons-fuel-budget-scheduler-800b53` | WHEN demons + fuel-budget scheduler | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 104 | vm/runtime/tests | feature | `tk-edge-modes-setshape-b3e704` | Edge modes + SETSHAPE | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 105 | inventory/other | feature | `tk-velocity-model-setspeed-setvelocity-continuous-m-183fdf` | Velocity model: SETSPEED/SETVELOCITY continuous motion | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 106 | vm/runtime/tests | feature | `tk-collision-detection-spatial-hash-touching-0c26c7` | Collision detection: spatial hash + TOUCHING | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 107 | vm/runtime/tests | feature | `tk-multi-turtle-soa-store-tell-ask-each-who-1caf38` | Multi-turtle SoA store + TELL/ASK/EACH/WHO | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 108 | vm/runtime/tests | feature | `tk-fixed-timestep-sim-loop-interpolated-rendering-1cd927` | Fixed-timestep sim loop + interpolated rendering | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 109 | vm/runtime/tests | epic | `tk-e1-v0-1-core-language-static-turtle-graphics-2fc004` | E1: v0.1 — Core language + static turtle graphics | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 110 | frontend/wasm/release | feature | `tk-macroquad-frontend-window-static-turtle-graphics-3b4587` | macroquad frontend: window + static turtle graphics | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 111 | vm/runtime/tests | feature | `tk-turtlebackend-trait-headless-test-harness-cc31ca` | TurtleBackend trait + headless test harness | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 112 | inventory/other | feature | `tk-terminal-repl-b0cd85` | Terminal REPL | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 113 | frontend/wasm/release | chore | `tk-ci-github-actions-fmt-clippy-test-c90ba8` | CI: GitHub Actions (fmt, clippy, test) | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 114 | rollup/epic | epic | `tk-e0-project-scaffolding-github-setup-7e7f17` | E0: Project scaffolding & GitHub setup | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 115 | vm/runtime/tests | feature | `tk-procedure-definition-to-end-argument-binding-af7bf5` | Procedure definition: TO…END, argument binding | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 116 | vm/runtime/tests | feature | `tk-v0-1-primitive-set-416990` | v0.1 primitive set | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 117 | vm/runtime/tests | feature | `tk-stack-vm-dynamic-scope-tco-output-stop-d7ca58` | Stack VM: dynamic scope, TCO, OUTPUT/STOP | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 118 | inventory/other | feature | `tk-bytecode-compiler-chunk-cache-9cf61a` | Bytecode compiler + chunk cache | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 119 | vm/runtime/tests | feature | `tk-parser-instruction-lists-with-arity-driven-expre-e4e615` | Parser: instruction lists with arity-driven expression grouping | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 120 | inventory/other | feature | `tk-value-types-words-numbers-lists-6aecdb` | Value types: words, numbers, lists | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 121 | vm/runtime/tests | feature | `tk-lexer-reader-ucblogo-tokenization-7bc4fa` | Lexer/reader: UCBLogo tokenization | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 122 | vm/runtime/tests | task | `tk-scaffold-cargo-workspace-repo-files-85aea2` | Scaffold cargo workspace + repo files | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 123 | inventory/other | task | `tk-create-feoh-dynalogo-github-repo-and-push-6ade89` | Create feoh/dynalogo GitHub repo and push | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 124 | vm/runtime/tests | bug | `tk-vm-missing-multi-turtle-language-primitives-tell-181d17` | VM missing multi-turtle language primitives (TELL/ASK/EACH/WHO/SETVELOCITY/SETSPEED/WHEN) | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 125 | docs/examples | task | `tk-graphics-follow-up-validate-filled-edge-semantic-e03a5a` | Graphics follow-up: validate FILLED edge semantics against Atari manual | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
| 126 | vm/runtime/tests | feature | `tk-finish-atari-timeout-and-text-screen-mode-helper-c56712` | Finish Atari TIMEOUT and text-screen mode helpers | `wp-dynalogo-ucblogo-compatible-logo-with-dynaturtle-8c37a7` |
