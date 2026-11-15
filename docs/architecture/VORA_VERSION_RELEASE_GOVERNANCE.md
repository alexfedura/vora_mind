# VORA MIND Version, Lockfile, and Release Governance

## A. Policy decisions and status

The project owner has approved the following policies:

1. Track `src-tauri/Cargo.lock` in Git.
2. Require explicit authorization for release-related actions. No agent-session
   or Stop-hook side effect may bump a version, stage changes, commit, or push.
3. Keep releases manual and separately authorized. Version changes, tags,
   GitHub Releases, web deployments, and desktop updater publication must not
   happen automatically.
4. Treat `package.json` as the authoritative application-version source.
   `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` must match it.

These are approved policies, **not implemented controls**. This document
records the policy and a proposed implementation sequence; it does not
authorize implementation or a release. Approval here does not authorize
editing versions, staging, committing, pushing, tagging, publishing, or
deploying.

| Area | Approved future policy | Current observed implementation |
|---|---|---|
| Version authority | `package.json` is authoritative; the Tauri and Cargo declarations match it. | The three declarations currently read `0.24.7`. `scripts/release.js` can synchronize them, but there is also a Stop hook with its own automatic patch bump. |
| Rust dependency lock | Review and track `src-tauri/Cargo.lock`; locked builds use that reviewed file. | The local file is ignored and untracked. Its contents and dependency resolution have not been reviewed or approved. |
| Release actions | Each action requires explicit authorization; releases are manual. | A Stop hook can bump, stage, commit, and push. A pushed `v*` tag triggers the release workflow. |
| Publication | Verify GitHub release, updater, and web deployment separately. | Prior audit found no `v0.24.7` tag or GitHub Release; the inspected updater feed advertised `0.24.0`. Vercel status for `0.24.7` was not verified. |

### Status terminology

- **Approved policy** means the owner has selected the rule for future
  implementation.
- **Current behavior** means behavior observed in the repository or prior
  read-only audits; it is not proof of authorization.
- **Required future implementation** means work that must be separately
  authorized and validated.
- **Unresolved** means the available evidence does not settle the point.

## B. Version governance

### Approved authority

`package.json` is the single source of truth for the application version.
`src-tauri/tauri.conf.json` and the `[package].version` in
`src-tauri/Cargo.toml` are dependent declarations and must match it exactly.
Version changes require explicit owner authorization; unrelated work must
never cause a patch bump.

### Proposed validation and release preparation

1. Read the version from `package.json`.
2. Compare it with the Tauri and Cargo declarations and fail if either differs
   or if the value is not an allowed stable semantic version.
3. During explicitly authorized release preparation, update `package.json`
   first, then deterministically synchronize the two dependent declarations.
4. Run the version verifier against all three declarations and, when a tag is
   being prepared, verify that the tag name is exactly `v` plus that version.
5. Do not create a commit or tag as a side effect of validation or
   synchronization.

### Existing behavior

`scripts/release.js` already has `verify` and `sync` operations. Its
verification checks agreement among the three declarations; the release
workflow invokes verification against its tag. Its synchronization operation
uses the `package.json` version to update the Tauri and Cargo declarations.
Release commands also interact with the local Cargo lockfile when it exists.
The patch/minor/major release path can stage files, commit, create an annotated
tag, and push. It therefore is not merely a validator and must not run without
the corresponding explicit authorization.

## C. Cargo lockfile governance

### Approved future state

- `src-tauri/Cargo.lock` must be tracked in Git.
- Before adding it, review the existing local ignored file and explicitly
  approve the dependency resolution represented by its contents. Until that
  review and adoption occur, the local file is not authoritative.
- CI and release builds must use the committed lockfile and locked dependency
  resolution, such as `cargo --locked` where applicable.
- Rust toolchain, Tauri build tooling, and other build tools must be pinned or
  otherwise explicitly controlled for reproducible builds.
- Review a lockfile diff as a dependency-resolution change. Lockfile updates
  require explicit authorization; developers and agents must not silently
  regenerate, replace, or discard the lockfile.

### Current observed state

Prior repository audits found `src-tauri/Cargo.lock` locally present but
ignored and untracked. `.gitignore` contains both a broad `Cargo.lock` pattern
and an explicit `src-tauri/Cargo.lock` pattern. The reason for the original
ignore policy has not been established, and the local lockfile has not been
read or approved. No lockfile occurrence was found in reachable Git history.

`docs/audit/RECON.md` records that Cargo lockfiles are ignored and that
deterministic Rust dependency locking is not guaranteed by the tracked
repository. That is an audit finding, not an explanation for why the policy
was selected. The local ignored file must not be treated as the approved
lockfile or as evidence of reproducible CI.

## D. Automatic release hook governance

### Approved future behavior

No Stop hook or other implicit automation may:

- bump a version;
- run `git add` or otherwise stage changes;
- create a commit or push;
- create a tag or GitHub Release;
- deploy a web application; or
- publish desktop updater metadata or artifacts.

Each release-related action requires explicit authorization for that action.
Authorization to change a version does not imply authorization to stage,
commit, push, tag, release, deploy, or publish updater artifacts. Normal
development, audit, test, and documentation work must not trigger release
actions.

Suppression variables and marker files are defense-in-depth, not a substitute
for removing unauthorized side effects. Any future automation must have
separate dry-run and execution paths; the execution path must require explicit
confirmation.

### Current behavior and alignment work

`.github/hooks/release.json` configures a Stop hook that runs
`scripts/auto-release.ps1`. When enabled and it finds a dirty worktree, that
script increments the patch version in the three declarations, runs
`git add -A`, commits with `--no-verify`, and pushes `origin HEAD`. It can
skip under `SKIP_AUTO_RELEASE=true`, `CI=true`, or when `.skip-auto-release`
exists. The hook does not pass the script's `-DryRun` option. These safeguards
do not make an implicit mutating hook consistent with the approved policy.

The current `.github/workflows/release.yml` runs on a pushed `v*` tag and uses
the Tauri release action to create a non-draft GitHub Release with updater
metadata and signed NSIS assets. Thus tag creation/push currently triggers
publication. Future work must change the hook configuration and script to
remove implicit mutations, and revise the workflow's trigger/approval path so
publication follows separately authorized execution. Any web deployment
pipeline must be identified and governed separately; its current
correspondence to `0.24.7` is unknown.

## E. Manual release procedure

This is a proposed checklist. Preparation, authorization, execution, and
post-release verification are separate phases.

### Preparation

1. Confirm the worktree is clean and identify the intended source commit.
2. Confirm the proposed release scope and release notes.
3. Review the Cargo lockfile and the proposed lockfile diff separately.
4. Select the application version in `package.json`.
5. Synchronize `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
6. Validate all three declarations and the intended `vX.Y.Z` tag relationship.
7. Run the required frontend and Rust checks with frozen/locked dependency
   resolution, plus the relevant tests.
8. Review the exact files and intended commit contents.

### Separate authorization and execution

9. Obtain explicit owner authorization for the release.
10. Obtain explicit authorization for version and lockfile changes, staging,
    commit, branch push, and tag creation as separate actions where applicable.
11. Create and verify the intended commit only after commit authorization.
12. Create and push the intended version tag only after tag authorization.
13. Run the release workflow only after publication authorization.
14. Authorize and perform any web deployment separately.

### Post-release verification

15. Verify CI completion and the exact commit/tag it built.
16. Verify GitHub Release publication, target commit, installer, signature,
    and release assets.
17. Verify updater metadata advertises the intended version and points to the
    expected signed artifacts; verify the configured feed itself.
18. Verify web deployment independently, if applicable.
19. Record the exact commit, tag, GitHub Release, updater-feed result, and web
    deployment evidence.

A successful push, successful CI run, published GitHub Release, usable desktop
updater publication, and actual web deployment are distinct events. A
successful build alone is not a completed release.

## F. Historical state and unresolved items

The following are historical read-only audit findings, not claims that a
release is currently available:

- Commit `057dca9476d53d6f36a11e4bc3a82d51ddb10cfa` is the recorded
  `0.24.7` version-bump commit.
- No `v0.24.7` GitHub Release was found.
- The inspected GitHub updater feed advertised `0.24.0`.
- Whether Vercel had a deployment corresponding to `0.24.7` was not verified.
- The intended release status of `0.24.7` requires owner clarification.
- The current automatic hook's version/stage/commit/push side effects conflict
  with the approved policy.
- This policy has not yet been implemented.
- The local ignored Cargo lockfile has not been inspected or approved; its
  internal package version and dependency resolution remain unknown.

No statement here implies that `0.24.7` was successfully released or that
users can receive it through the updater.

## G. Future implementation plan

Each item below is a separate proposed task. Each requires explicit owner
authorization before implementation.

### 1. Cargo lockfile review and tracking

- **Scope:** Review the existing local lockfile without silently replacing it;
  approve its dependency resolution, then adopt and track it.
- **Expected files:** `src-tauri/Cargo.lock`, `.gitignore`, and relevant
  documentation.
- **Validation:** Review the complete lockfile diff; verify the tracked file
  supports locked Rust validation/builds.
- **Risks:** The ignored file may contain an outdated or locally resolved
  dependency graph; adopting it may surface incompatibilities or a large
  dependency update.

### 2. Automatic release hook governance correction

- **Scope:** Remove implicit version, staging, commit, and push side effects;
  provide a separately authorized release path if automation remains.
- **Expected files:** `.github/hooks/release.json`,
  `scripts/auto-release.ps1`, and potentially explicit release-command
  documentation.
- **Validation:** Demonstrate that normal Stop events and dirty audit/
  documentation work do not alter files or Git state; test dry-run and
  explicitly confirmed execution separately.
- **Risks:** Existing workflows may rely on the hook; removing it may expose
  undocumented assumptions. Do not preserve an automatic mutation as a
  fallback.

### 3. Version synchronization and validation

- **Scope:** Make `package.json` the unambiguous authority; synchronize and
  verify the dependent declarations without bundling unrelated Git actions.
- **Expected files:** `scripts/release.js`, `package.json` scripts, and
  potentially version-related documentation.
- **Validation:** Test matching versions, mismatches, invalid versions, and
  tag/version mismatches; verify validation does not stage, commit, push, or
  tag.
- **Risks:** Current release commands combine version edits with Git
  mutations; separating those responsibilities may change operator workflow.

### 4. CI and Rust dependency reproducibility

- **Scope:** Require the approved tracked lockfile, locked Rust resolution,
  controlled Rust/Tauri tooling, and explicit release checks.
- **Expected files:** `.github/workflows/release.yml`, and possibly
  `src-tauri/Cargo.toml` or toolchain configuration.
- **Validation:** Confirm CI fails when the lockfile is missing or would
  change; verify the pinned build and test commands on the supported runner.
- **Risks:** Pinning toolchains/actions and enforcing `--locked` can reveal
  drift or break existing build assumptions.

### 5. Manual release and updater verification procedure

- **Scope:** Separate preparation from authorization, execution, and
  publication verification; gate release publication explicitly.
- **Expected files:** `.github/workflows/release.yml`,
  `specs/vora-tauri-stable-dev-updater.md`, `README.md`, and possibly
  `scripts/release.js`.
- **Validation:** Exercise a dry-run without mutations; for an explicitly
  authorized release, verify target commit, release assets, signature,
  updater manifest, and feed version.
- **Risks:** A tag-triggered workflow currently publishes automatically;
  changing the trigger or approval gates may affect existing release
  operations and updater expectations.

### 6. Release-state verification and documentation

- **Scope:** Provide a repeatable evidence-based check for tags, GitHub
  Releases, CI, updater publication, and any web deployment without conflating
  them.
- **Expected files:** `docs/audit/RECON.md`, this governance document, and
  any separately approved verification tooling. The web deployment entry
  point must first be identified.
- **Validation:** Compare a known published release with an unpublished
  version; record exact commit, tag, run, assets, updater-feed payload, and
  deployment evidence or state what is unavailable.
- **Risks:** External services may be unavailable or have delayed/cached
  metadata; reports must distinguish unknown from absent.

## H. Stage 3.2 gate

Stage 3.2 remains blocked until all of the following are satisfied:

1. The release hook no longer causes unauthorized changes.
2. `src-tauri/Cargo.lock` has been reviewed and tracked under the approved
   policy.
3. CI reproducibility requirements are implemented and verified.
4. Version synchronization is validated.
5. Manual release and updater verification procedures are documented and
   tested.
6. Repository state and release publication status are understood.
7. The owner explicitly authorizes Stage 3.2.

Policy approval and this planning document do not constitute implementation
acceptance or authorization to begin Stage 3.2.
