# Releasing to crates.io

Publishing is manual-trigger: pushing a `vX.Y.Z` tag (or running the
workflow via `workflow_dispatch`) is what starts it. Nothing publishes on a
plain push to `main`.

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
   fmt/clippy/tests, then publishes `dynalogo-core` first and `dynalogo`
   second, since `dynalogo` depends on `dynalogo-core` as a path dependency
   that only resolves once the former is on crates.io.

There is no automatic rollback: a bad publish must be yanked by hand with
`cargo yank -p <crate> --version <version>`.
