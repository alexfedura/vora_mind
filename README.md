# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Stable and Development Builds

Run `pnpm dev` to start Vora's development build with the `.dev` Tauri identifier. This keeps the development app's identity separate from the installed stable app, allowing both to run at the same time (including when the single-instance plugin is enabled). The app's Vite server is started by `pnpm dev:frontend`.

### Signed Windows Releases and Automatic Updates

The `Release` GitHub Actions workflow runs when a `v*` version tag is pushed and builds and publishes a signed Windows release. Installed production builds check the GitHub Releases updater manifest on launch and every minute, install a signed update quietly, and relaunch. Update checks are disabled for development builds and failures are logged without blocking startup.

On Windows, Rust/Tauri build artifacts, Rust Analyzer checks, Vite's dependency cache, and frontend build output are stored under `C:\vora-compile` to keep generated files off the workspace drive. Rust outputs (including packaged bundles) are in `C:\vora-compile\cargo-target`; Rust Analyzer uses its `rust-analyzer` subdirectory; Vite's dependency cache is in `C:\vora-compile\vite-cache`; frontend assets are in `C:\vora-compile\frontend`.

`C:\vora-compile\frontend` is a build-output folder, not a web address to open directly. To launch the app, use `pnpm dev` or install the generated Tauri installer. To inspect the frontend in a browser, run `pnpm preview` after `pnpm build` and open `http://localhost:4173`.

One-time updater setup:

1. Generate a signing key pair with `pnpm tauri signer generate -w "$HOME/.tauri/vora.key"` (create the `.tauri` directory first if needed). Keep the generated private key out of the repository.
2. Put the generated public key in `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`.
3. Add the private key contents and its password as the GitHub Actions secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

To manually select a release version, ensure the working tree is clean and run `pnpm release:patch`, `pnpm release:minor`, or `pnpm release:major`. The script synchronizes the package, Tauri, Cargo, and Cargo lock versions, creates a conventional release commit and annotated `vX.Y.Z` tag, and pushes them to `origin`; the push to `main` starts the release workflow. Use `pnpm release:verify` to check synchronized versions, or `node scripts/release.js patch --dry-run` to preview the next version.

## Vora Trace

Vora Trace is available as a workspace in the left sidebar. Its graph, decision Markdown, and project registry are stored under Vora Mind's AppData directory. On first launch, existing Trace files are copied from `D:\workspace\vora_trace`; the source project is left unchanged. Git history search uses the selected existing project in the Trace ecosystem sidebar.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
