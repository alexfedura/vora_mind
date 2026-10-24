use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

static LINEAGE_COUNTER: AtomicU64 = AtomicU64::new(1);

// =========================================================================
// 1. Core Cryptographic Hash & Version Identifiers (P0/P1)
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub String);

impl WorkspaceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl rusqlite::ToSql for WorkspaceId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.as_str().into())
    }
}

impl rusqlite::types::FromSql for WorkspaceId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        String::column_result(value).map(WorkspaceId)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub String);

impl ContentHash {
    pub fn new(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn compute(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes).to_hex().to_string())
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(content: &str) -> Self {
        Self::compute(content.as_bytes())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl rusqlite::ToSql for ContentHash {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.as_str().into())
    }
}

impl rusqlite::types::FromSql for ContentHash {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        String::column_result(value).map(ContentHash)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub String);

impl SnapshotId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    pub fn generate_root() -> Self {
        Self::generate()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl rusqlite::ToSql for SnapshotId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.as_str().into())
    }
}

impl rusqlite::types::FromSql for SnapshotId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        String::column_result(value).map(SnapshotId)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(pub String);

impl FileId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    pub fn generate_persistent() -> Self {
        Self::generate()
    }

    #[deprecated(
        note = "Path-derived FileIds violate entity identity invariants; use FileId::generate() or FileIdentityService instead"
    )]
    pub fn from_initial_path(initial_path: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        "vora:file_id:v1:".hash(&mut hasher);
        initial_path.hash(&mut hasher);
        Self(format!("file_{:016x}", hasher.finish()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl rusqlite::ToSql for FileId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.as_str().into())
    }
}

impl rusqlite::types::FromSql for FileId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        String::column_result(value).map(FileId)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileVersionId(pub String);

impl FileVersionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    pub fn compute_version(_file_id: &FileId, _content_hash: &ContentHash) -> Self {
        Self::generate()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl rusqlite::ToSql for FileVersionId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.as_str().into())
    }
}

impl rusqlite::types::FromSql for FileVersionId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        String::column_result(value).map(FileVersionId)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub file_id: FileId,
    pub current_path: String,
    pub content_hash: ContentHash,
    pub current_version_id: FileVersionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub file_id: FileId,
    pub path: String,
    pub content_hash: ContentHash,
    pub lineage_parent_id: Option<FileId>,
    pub version_origin: String,
}

// =========================================================================
// 2. Namespace-Aware Symbol Identity & Independent Lineage (Priority 2)
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub String);

impl SymbolId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn compute_canonical(
        language: &str,
        module_namespace: &str,
        semantic_scope: &str,
        kind: SymbolKind,
        canonical_signature: &str,
    ) -> Self {
        let raw = format!(
            "vora:symbol:v3:{}:{}:{}:{}:{}",
            language,
            module_namespace,
            semantic_scope,
            kind.as_str(),
            canonical_signature
        );
        let hash_hex = blake3::hash(raw.as_bytes()).to_hex().to_string();
        Self(format!("sym_{}", &hash_hex[..16]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl rusqlite::ToSql for SymbolId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.as_str().into())
    }
}

impl rusqlite::types::FromSql for SymbolId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        String::column_result(value).map(SymbolId)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineageId(pub String);

impl LineageId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate_persistent() -> Self {
        let seq = LINEAGE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let raw = format!("{}:{}", Uuid::now_v7(), seq);
        let hash_hex = blake3::hash(raw.as_bytes()).to_hex().to_string();
        Self(format!("lin_{}", &hash_hex[..16]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl rusqlite::ToSql for LineageId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.as_str().into())
    }
}

impl rusqlite::types::FromSql for LineageId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        String::column_result(value).map(LineageId)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageRecord {
    pub lineage_id: LineageId,
    pub workspace_id: WorkspaceId,
    pub created_snapshot_id: SnapshotId,
    pub retired_snapshot_id: Option<SnapshotId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageVersion {
    pub lineage_id: LineageId,
    pub snapshot_id: SnapshotId,
    pub symbol_id: SymbolId,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageEvidence {
    pub same_file_id: bool,
    pub same_module_namespace: bool,
    pub same_scope: bool,
    pub same_kind: bool,
    pub name_similarity: f64,
    pub signature_similarity: f64,
    pub body_similarity: f64,
    pub ast_structure_similarity: f64,
    pub relative_position_similarity: f64,
    pub neighboring_symbol_similarity: f64,
    pub reference_topology_similarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageCandidate {
    pub lineage_id: LineageId,
    pub symbol_id: SymbolId,
    pub score: f64,
    pub evidence: LineageEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineageResolution {
    Matched {
        lineage_id: LineageId,
        old_symbol_id: SymbolId,
        new_symbol_id: SymbolId,
        confidence: f64,
        evidence: LineageEvidence,
    },
    New {
        lineage_id: LineageId,
        new_symbol_id: SymbolId,
    },
    Ambiguous {
        new_symbol_id: SymbolId,
        candidates: Vec<LineageCandidate>,
    },
    Removed {
        lineage_id: LineageId,
        old_symbol_id: SymbolId,
    },
}

#[derive(Debug, Clone)]
pub struct LineageWeights {
    pub file: f64,
    pub module: f64,
    pub scope: f64,
    pub kind: f64,
    pub name: f64,
    pub signature: f64,
    pub body: f64,
    pub ast: f64,
    pub position: f64,
    pub neighborhood: f64,
    pub topology: f64,
}

impl Default for LineageWeights {
    fn default() -> Self {
        Self {
            file: 0.15,
            module: 0.15,
            scope: 0.10,
            kind: 0.15,
            name: 0.15,
            signature: 0.10,
            body: 0.10,
            ast: 0.05,
            position: 0.02,
            neighborhood: 0.01,
            topology: 0.02,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Interface,
    Trait,
    Enum,
    TypeAlias,
    Constant,
    Module,
    Variable,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Struct => "struct",
            SymbolKind::Interface => "interface",
            SymbolKind::Trait => "trait",
            SymbolKind::Enum => "enum",
            SymbolKind::TypeAlias => "type_alias",
            SymbolKind::Constant => "constant",
            SymbolKind::Module => "module",
            SymbolKind::Variable => "variable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisibilityKind {
    Public,
    Crate,
    Super,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolIdentity {
    pub id: SymbolId,
    pub file_id: FileId,
    pub module_namespace: String,
    pub name: String,
    pub qualified_scope: String,
    pub kind: SymbolKind,
    pub canonical_signature: String,
    pub body_hash: String,
    pub visibility: VisibilityKind,
    pub lineage_id: Option<LineageId>,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

// =========================================================================
// 3. Hierarchical Lexical Scopes (P0/P2)
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScopeId(pub String);

impl ScopeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn compute_scoped(
        file_id: &FileId,
        module_path: &str,
        lexical_depth: usize,
        scope_discriminator: &str,
    ) -> Self {
        let raw = format!(
            "vora:scope:v3:{}:{}:{}:{}",
            file_id.0, module_path, lexical_depth, scope_discriminator
        );
        let hash_hex = blake3::hash(raw.as_bytes()).to_hex().to_string();
        Self(format!("scope_{}", &hash_hex[..16]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl rusqlite::ToSql for ScopeId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.as_str().into())
    }
}

impl rusqlite::types::FromSql for ScopeId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        String::column_result(value).map(ScopeId)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportKind {
    Local,
    Imported {
        source_module: String,
        original_name: String,
    },
    ReExported {
        source_module: String,
    },
    Glob {
        source_module: String,
    },
    Alias {
        source_module: String,
        alias: String,
    },
    AssociatedType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeContext {
    pub scope_id: ScopeId,
    pub file_id: FileId,
    pub parent_scope_id: Option<ScopeId>,
    pub visible_bindings: HashMap<String, ImportKind>,
}

// =========================================================================
// 4. Raw References & Explicit Resolution Contracts (P0/P6)
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceKind {
    Call,
    TypeUsage,
    ImplTrait,
    Instantiation,
    MacroInvocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawReference {
    pub file_id: FileId,
    pub scope_id: ScopeId,
    pub caller_symbol_id: Option<SymbolId>,
    pub callee_raw_name: String,
    pub receiver_name: Option<String>,
    pub is_member_access: bool,
    pub access_chain: Vec<String>,
    pub ref_kind: ReferenceKind,
    pub byte_range: (usize, usize),
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStatus {
    Resolved(SymbolId),
    Ambiguous(Vec<SymbolId>),
    Unresolved,
    DynamicFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeRelationKind {
    Calls,
    ReferencesType,
    Implements,
    Extends,
    UsesField,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedSemanticEdge {
    pub source_symbol: SymbolId,
    pub target_symbol: Option<SymbolId>,
    pub candidate_symbols: Vec<SymbolId>,
    pub relation: EdgeRelationKind,
    pub status: ResolutionStatus,
    pub call_line: usize,
}

// =========================================================================
// 5. Explicit Semantic Mutation Taxonomy with Old/New Identity Binding (P0/P7)
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SemanticChange {
    BodyChanged {
        symbol_id: SymbolId,
        lineage_id: LineageId,
        old_body_hash: String,
        new_body_hash: String,
    },
    SignatureChanged {
        old_symbol_id: SymbolId,
        new_symbol_id: SymbolId,
        lineage_id: LineageId,
        old_signature: String,
        new_signature: String,
    },
    TypeDefinitionChanged {
        old_symbol_id: SymbolId,
        new_symbol_id: SymbolId,
        lineage_id: LineageId,
    },
    SymbolRenamed {
        old_symbol_id: SymbolId,
        new_symbol_id: SymbolId,
        lineage_id: LineageId,
        old_name: String,
        new_name: String,
    },
    SymbolAdded {
        symbol_id: SymbolId,
        lineage_id: LineageId,
        signature: String,
    },
    SymbolRemoved {
        symbol_id: SymbolId,
        lineage_id: LineageId,
    },
    VisibilityChanged {
        symbol_id: SymbolId,
        lineage_id: LineageId,
        old_vis: VisibilityKind,
        new_vis: VisibilityKind,
    },
}

impl SemanticChange {
    pub fn target_symbol_id(&self) -> &SymbolId {
        match self {
            SemanticChange::BodyChanged { symbol_id, .. }
            | SemanticChange::SymbolAdded { symbol_id, .. }
            | SemanticChange::SymbolRemoved { symbol_id, .. }
            | SemanticChange::VisibilityChanged { symbol_id, .. } => symbol_id,
            SemanticChange::SignatureChanged { new_symbol_id, .. }
            | SemanticChange::TypeDefinitionChanged { new_symbol_id, .. }
            | SemanticChange::SymbolRenamed { new_symbol_id, .. } => new_symbol_id,
        }
    }
}

// =========================================================================
// 6. Optimistic Concurrency Preconditions & Atomic State (P0/P8)
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OverlayId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedCodeState {
    pub overlay_id: OverlayId,
    pub base_commit: String,
    pub transaction_id: String,
    pub expected_content_hash: ContentHash,
    pub dirty_files: HashMap<FileId, String>,
}

// =========================================================================
// 7. Canonical Semantic Entity Definitions & Semantic IR (Priority 3)
// =========================================================================

/// شناسه پایدار کانونیکال بر پایه ترکیب Workspace و Lineage با هش ۲۵۶ بیتی کامل
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanonicalEntityId(pub String);

impl CanonicalEntityId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// تنها سازنده استاندارد و معتبر شناسه کانونیکال با طول کامل BLAKE3-256
    pub fn from_lineage(workspace_id: &WorkspaceId, lineage_id: &LineageId) -> Self {
        let raw = format!(
            "vora:canonical:v2:{}:{}",
            workspace_id.as_str(),
            lineage_id.as_str()
        );
        let hash_hex = blake3::hash(raw.as_bytes()).to_hex().to_string();
        Self(format!("ent_{}", hash_hex))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl rusqlite::ToSql for CanonicalEntityId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.as_str().into())
    }
}

impl rusqlite::types::FromSql for CanonicalEntityId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        String::column_result(value).map(CanonicalEntityId)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CanonicalKind {
    Function,
    Method,
    Type,
    Struct,
    Class,
    Trait,
    Interface,
    Enum,
    EnumVariant,
    Field,
    Constant,
    Variable,
    Module,
    TypeAlias,
    Parameter,
    GenericParameter,
    Impl,
    Namespace,
}

impl CanonicalKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            CanonicalKind::Function => "function",
            CanonicalKind::Method => "method",
            CanonicalKind::Type => "type",
            CanonicalKind::Struct => "struct",
            CanonicalKind::Class => "class",
            CanonicalKind::Trait => "trait",
            CanonicalKind::Interface => "interface",
            CanonicalKind::Enum => "enum",
            CanonicalKind::EnumVariant => "enum_variant",
            CanonicalKind::Field => "field",
            CanonicalKind::Constant => "constant",
            CanonicalKind::Variable => "variable",
            CanonicalKind::Module => "module",
            CanonicalKind::TypeAlias => "type_alias",
            CanonicalKind::Parameter => "parameter",
            CanonicalKind::GenericParameter => "generic_parameter",
            CanonicalKind::Impl => "impl",
            CanonicalKind::Namespace => "namespace",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanonicalNamespace(pub String);

impl CanonicalNamespace {
    pub fn from_rust(crate_name: &str, module_path: &str) -> Self {
        let clean_path = module_path.trim_matches(':').replace("::", "/");
        if clean_path.is_empty() {
            Self(format!("crate://{}", crate_name))
        } else {
            Self(format!("crate://{}/{}", crate_name, clean_path))
        }
    }

    pub fn from_typescript(package_name: &str, module_path: &str) -> Self {
        let clean_path = module_path.trim_matches('/').replace('\\', "/");
        if clean_path.is_empty() {
            Self(format!("ts://{}", package_name))
        } else {
            Self(format!("ts://{}/{}", package_name, clean_path))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalParameter {
    pub name: String,
    pub param_type: String,
    pub is_optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalSignature {
    pub raw_signature: String,
    pub normalized_text: String,
    pub parameters: Vec<CanonicalParameter>,
    pub return_type: Option<String>,
    pub generic_parameters: Vec<String>,
    pub is_async: bool,
}

impl CanonicalSignature {
    pub fn simple(sig: impl Into<String>) -> Self {
        let raw = sig.into();
        Self {
            raw_signature: raw.clone(),
            normalized_text: raw.split_whitespace().collect::<Vec<_>>().join(" "),
            parameters: Vec::new(),
            return_type: None,
            generic_parameters: Vec::new(),
            is_async: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalSymbol {
    pub entity_id: CanonicalEntityId,
    pub lineage_id: LineageId,
    pub workspace_id: WorkspaceId,
    pub snapshot_id: SnapshotId,
    pub language: String,
    pub canonical_namespace: CanonicalNamespace,
    pub canonical_name: String,
    pub canonical_kind: CanonicalKind,
    pub canonical_signature: CanonicalSignature,
    pub visibility: VisibilityKind,
    pub file_id: FileId,
    pub symbol_id: SymbolId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalSymbolRecord {
    pub entity_id: CanonicalEntityId,
    pub symbol_id: SymbolId,
    pub lineage_id: LineageId,
    pub file_id: FileId,
    pub module_namespace: String,
    pub name: String,
    pub kind: SymbolKind,
    pub canonical_signature: String,
    pub scope_path: String,
    pub qualified_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalEntityRecord {
    pub entity_id: CanonicalEntityId,
    pub workspace_id: WorkspaceId,
    pub lineage_id: LineageId,
    pub initial_name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalEntityVersion {
    pub entity_id: CanonicalEntityId,
    pub snapshot_id: SnapshotId,
    pub symbol_id: SymbolId,
    pub lineage_id: LineageId,
    pub canonical_signature: String,
    pub canonical_namespace: String,
    pub canonical_name: String,
    pub created_at: i64,
}
