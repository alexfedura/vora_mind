# Phase 0 — Rust and TypeScript Contract Plan

**Status:** Wire-contract specification; no source types or generated code added in Phase 0.  
**Domain rules:** [Canonical domain contracts](./09-phase-0-domain-contracts.md).  
**Relationship/lifecycle semantics:** [Relationship contracts](./11-phase-0-relationship-and-lifecycle-contracts.md).

## 1. Contract ownership and organization

- **Rust owning-domain structs and operations** are authoritative for validation, persistence, lifecycle transitions, and error behavior. Rust DTOs are explicit transport contracts, not raw database rows.
- Put canonical public scientific interfaces, discriminated unions, branded IDs and string enums in the existing approved `src/types/index.ts` contract location. Feature code imports these types; do not shadow them in `src/mind/types`, component files, stores, or service DTO modules.
- Keep Rust domain models in owning modules (`source`, `learning`, `research`, `experiment`, `model`, `lineage`, `persistence`). IPC DTOs are defined once in the Rust contract layer and re-exported by command modules; avoid a second Rust struct per command with identical meaning.
- Existing project types are split across files and this Phase 0 decision does not refactor them. New scientific contracts follow one canonical frontend surface and domain-owned Rust contracts.
- Do not add a code-generation dependency in this phase. Before a DTO is exposed, use a shared JSON fixture plus Rust Serde golden tests and TypeScript type/runtime validation to prevent drift.
- Domain services own business errors. Tauri handlers only parse/dispatch/map errors. Frontend validation is for UX, never the source of truth.

## 2. Wire representation rules

| Contract element | Rust | JSON/Tauri | TypeScript |
|---|---|---|---|
| Field casing | Rust `snake_case` fields | `camelCase` via `#[serde(rename_all = "camelCase")]` | `camelCase` |
| Enum values | Explicit enum variants with `#[serde(rename_all = "snake_case")]` or explicit per-variant rename | Exact documented `snake_case` strings | String-literal union with the same values |
| Optional known-absent field | `Option<T>` | Present as JSON `null`; do not add `skip_serializing_if` to stable DTOs | `T \| null`, property required |
| Create/patch input | Separate request struct; patch uses `Option<Option<T>>` only when “leave unchanged” vs “clear” is needed, or an explicit field operation enum | Omission semantics documented per request | Distinct create/patch input type; no reuse of response DTO |
| IDs | UUIDv7-backed domain newtypes | Canonical lowercase UUID string for new IDs; legacy IDs remain representable only through a dedicated legacy-reference adapter | Branded string aliases, e.g. `SourceId`, `SourceVersionId` |
| Version identity | Separate typed ID from parent stable ID | Separate `versionId`; never a mutable number as identity | Separate branded `VersionId` family alias |
| System instant | Checked `i64` microseconds since Unix epoch UTC | JSON integer; converters reject unsafe/out-of-range values | Branded safe integer number `UtcTimestampUs` |
| Exact measurement | Validated decimal string plus unit/missingness | JSON string, not floating-point number | Branded `DecimalString` and `UnitCode` |
| JSON payload | Typed versioned Serde struct for each supported schema | `{schemaId, schemaVersion, data}`; unknown versions preserved/reported | Discriminated versioned union; no `any` or unchecked casts |
| Relation endpoint | Typed `ScientificEntityRefDto` with a type-specific ID/version | `{entityType, entityId, versionId}` | Discriminated union narrows ID/version family for `entityType` |

`Option<T>` and TypeScript `null` agree for response DTOs. Missing is not interchangeable with `null`, empty text, zero, or an unknown value. Unknowns use documented enums or explicit unresolved-reference DTOs.

## 3. Shared scalar and enum contracts

Illustrative TypeScript declarations belong in the canonical type module; equivalent Rust newtypes/enums belong in the common Rust contract layer and domain modules:

```ts
type UtcTimestampUs = number & { readonly __brand: "UtcTimestampUs" };
type DecimalString = string & { readonly __brand: "DecimalString" };
type Sha256Hex = string & { readonly __brand: "Sha256Hex" };

type OwnershipState = "project" | "workspace" | "unassigned" | "unknown";
type ValueState =
  | "observed"
  | "not_measured"
  | "not_recorded"
  | "unknown"
  | "not_applicable"
  | "withheld"
  | "invalid";
type RelationPredicate =
  | "cites"
  | "derived_from"
  | "defines"
  | "contains"
  | "in_domain"
  | "addresses"
  | "investigates"
  | "measures"
  | "observed_in"
  | "tests"
  | "analyzed_by"
  | "produces"
  | "supports"
  | "contradicts"
  | "validates"
  | "refines"
  | "extends"
  | "revises"
  | "supersedes"
  | "replicated_by"
  | "based_on"
  | "originated_from"
  | "adopts"
  | "implements"
  | "assesses"
  | "retracts";
```

These are nominally branded on the TypeScript side and validated at the Rust boundary. Brand casts are created only by parsing/validation helpers, never from untrusted JSON. Rust field widths, decimal syntax/ranges, SHA-256 length/hex, enum membership, UUID syntax, and timestamp safe-range are runtime-validated.

There is **no** universal `Status` enum. Each entity has its own lifecycle enum. The same words can have different meanings in `HypothesisAssessmentOutcome`, `ValidationOutcome`, `ReplicationOutcome`, and `CompetencyOutcome`; those enums are not interchangeable.

## 4. DTO shape examples

These shapes define casing/nullability and identity semantics. The full entity field sets follow the entity catalog; additional fields require an explicit contract change and test.

### 4.1 Versioned source

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceVersionDto {
    pub version_id: SourceVersionId,
    pub source_id: SourceId,
    pub capture_kind: SourceCaptureKind,
    pub edition_label: Option<String>,
    pub captured_at_us: UtcTimestampUs,
    pub content_sha256: Option<Sha256Hex>,
    pub asset_id: Option<WorkspaceAssetId>,
    pub created_at_us: UtcTimestampUs,
}
```

```ts
interface SourceVersionDto {
  versionId: SourceVersionId;
  sourceId: SourceId;
  captureKind: SourceCaptureKind;
  editionLabel: string | null;
  capturedAtUs: UtcTimestampUs;
  contentSha256: Sha256Hex | null;
  assetId: WorkspaceAssetId | null;
  createdAtUs: UtcTimestampUs;
}
```

The JSON fixture must contain every nullable property with `null` when absent. `SourceVersionId` never substitutes for `SourceId`; a SourceVersion is immutable and points to one Source.

### 4.2 Typed version-pinned relation

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "entityType", rename_all = "snake_case")]
pub enum ScientificEntityRefDto {
    #[serde(rename_all = "camelCase")]
    Source {
        entity_id: SourceId,
        version_id: Option<SourceVersionId>,
    },
    #[serde(rename_all = "camelCase")]
    SourceClaim {
        entity_id: SourceClaimId,
    },
    #[serde(rename_all = "camelCase")]
    ResearchProject {
        entity_id: ResearchProjectId,
    },
    #[serde(rename_all = "camelCase")]
    ResearchFinding {
        entity_id: ResearchFindingId,
    },
    #[serde(rename_all = "camelCase")]
    Model {
        entity_id: ModelId,
        version_id: Option<ModelVersionId>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScientificRelationDto {
    pub relation_id: ScientificRelationId,
    pub predicate: RelationPredicate,
    pub subject: ScientificEntityRefDto,
    pub object: ScientificEntityRefDto,
    pub context_project_id: Option<ResearchProjectId>,
    pub scope_key: String,
    pub created_at_us: UtcTimestampUs,
}
```

```ts
type ScientificEntityRefDto =
  | { entityType: "source"; entityId: SourceId; versionId: SourceVersionId | null }
  | { entityType: "source_claim"; entityId: SourceClaimId }
  | { entityType: "research_project"; entityId: ResearchProjectId }
  | { entityType: "research_finding"; entityId: ResearchFindingId }
  | { entityType: "model"; entityId: ModelId; versionId: ModelVersionId | null };

interface ScientificRelationDto {
  relationId: ScientificRelationId;
  predicate: RelationPredicate;
  subject: ScientificEntityRefDto;
  object: ScientificEntityRefDto;
  contextProjectId: ResearchProjectId | null;
  scopeKey: string;
  createdAtUs: UtcTimestampUs;
}
```

This is a representative subset, not an exhaustive generated union. The final union is built from the same registry/type-pair allowlist as the Rust relation validator. It must not be widened to `{entityType: string, entityId: string}`.

### 4.3 Measurement, experiment run, and state separation

```ts
interface QuantityDto {
  value: DecimalString | null;
  unitCode: string | null;
  valueState: ValueState;
  uncertainty: DecimalString | null;
}

type ExperimentRunStatus =
  | "scheduled"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "inconclusive";

type HypothesisAssessmentOutcome =
  | "supported"
  | "partially_supported"
  | "rejected"
  | "inconclusive"
  | "unresolved";

interface ExperimentRunDto {
  experimentRunId: ExperimentRunId;
  designVersionId: ExperimentDesignVersionId;
  status: ExperimentRunStatus;
  startedAtUs: UtcTimestampUs | null;
  completedAtUs: UtcTimestampUs | null;
  datasetVersionIds: readonly DatasetVersionId[];
  deviationSchemaId: string;
  deviationSchemaVersion: number;
}
```

A run status never contains a hypothesis outcome. A `HypothesisAssessment` references one HypothesisVersion and its supporting runs/evidence; a `Validation` and `CompetencyRecord` use their own outcome enums.

## 5. Lifecycle and API constraints

- Response DTOs expose status using the owning entity enum. State-changing commands accept transition intent (e.g. `LockExperimentDesignVersion`) rather than arbitrary new status strings.
- Rust checks the transition graph, required fields, actor, exact version, ownership and relation rule in one transaction. Invalid/duplicate/stale transitions return a domain error.
- Version DTOs include stable parent ID, version ID, sequence, parent-version IDs, immutable snapshot checksum and creation metadata. They do not accept client-supplied “latest” as a version reference.
- Signed decimal strings are validated without lossy `f64` conversion. `null` quantity with `valueState: "unknown"` is a different state from not-measured or not-applicable.
- Long-running run state is returned by stable run ID; logs/results are separate paged DTOs if necessary and do not inflate one IPC response.
- Error responses distinguish validation, missing record/version, invalid transition, conflict/stale revision, FK/integrity failure, asset/checksum failure, migration failure and storage failure. Errors do not return a fabricated default entity.

## 6. Contract test plan

For each persisted DTO and enum:

1. Rust serializes to the checked-in JSON fixture; TypeScript fixture validator accepts the exact shape.
2. Rust deserializes the fixture and round-trips it without changing field names, enum strings, nulls, timestamps, IDs or decimal strings.
3. Missing required property, wrong enum case/value, invalid UUID, unsafe timestamp, malformed decimal/hash, unknown JSON schema version and mismatched relation endpoint type fail explicitly.
4. `Option<T>` maps to explicit JSON `null`; patch omission semantics have separate tests.
5. A compile-time TS exhaustive switch over each lifecycle/relation enum forces review when a variant is added.
6. Relation fixtures cover every allowed predicate/type tuple and each deliberately rejected tuple/version pin.
7. Test errors show backend validation and do not update Zustand as if the operation succeeded.

No fixtures, interfaces, runtime validation package, generated code, or tests are added during this Phase 0 contract-only change.
