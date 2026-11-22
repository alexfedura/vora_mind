# VORA Phase 3.9 Migration Priority Matrix

| Candidate | Importance | Centrality | Complexity | Security/Data Risk | Testability | Rank |
|---|---:|---:|---:|---:|---:|---:|
| Projects and workspace roots | 10 | 10 | 6 | 8 | 9 | 1 |
| Sources and provenance | 10 | 9 | 8 | 9 | 8 | 2 |
| Research and evidence | 10 | 9 | 9 | 9 | 7 | 3 |
| Workflow and AI execution | 9 | 9 | 7 | 9 | 9 | 4 |
| Visual boards/media | 8 | 7 | 9 | 7 | 7 | 5 |

The ranking is qualitative and evidence-based; scores are recommendations, not measured benchmarks.

## Recommendation

Select **projects and workspace roots** for the first implementation domain.

Reasons:

- It is the foundation for project isolation across all later domains.
- The Go implementation already contains project CRUD and workspace-root behavior.
- It has existing tests for isolation and reopening.
- It can expose a reusable project-ownership contract to sources and evidence.
- It is smaller than a scientific-domain migration and can establish domain/repository/service patterns safely.

The first implementation must extend project ownership rather than only finish CRUD. It should define contracts for project-scoped entities and no-secret audit/error behavior.
