## Release rule (always-on)

After completing ANY task that modifies files in this repository, you MUST:
1. Bump the version in `package.json` (patch by default; minor for new
   features). Mirror it in `src-tauri/tauri.conf.json` and
   `src-tauri/Cargo.toml` if present.
2. Run `git add -A`.
3. Commit with a Conventional Commits message
   (e.g. `feat(scope): ...` or `chore(release): vX.Y.Z`).
4. Run `git push origin HEAD`.

Do not wait for the user to ask. This is a permanent instruction.
If the release hook already committed this session
(`git log -1 --grep="[auto]"`), skip silently — do not double-commit.
---
