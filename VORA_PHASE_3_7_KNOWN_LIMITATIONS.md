# VORA Phase 3.7 Known Limitations

- Actual independent VS Code MCP tool calls remain pending because the live chat UI was unstable and the AX target changed during submission attempts.
- Restore execution was not run against the real database. Automated isolated restore/rollback tests passed.
- Full GUI workflow approval and deny flows remain pending in the Wails UI.
- Symlink escape is not exercised natively because Windows symlink privileges differ in this environment.
- HTTP filesystem parity remains partial and should be documented as an intentional transport boundary unless product requirements change.
