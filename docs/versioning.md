# Versioning and changelog

## Version number

The workspace version lives in one place:
`[workspace.package].version` in [`Cargo.toml`](../Cargo.toml). Both
workspace crates (`dynalogo-core`, `dynalogo`) inherit it via
`version.workspace = true`, so bumping the version is a single edit.

## Policy

DynaLOGO follows [Semantic Versioning](https://semver.org/). For historical
pre-1.0.0 releases:

- `0.x.0` (minor) bumps may include breaking changes to the language,
  primitives, or CLI/API surface — this matches the version roadmap in
  [ROADMAP.md](../ROADMAP.md), where each `0.x` line corresponds to a
  development milestone rather than a stability guarantee.
- `0.x.y` (patch) bumps are for fixes and small additions within a
  milestone that don't change the primitive surface.

Starting with 1.0.0, standard SemVer applies: patch for fixes, minor for
backward-compatible additions, major for breaking changes.

## Changelog

User-facing changes are tracked in [`CHANGELOG.md`](../CHANGELOG.md),
following [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Add an
entry under `## [Unreleased]` in the same pull request that makes the
change — this is checked by CI (see
[`.github/workflows/changelog.yml`](../.github/workflows/changelog.yml)).

Not every change needs a changelog entry: internal refactors, test-only
changes, and doc fixes can skip it. The CI check only requires *some*
edit to `CHANGELOG.md` when source under `crates/` changes, so it can't
tell whether the entry is meaningful — use judgment.

## Cutting a release

Release automation is provided by the tag-triggered
[`release.yml`](../.github/workflows/release.yml) workflow. Cutting a release
still requires a deliberate version/changelog commit:

1. Move the `## [Unreleased]` entries into a new `## [x.y.z] - YYYY-MM-DD`
   section in `CHANGELOG.md`, and update the compare/tag links at the bottom.
2. Bump `workspace.package.version` in `Cargo.toml` and the matching
   `dynalogo-core` workspace dependency version when starting `x.y.z`.
3. Run the local validation and package checks, commit the release metadata,
   and push the commit to `main`.
4. Create and push tag `vx.y.z`. The release workflow builds platform archives
   and publishes them as GitHub Release assets.
