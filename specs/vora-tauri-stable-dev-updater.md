# Vora Stable/Dev Builds and Signed Updates

## Goal

Allow a Vora development build to run beside the installed stable build, publish signed Windows stable releases, and install stable updates.

## Design

- Keep the installed stable bundle identifier unchanged and apply a JSON Merge Patch with a `.dev` identifier to Tauri development runs.
- The distinct identifier prevents the development build from sharing the stable app identity (and any single-instance identity) so both can run at once.
- Keep Vite's frontend dev server separate from the Tauri `dev` command to avoid recursively invoking Tauri from `beforeDevCommand`.
- Register Tauri v2 updater and process plugins with the required window permissions.
- Enable signed NSIS updater artifacts and point the updater manifest at the public GitHub Releases latest feed.
- Build Windows releases from version tags using `tauri-apps/tauri-action`; the workflow verifies the tag matches the app version and publishes the signed updater manifest and NSIS updater archive to a GitHub Release.
- Configure NSIS to install for the current user so app installation and updates remain in the user's profile without elevation. Existing per-machine MSI installations require a one-time migration/reinstall to switch to a per-user installation.
- Build release artifacts with Cargo's `ci` profile and restore Rust build artifacts through the GitHub Actions Rust cache.
- Expose a manual update check in the stable app and automatically check every 60 seconds. The backend downloads and verifies the signed update, then delegates installation to the Tauri updater plugin. On Windows, the plugin handles the signed NSIS updater archive and starts the installer in quiet mode.
- Log update installation failures and surface errors to the frontend. Do not request administrator elevation for the per-user NSIS install.
- Do not use an elevated scheduled task for updates. A persistent highest-privilege task that consumes installer paths from user-writable storage would create a local privilege-escalation path.
- Keep the existing local release commands as legacy helpers; version tags start the automated release workflow.
- Never commit or store the updater private key in the repository; configure it only as GitHub Actions secrets.

## One-time Setup

- Generate a key pair outside the repository with `pnpm tauri signer generate -w "$env:USERPROFILE\.tauri\vora-updater.key"`.
- Add the generated private key contents and its password to the GitHub Actions secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Never commit the private key.
- Put the generated public key in `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`. This is a one-time setup step; the matching private key must be used by CI to sign artifacts.
- Commit the setup; version tags will create future signed releases automatically.

## Release and Local Use

- Run `pnpm dev` to launch the development build beside the installed stable build. Its config overlay uses a `.dev` bundle identifier.
- Run `pnpm release:verify` to check that `package.json`, `tauri.conf.json`, and `Cargo.toml` have one synchronized semantic version.
- Create a version tag to start the Windows release workflow. It builds signed NSIS updater artifacts and publishes `latest.json`, the updater archive, and its signature to the GitHub Release.
- The stable app checks for updates on launch and every 60 seconds, and also exposes a manual check. On Windows, the frontend invokes `install_update`; the Rust command checks the configured updater endpoint and downloads/verifies the update before the Tauri updater plugin installs it. Download progress is emitted to the frontend.
- The Windows updater uses the Tauri updater plugin directly; no detached helper binary is built or bundled.
- The repository must be public so updater clients can download `latest.json` and the matching installer assets from GitHub Releases without credentials.

## Verification

- Frontend TypeScript/build passes.
- Rust checks with updater and process plugins enabled.
- Tauri config validates with the updater artifact settings.
- The signed local build helper verifies the NSIS updater archive signature (`*.exe.zip.sig`) beside the archive.
- The release workflow runs on version tags, verifies the tag version, and publishes updater assets to a non-draft GitHub Release.
- `plugins.updater.endpoints` points to `https://github.com/alexfedura/vora_mind/releases/latest/download/latest.json`.
