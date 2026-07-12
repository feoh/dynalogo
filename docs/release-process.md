# Release process

Pushing a tag matching `v*.*.*` (e.g. `v0.1.0`) triggers the
[`release.yml`](../.github/workflows/release.yml) workflow, which builds
native binaries for the `dynalogo` (REPL) and `dynalogo-window` (native
turtle window) frontends and attaches them to a GitHub Release:

- Linux: `x86_64-unknown-linux-gnu` (`.tar.gz`)
- macOS: `aarch64-apple-darwin` (`.zip`)
- Windows: `x86_64-pc-windows-msvc` (`.zip`)

Each archive bundles the two binaries alongside `README.md`, `LICENSE`, and
the `examples/` directory.

The workflow can also be run manually via `workflow_dispatch` to sanity-check
the build matrix without publishing a release (the publish job only runs on
tag pushes).

## Known limitations

- **No Intel macOS build.** Only `aarch64-apple-darwin` is built, matching
  GitHub's arm64 `macos-latest` runners. An `x86_64-apple-darwin` cross-build
  could be added later if there's demand.
- **No code signing or notarization.** macOS binaries are unsigned, so
  Gatekeeper will require the user to explicitly allow the app (System
  Settings → Privacy & Security) before it will run. Windows binaries are
  unsigned as well and may trigger SmartScreen warnings.
- **No installers.** Releases ship as plain archives, not `.dmg`, `.msi`, or
  packaged installers.
