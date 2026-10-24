# Policy Panel Version Label

## Goal

Show the running Vora application version beside the Policy Panel title so users
can identify which app release they are using.

## Requirements

- Display the synchronized application version in the Policy Panel header as
  `v<version>`.
- Match the Database Storage label's 13px medium font and panel text color.
- Read the version from `package.json`, which the release workflow keeps in sync
  with the Tauri and Rust package versions.
- Keep the version visible whenever the Policy Panel is open, including in
  development and production builds.
