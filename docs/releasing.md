# Releasing DynaLOGO

DynaLOGO releases are distributed as GitHub Release archives, not crates.io
packages. Pushing a `vX.Y.Z` tag (or running the release workflow manually for a
build-matrix sanity check) builds native packages for the terminal REPL and
native turtle window.

## Cutting a release

1. Bump `workspace.package.version` **and** the `dynalogo-core` entry under
   `workspace.dependencies` in the root `Cargo.toml` (both must match), and
   update `CHANGELOG.md`.
2. Commit that bump on `main` and let CI pass.
3. Tag the commit and push the tag:

   ```sh
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

4. The release workflow (`.github/workflows/release.yml`) builds Linux, macOS
   arm64, and Windows archives and attaches them to the GitHub Release.

## Distribution policy

The Rust crates are workspace implementation units for the DynaLOGO app and test
harness. They are intentionally not published to crates.io unless a concrete
external library/API consumer appears and the public API is designed for that
use case. Keeping distribution on GitHub Releases avoids advertising internal
crate APIs as a stable Rust library surface.
