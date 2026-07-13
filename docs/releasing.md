# Releasing to crates.io

Publishing is manual-trigger: pushing a `vX.Y.Z` tag (or running the
workflow via `workflow_dispatch`) is what starts it. Nothing publishes on a
plain push to `main`. If `CARGO_REGISTRY_TOKEN` is absent, the workflow still
runs validation but skips the crates.io publish steps cleanly.

## One-time setup

- Create a crates.io API token with publish rights for `dynalogo-core` and
  `dynalogo`.
- Add it as the `CARGO_REGISTRY_TOKEN` secret on the `crates-io` GitHub
  Environment for this repo. Using an environment (rather than a repo-level
  secret) lets you gate publishes behind required reviewers if desired.

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

4. The `Publish crates` workflow (`.github/workflows/publish.yml`) runs
   fmt/clippy/tests. When `CARGO_REGISTRY_TOKEN` is configured, it then
   publishes `dynalogo-core` first and `dynalogo` second, since `dynalogo`
   depends on `dynalogo-core` as a path dependency that only resolves once the
   former is on crates.io. When the token is not configured, the publish steps
   are skipped and the GitHub Release artifacts are still produced by
   `.github/workflows/release.yml`.

A local `cargo package -p dynalogo-core` preflight can fully verify before the
release tag. A local `cargo package -p dynalogo` preflight for the same new
version is expected to fail until `dynalogo-core` for that version is visible in
the crates.io index; the workflow handles that by publishing `dynalogo-core`
first and waiting briefly before publishing `dynalogo`.

There is no automatic rollback: a bad publish must be yanked by hand with
`cargo yank -p <crate> --version <version>`.
