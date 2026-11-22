# VORA Phase 3.5 Known Limitations

- Windows DPAPI provider requires actual Windows user-context acceptance.
- Non-Windows OS protection is intentionally unavailable unless the test
  provider is explicitly enabled.
- Third-party desktop MCP client acceptance is pending.
- Visual GUI interaction, loading states, and sensitive-value behavior require
  a real GUI session or automation unavailable in this environment.
- Project filesystem roots require full project metadata/lifecycle integration.
- Restore UI controls are not yet complete.
