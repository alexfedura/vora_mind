# ADR-004: Time, Numeric Precision, and Immutable Asset References

- **Status:** Accepted for Phase 0 logical baseline; scientific-domain range/unit governance pending.
- **Date:** 2026-11-11
- **Decision owners:** Architecture proposal; scientific data owners not recorded.
- **Scope:** Cross-domain values, measurements, timestamps, structured JSON and external payloads.

## Context

The current repository has mixed timestamp units across domains, text/JSON scientific fields, SQLite `REAL` market values, and filesystem/Parquet assets whose complete backup coverage is unknown. Scientific data cannot safely rely on JavaScript floating-point values, filename-derived split roles, or implicit local time.

## Decision

- System instants use UTC Unix microseconds (`i64`/safe integer JSON), with `_at_us`/`_time_us` naming. Civil dates remain ISO date strings. Measurement time additionally records precision/offset/zone/original text where needed; an unknown/date-only value is never fabricated into an instant.
- Scientific decimal values are canonical base-10 strings with explicit unit code/dimension and uncertainty where relevant. Rust validates exact syntax/range; TypeScript never coerces them to `number`; SQLite stores exact decimal text, not `REAL`.
- Missingness is a typed state distinct from numeric zero, empty string and SQL `NULL`.
- Core query/lifecycle/identity fields are columns and FKs. JSON is versioned by `schema_id` and `schema_version`, validated by Rust, and used only for typed payload/configuration or noncritical extension. Unknown schema versions are preserved and surfaced.
- Immutable files have a workspace asset manifest with opaque ID, SHA-256, byte length, media type, storage-relative key and integrity state. Scientific records pin asset identity and captured hash. Stage/hash durable bytes before committing DB references.
- Complete workspace backup must explicitly account for DB, settings and declared external assets; no DB-only backup is described as full restore.

## Alternatives considered

- Epoch milliseconds: rejected as a universal time contract because sample/source time precision varies and old values have unverified units.
- ISO local timestamp strings: rejected for instants because timezone/offset ambiguity harms ordering and comparison.
- `f64` for all numeric measures: rejected due to rounding and precision issues, especially prices/currency and exact decimal quantities.
- Infer missingness from `NULL`, empty JSON or zero: rejected because these states have different scientific meanings.
- Store all payloads in SQLite BLOBs: not selected; larger scientific datasets/assets may be external and need content-addressed manifests.
- Store external file paths alone: rejected because files can move/change and the DB cannot verify content.

## Consequences and unresolved policy

Exact decimals require parsing and range validation at the Rust boundary; JavaScript consumers handle branded decimal strings. Unit vocabulary/custom-unit governance, domain-specific significant-digit/range policy, sub-microsecond acquisition, asset encryption/quotas and cloud sync remain future owner decisions. No third-party numeric/asset dependency is added in Phase 0.
