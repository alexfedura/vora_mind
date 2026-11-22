# VORA MIND Autonomy and Approval Policies

## Operation classes

1. Read-only
2. Reversible write
3. External network
4. File creation/modification
5. Destructive
6. High-impact

## Policy states

- Allowed
- Denied
- ApprovalRequired

## Current implementation

Human and internal contexts receive project/database/filesystem permissions.
AI contexts receive project/database/read-only filesystem permissions.
External API contexts receive project read/write and database inspection
permissions only. Filesystem write is not granted to external API contexts.

Workflow approval gates persist approval status on the specific step. Approval
does not authorize unrelated actions.
