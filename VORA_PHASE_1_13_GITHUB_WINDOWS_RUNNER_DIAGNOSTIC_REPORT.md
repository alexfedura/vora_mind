# VORA Phase 1.13 - GitHub-Hosted Windows Runner Diagnostic

## 1. Repository state

- Branch: `feat/e2-electron-foundation`
- HEAD: `b256ef85dd84ec847f1d3fafd836852a648ea4fb`
- Modified tracked files: 17
- Staged files: 0
- Untracked files: 433
- Untracked files under `loop/`: 395
- `chatmcp/`: absent

The R2 implementation and tests inspected for this phase exist in local worktree source. Several relevant persistence files are untracked and therefore are not guaranteed to be present in a GitHub Actions checkout until committed and pushed by an explicitly separate authorized operation.

No staging, commit, push, reset, clean, stash, or history operation was performed.

## 2. Workflow created

Workflow:

- `.github/workflows/phase-1-windows-runner-diagnostic.yml`

Trigger:

- `workflow_dispatch` only

Runner:

- `windows-2025`

Permissions:

- `contents: read`

No repository secrets, credentials, database files, marker files, or profile data are referenced.

## 3. Existing Actions pinning

The repository currently uses action major tags in existing workflows:

- `actions/checkout@v4`
- `actions/setup-node@v4`
- `pnpm/action-setup@v4`
- `mozilla-actions/sccache-action@v0.0.6`
- `swatinem/rust-cache@v2`
- `dtolnay/rust-toolchain@stable`
- `tauri-apps/tauri-action@v0`

The diagnostic workflow follows the repository's existing major-tag convention and uses:

- `actions/checkout@v4`
- `actions/setup-go@v5`

No branch, fork-PR, or automatic participant test trigger is used.

## 4. Workflow verification scope

The diagnostic workflow verifies:

- `RUNNER_ENVIRONMENT=github-hosted`
- Windows runner OS
- AMD64/x64 architecture
- PowerShell availability
- Go availability
- Go 1.26 module compatibility
- temporary storage above 1 GB
- user profile path availability without printing it
- GitHub ephemeral runner indicators
- disposable integration variables remain unset

It does not:

- execute participant lifecycle tests;
- execute DPAPI tests;
- create a disposable marker;
- execute Restore;
- print environment variables or account/profile identifiers;
- upload database or profile artifacts;
- access repository secrets.

## 5. Go and module compatibility

The module declares:

```text
go 1.26.0
```

The workflow asks `actions/setup-go@v5` to install Go `1.26.0` and verifies:

- `go version`;
- `go list -m -f "{{.GoVersion}}"`;
- `GOOS=windows`;
- `GOARCH=amd64`.

The workflow does not claim the installed runner toolchain passed if the workflow is not executed successfully on GitHub.

## 6. Disposable-profile guard compatibility

The existing guard in `go-app/internal/testguard/environment.go` requires:

- runtime Windows x64;
- `VORA_RUN_WINDOWS_PARTICIPANT_INTEGRATION=1`;
- `VORA_DISPOSABLE_PROFILE_MARKER` set;
- marker path equals `<UserConfigDir>/Vora Mind/disposable-windows-integration-test-profile.marker`;
- exact marker contents:
  `VORA_DISPOSABLE_WINDOWS_INTEGRATION_PROFILE_V1`.

GitHub-hosted runner compatibility assessment:

- Windows runner x64 is compatible with the runtime guard.
- A fresh ephemeral runner profile can theoretically satisfy the guard.
- The marker would need a later explicit, reviewed setup phase.
- The guard alone does not prove GitHub runner isolation or disposal safety.
- The runner must also prove separate helper processes, temp database isolation, named-pipe behavior, named-mutex behavior, and process abandonment.

The diagnostic workflow deliberately leaves both integration opt-in variables unset.

## 7. Existing integration selectors

The relevant selectors are:

- `TestParticipantConcurrentCrossProcessAggregatesAreFenced`
- `TestParticipantAbandonedAggregateFenceIsRecoverable`

Related helper:

- `TestParticipantHelperProcess`

The workflow does not execute these tests.

## 8. Security

Security properties:

- manual trigger only;
- minimal `contents: read` permission;
- no `pull_request_target`;
- no secrets;
- no artifact upload;
- no database/profile upload;
- no environment dump;
- no account or profile-path output;
- no DPAPI or HKCU setup;
- no participant integration opt-in.

## 9. Disk and toolchain considerations

The local host was not the execution target. GitHub-hosted runner disk and toolchain availability remain unknown until the workflow executes.

The workflow includes a 1 GB temporary-storage threshold and will fail if unavailable.

Potential limitations to validate in the next execution:

- Windows 2025 image availability;
- setup-go availability for Go 1.26.0;
- Go module dependency download access;
- temporary disk capacity;
- repository checkout size;
- generated cache or build storage.

## 10. Final classification

**GITHUB_WINDOWS_RUNNER_REQUIRES_CONFIGURATION**

The workflow is ready for manual GitHub execution, but GitHub-hosted runner properties have not been observed because this local phase did not run Actions.

The original hosted-runner check expected `RUNNER_ENVIRONMENT=github`. It has been corrected to require `RUNNER_ENVIRONMENT=github-hosted`; self-hosted runners are therefore rejected.

The workflow remains local and unpublished on the GitHub default branch. Actual GitHub runner execution remains pending and no runner validation is claimed.

Before profile validation:

- verify the workflow executes successfully;
- verify Windows/AMD64 and Go evidence;
- verify the relevant R2 source/tests are committed and available in the checkout;
- then create a separate reviewed profile-validation phase.
