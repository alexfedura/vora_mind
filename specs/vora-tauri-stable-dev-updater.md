# Vora Stable/Dev Builds and Signed Updates

## Goal

Allow a Vora development build to run beside the installed stable build, publish signed Windows stable releases, and install stable updates.

## Design

- Keep the installed stable bundle identifier unchanged and apply a JSON Merge Patch with a `.dev` identifier to Tauri development runs.
- The distinct identifier prevents the development build from sharing the stable app identity (and any single-instance identity) so both can run at once.
- Keep Vite's frontend dev server separate from the Tauri `dev` command to avoid recursively invoking Tauri from `beforeDevCommand`.
- Register Tauri v2 updater and process plugins with the required window permissions.
- Enable signed MSI updater artifacts and point the updater manifest at the public GitHub Releases latest feed.
- Build Windows releases from version tags using `tauri-apps/tauri-action`; the workflow verifies the tag matches the app version and publishes the signed updater manifest and MSI to a GitHub Release.
- Build release artifacts with Cargo's `ci` profile and restore Rust build artifacts through the GitHub Actions Rust cache; reuse preinstalled WiX when available and install it only when missing.
- Expose a manual update check in the stable app and automatically check every 60 seconds. On Windows, the backend downloads and verifies the signed update, then hands it to a detached helper that waits for the app to exit, installs quietly, and relaunches the app. Other platforms continue using the Tauri updater plugin installation flow.
- If installation fails, show a native error dialog before releasing the update resource. Do not request administrator elevation by default.
- Configure Windows updater installations for quiet mode.
- Bundle a separate Windows updater helper executable as a Tauri resource. The app copies it and the verified installer into app-local data before starting it with `DETACHED_PROCESS` and `CREATE_NEW_PROCESS_GROUP`.
- The helper opens a synchronization handle for the main app and signals readiness before the app exits, preventing it from racing the app shutdown. It waits for the app process, runs MSI (`/quiet /norestart`) or NSIS (`/S`) installers to completion, and relaunches the current executable after success.
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
- Create a version tag to start the Windows release workflow. It builds signed MSI updater artifacts and publishes `latest.json`, the installer, and its signature to the GitHub Release.
- The stable app checks for updates on launch and every 60 seconds, and also exposes a manual check. It downloads signed updates in the background and asks whether to restart once the download is ready. Installation errors are logged and shown in a native dialog.
- Build the helper with `pnpm build:updater-helper` before bundling locally; `pnpm tauri:build` and the release workflow build it automatically.
- The repository must be public so updater clients can download `latest.json` and the matching installer assets from GitHub Releases without credentials.

## Verification

- Frontend TypeScript/build passes.
- Rust checks with updater and process plugins enabled.
- Tauri config validates with the updater artifact settings.
- The signed local build helper verifies that MSI (`*.msi.sig`) and NSIS (`*.exe.sig`) signatures exist beside their installers.
- The release workflow runs on version tags, verifies the tag version, and publishes updater assets to a non-draft GitHub Release.
- `plugins.updater.endpoints` points to `https://github.com/alexfedura/vora_mind/releases/latest/download/latest.json`.
