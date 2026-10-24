use super::ast_engine::{AstEngine, ParsedFilePayload, RawParsedFile};
use super::model::{
    CanonicalEntityId, CanonicalEntityVersion, CanonicalSymbolRecord, ContentHash, FileId,
    FileVersionId, LineageId, SnapshotId, SymbolId, SymbolKind, WorkspaceId,
};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const IDENTITY_SPINE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workspaces (
    workspace_id TEXT PRIMARY KEY,
    root_path TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    current_snapshot_id TEXT
);

CREATE TABLE IF NOT EXISTS snapshots (
    snapshot_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    parent_snapshot_id TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id),
    FOREIGN KEY(parent_snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS files (
    file_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    deleted_at INTEGER,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id)
);

CREATE TABLE IF NOT EXISTS file_versions (
    file_version_id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    byte_size INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    parent_file_version_id TEXT,
    FOREIGN KEY(file_id) REFERENCES files(file_id),
    FOREIGN KEY(parent_file_version_id) REFERENCES file_versions(file_version_id)
);

CREATE TABLE IF NOT EXISTS snapshot_files (
    snapshot_id TEXT NOT NULL,
    file_id TEXT NOT NULL,
    path_at_snapshot TEXT NOT NULL,
    file_version_id TEXT NOT NULL,
    PRIMARY KEY(snapshot_id, file_id),
    UNIQUE(snapshot_id, path_at_snapshot),
    FOREIGN KEY(snapshot_id) REFERENCES snapshots(snapshot_id),
    FOREIGN KEY(file_id) REFERENCES files(file_id),
    FOREIGN KEY(file_version_id) REFERENCES file_versions(file_version_id)
);

CREATE TABLE IF NOT EXISTS symbol_lineages (
    lineage_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    created_snapshot_id TEXT NOT NULL,
    retired_snapshot_id TEXT,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id),
    FOREIGN KEY(created_snapshot_id) REFERENCES snapshots(snapshot_id),
    FOREIGN KEY(retired_snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS symbol_lineage_versions (
    lineage_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    symbol_id TEXT NOT NULL,
    confidence REAL,
    PRIMARY KEY(lineage_id, snapshot_id),
    FOREIGN KEY(lineage_id) REFERENCES symbol_lineages(lineage_id),
    FOREIGN KEY(snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS canonical_entities (
    entity_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    lineage_id TEXT NOT NULL,
    initial_name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id),
    FOREIGN KEY(lineage_id) REFERENCES symbol_lineages(lineage_id)
);

CREATE TABLE IF NOT EXISTS canonical_entity_versions (
    entity_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    symbol_id TEXT NOT NULL,
    lineage_id TEXT NOT NULL,
    canonical_signature TEXT NOT NULL,
    canonical_namespace TEXT NOT NULL,
    canonical_name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY(entity_id, snapshot_id),
    FOREIGN KEY(entity_id) REFERENCES canonical_entities(entity_id),
    FOREIGN KEY(snapshot_id) REFERENCES snapshots(snapshot_id),
    FOREIGN KEY(lineage_id) REFERENCES symbol_lineages(lineage_id)
);

CREATE TABLE IF NOT EXISTS forge_audit_trail (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    intent TEXT NOT NULL,
    target_symbol TEXT NOT NULL,
    target_file TEXT NOT NULL,
    base_snapshot_id TEXT,
    committed_snapshot_id TEXT,
    healing_attempts INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL,
    error_payload TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id)
);

CREATE TABLE IF NOT EXISTS code_files (
    file_id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL UNIQUE,
    content_hash TEXT NOT NULL,
    lineage_parent_id TEXT,
    last_indexed_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS code_symbols (
    id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    module_namespace TEXT NOT NULL,
    file_path TEXT NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    scope_path TEXT NOT NULL,
    qualified_scope TEXT NOT NULL,
    signature TEXT NOT NULL,
    canonical_signature TEXT NOT NULL,
    body_hash TEXT NOT NULL,
    visibility TEXT NOT NULL,
    lineage_id TEXT,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    start_byte INTEGER NOT NULL,
    end_byte INTEGER NOT NULL,
    FOREIGN KEY(file_id) REFERENCES code_files(file_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS code_scopes (
    scope_id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    parent_scope_id TEXT,
    visible_bindings_json TEXT NOT NULL,
    FOREIGN KEY(file_id) REFERENCES code_files(file_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS code_imports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id TEXT,
    file_path TEXT NOT NULL,
    source_module TEXT NOT NULL,
    imported_name TEXT NOT NULL,
    alias TEXT,
    line INTEGER NOT NULL,
    FOREIGN KEY(file_path) REFERENCES code_files(file_path) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS code_references (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    caller_scope TEXT NOT NULL,
    callee_name TEXT NOT NULL,
    line INTEGER NOT NULL,
    byte_offset INTEGER NOT NULL,
    FOREIGN KEY(file_path) REFERENCES code_files(file_path) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS code_raw_references (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    caller_symbol_id TEXT,
    callee_raw_name TEXT NOT NULL,
    receiver_name TEXT,
    is_member_access INTEGER NOT NULL DEFAULT 0,
    access_chain_json TEXT,
    ref_kind TEXT NOT NULL,
    byte_start INTEGER NOT NULL,
    byte_end INTEGER NOT NULL,
    line INTEGER NOT NULL,
    FOREIGN KEY(file_id) REFERENCES code_files(file_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_files_path ON code_files(file_path);
CREATE INDEX IF NOT EXISTS idx_files_hash ON code_files(content_hash);
CREATE INDEX IF NOT EXISTS idx_symbols_file ON code_symbols(file_id);
CREATE INDEX IF NOT EXISTS idx_symbols_namespace ON code_symbols(module_namespace);
CREATE INDEX IF NOT EXISTS idx_symbols_name ON code_symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_lineage ON code_symbols(lineage_id);
CREATE INDEX IF NOT EXISTS idx_scopes_file ON code_scopes(file_id);
CREATE INDEX IF NOT EXISTS idx_references_callee ON code_references(callee_name);
CREATE INDEX IF NOT EXISTS idx_raw_refs_callee ON code_raw_references(callee_raw_name);
CREATE INDEX IF NOT EXISTS idx_raw_refs_scope ON code_raw_references(scope_id);

CREATE INDEX IF NOT EXISTS idx_spine_snapshots_ws ON snapshots(workspace_id);
CREATE INDEX IF NOT EXISTS idx_spine_files_ws ON files(workspace_id);
CREATE INDEX IF NOT EXISTS idx_spine_versions_file ON file_versions(file_id);
CREATE INDEX IF NOT EXISTS idx_spine_versions_hash ON file_versions(content_hash);
CREATE INDEX IF NOT EXISTS idx_spine_snapfiles_snap ON snapshot_files(snapshot_id);
CREATE INDEX IF NOT EXISTS idx_spine_snapfiles_path ON snapshot_files(path_at_snapshot);
CREATE INDEX IF NOT EXISTS idx_lineage_versions_symbol ON symbol_lineage_versions(symbol_id);
CREATE INDEX IF NOT EXISTS idx_lineage_versions_snapshot ON symbol_lineage_versions(snapshot_id);
CREATE INDEX IF NOT EXISTS idx_canonical_entities_ws ON canonical_entities(workspace_id);
CREATE INDEX IF NOT EXISTS idx_canonical_entities_lin ON canonical_entities(lineage_id);
CREATE INDEX IF NOT EXISTS idx_canonical_versions_ent ON canonical_entity_versions(entity_id);
CREATE INDEX IF NOT EXISTS idx_canonical_versions_snap ON canonical_entity_versions(snapshot_id);
CREATE INDEX IF NOT EXISTS idx_forge_audit_task ON forge_audit_trail(task_id);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileIdentityResolution {
    Existing(FileId),
    New(FileId),
    Ambiguous(Vec<FileId>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrailRecord {
    pub id: i64,
    pub task_id: String,
    pub workspace_id: String,
    pub intent: String,
    pub target_symbol: String,
    pub target_file: String,
    pub base_snapshot_id: Option<String>,
    pub committed_snapshot_id: Option<String>,
    pub healing_attempts: usize,
    pub status: String,
    pub error_payload: Option<String>,
    pub created_at: i64,
}

pub struct CodeIndex {
    conn: Connection,
}

impl CodeIndex {
    pub fn initialize_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        // افزودن ایمن ستون‌های ساختاری در صورت وجود اسکیما از ماژول‌های دیگر
        let _ = conn.execute("ALTER TABLE snapshots ADD COLUMN workspace_id TEXT;", []);
        let _ = conn.execute("ALTER TABLE snapshots ADD COLUMN snapshot_id TEXT;", []);
        let _ = conn.execute("ALTER TABLE snapshots ADD COLUMN id TEXT;", []);
        let _ = conn.execute("ALTER TABLE snapshots ADD COLUMN name TEXT;", []);
        let _ = conn.execute(
            "ALTER TABLE snapshots ADD COLUMN depth INTEGER DEFAULT 0;",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE snapshots ADD COLUMN replay_cost INTEGER DEFAULT 0;",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE snapshots ADD COLUMN is_checkpoint INTEGER DEFAULT 0;",
            [],
        );
        let _ = conn.execute("ALTER TABLE files ADD COLUMN workspace_id TEXT;", []);

        conn.execute_batch(IDENTITY_SPINE_SCHEMA)?;
        Ok(())
    }

    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Self::initialize_schema(&conn)?;
        Ok(Self { conn })
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::initialize_schema(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::new_in_memory()
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn get_or_create_workspace(&self, root_path: &str) -> Result<WorkspaceId> {
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT workspace_id FROM workspaces WHERE root_path = ?1",
                [root_path],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            return Ok(WorkspaceId(id));
        }

        let ws_id = WorkspaceId::generate();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, created_at, current_snapshot_id) VALUES (?1, ?2, ?3, NULL)",
            params![ws_id.as_str(), root_path, now],
        )?;

        Ok(ws_id)
    }

    pub fn compute_hash(content: &str) -> String {
        ContentHash::from_str(content).0
    }

    pub fn resolve_file_identity(
        &self,
        workspace_id: &WorkspaceId,
        file_path: &str,
        content_hash: &str,
    ) -> Result<FileIdentityResolution> {
        let current_snap: Option<String> = self
            .conn
            .query_row(
                "SELECT current_snapshot_id FROM workspaces WHERE workspace_id = ?1",
                [workspace_id.as_str()],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        if let Some(snap_id) = current_snap {
            let direct_match: Option<String> = self
                .conn
                .query_row(
                    "SELECT file_id FROM snapshot_files WHERE snapshot_id = ?1 AND path_at_snapshot = ?2",
                    params![&snap_id, file_path],
                    |row| row.get(0),
                )
                .ok();

            if let Some(fid) = direct_match {
                return Ok(FileIdentityResolution::Existing(FileId(fid)));
            }
        }

        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT f.file_id FROM files f
             JOIN file_versions fv ON f.file_id = fv.file_id
             WHERE f.workspace_id = ?1 AND fv.content_hash = ?2 AND f.deleted_at IS NULL",
        )?;

        let candidates: Vec<String> = stmt
            .query_map(params![workspace_id.as_str(), content_hash], |row| {
                row.get(0)
            })?
            .filter_map(|r| r.ok())
            .collect();

        match candidates.len() {
            0 => Ok(FileIdentityResolution::New(FileId::generate())),
            1 => Ok(FileIdentityResolution::Existing(FileId(
                candidates[0].clone(),
            ))),
            _ => Ok(FileIdentityResolution::Ambiguous(
                candidates.into_iter().map(FileId).collect(),
            )),
        }
    }

    pub fn get_or_create_file_id(&self, file_path: &str) -> FileId {
        let existing_id: Option<String> = self
            .conn
            .query_row(
                "SELECT file_id FROM code_files WHERE file_path = ?1",
                [file_path],
                |row| row.get(0),
            )
            .ok();

        match existing_id {
            Some(id) => FileId::new(id),
            None => FileId::generate(),
        }
    }

    pub fn discover_persistent_file_id(&self, file_path: &str, content_hash: &str) -> FileId {
        if let Ok(id) = self.conn.query_row(
            "SELECT file_id FROM code_files WHERE file_path = ?1",
            [file_path],
            |row| row.get::<_, String>(0),
        ) {
            return FileId::new(id);
        }

        let matching_ids: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT file_id FROM code_files WHERE content_hash = ?1")
                .unwrap();
            stmt.query_map([content_hash], |row| row.get::<_, String>(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };

        if matching_ids.len() == 1 {
            return FileId::new(matching_ids[0].clone());
        }

        FileId::generate()
    }

    pub fn commit_identity_snapshot(
        &mut self,
        workspace_id: &WorkspaceId,
        parent_snapshot_id: Option<&SnapshotId>,
        files_state: &[(&str, &[u8])],
    ) -> Result<SnapshotId> {
        let tx = self.conn.transaction()?;
        let snapshot_id = SnapshotId::generate();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        tx.execute(
            "INSERT INTO snapshots (snapshot_id, workspace_id, parent_snapshot_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                snapshot_id.as_str(),
                workspace_id.as_str(),
                parent_snapshot_id.map(|p| p.as_str()),
                now
            ],
        )?;

        for (path, bytes) in files_state {
            let hash = ContentHash::compute(bytes);
            let byte_size = bytes.len() as i64;

            let file_id = {
                let existing_file: Option<String> = tx
                    .query_row(
                        "SELECT sf.file_id FROM snapshot_files sf
                         JOIN snapshots s ON sf.snapshot_id = s.snapshot_id
                         WHERE s.workspace_id = ?1 AND sf.path_at_snapshot = ?2
                         ORDER BY s.created_at DESC LIMIT 1",
                        params![workspace_id.as_str(), path],
                        |row| row.get(0),
                    )
                    .ok();

                match existing_file {
                    Some(fid) => FileId(fid),
                    None => {
                        let new_fid = FileId::generate();
                        tx.execute(
                            "INSERT INTO files (file_id, workspace_id, created_at, deleted_at)
                             VALUES (?1, ?2, ?3, NULL)",
                            params![new_fid.as_str(), workspace_id.as_str(), now],
                        )?;
                        new_fid
                    }
                }
            };

            let version_id = {
                let existing_ver: Option<String> = tx
                    .query_row(
                        "SELECT file_version_id FROM file_versions WHERE file_id = ?1 AND content_hash = ?2",
                        params![file_id.as_str(), hash.as_str()],
                        |row| row.get(0),
                    )
                    .ok();

                match existing_ver {
                    Some(vid) => FileVersionId(vid),
                    None => {
                        let new_vid = FileVersionId::generate();
                        tx.execute(
                            "INSERT INTO file_versions (file_version_id, file_id, content_hash, byte_size, created_at, parent_file_version_id)
                             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                            params![new_vid.as_str(), file_id.as_str(), hash.as_str(), byte_size, now],
                        )?;
                        new_vid
                    }
                }
            };

            tx.execute(
                "INSERT INTO snapshot_files (snapshot_id, file_id, path_at_snapshot, file_version_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![snapshot_id.as_str(), file_id.as_str(), path, version_id.as_str()],
            )?;
        }

        tx.execute(
            "UPDATE workspaces SET current_snapshot_id = ?1 WHERE workspace_id = ?2",
            params![snapshot_id.as_str(), workspace_id.as_str()],
        )?;

        tx.commit()?;
        Ok(snapshot_id)
    }

    pub fn record_lineage_version(
        &self,
        workspace_id: &WorkspaceId,
        snapshot_id: &SnapshotId,
        lineage_id: &LineageId,
        symbol_id: &SymbolId,
        confidence: Option<f64>,
    ) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT OR IGNORE INTO symbol_lineages (lineage_id, workspace_id, created_snapshot_id, retired_snapshot_id)
             VALUES (?1, ?2, ?3, NULL)",
            params![lineage_id.as_str(), workspace_id.as_str(), snapshot_id.as_str()],
        )?;

        tx.execute(
            "INSERT OR REPLACE INTO symbol_lineage_versions (lineage_id, snapshot_id, symbol_id, confidence)
             VALUES (?1, ?2, ?3, ?4)",
            params![lineage_id.as_str(), snapshot_id.as_str(), symbol_id.as_str(), confidence],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn get_lineage_history(
        &self,
        lineage_id: &LineageId,
    ) -> Result<Vec<(SnapshotId, SymbolId, Option<f64>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT slv.snapshot_id, slv.symbol_id, slv.confidence
             FROM symbol_lineage_versions slv
             JOIN snapshots s ON slv.snapshot_id = s.snapshot_id
             WHERE slv.lineage_id = ?1
             ORDER BY s.created_at ASC",
        )?;

        let rows = stmt.query_map([lineage_id.as_str()], |row| {
            Ok((
                SnapshotId::new(row.get::<_, String>(0)?),
                SymbolId::new(row.get::<_, String>(1)?),
                row.get::<_, Option<f64>>(2)?,
            ))
        })?;

        let mut history = Vec::new();
        for r in rows.flatten() {
            history.push(r);
        }
        Ok(history)
    }

    /// ثبت اتمیک CanonicalEntity و نسخه تاریخی آن
    #[allow(clippy::too_many_arguments)]
    pub fn register_canonical_entity(
        &self,
        workspace_id: &WorkspaceId,
        snapshot_id: &SnapshotId,
        lineage_id: &LineageId,
        symbol_id: &SymbolId,
        canonical_name: &str,
        canonical_namespace: &str,
        canonical_signature: &str,
    ) -> Result<CanonicalEntityId> {
        let entity_id = CanonicalEntityId::from_lineage(workspace_id, lineage_id);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let tx = self.conn.unchecked_transaction()?;

        // ثبت تبار والد با اسنپ‌شات معتبر
        tx.execute(
            "INSERT OR IGNORE INTO symbol_lineages (lineage_id, workspace_id, created_snapshot_id, retired_snapshot_id)
             VALUES (?1, ?2, ?3, NULL)",
            params![lineage_id.as_str(), workspace_id.as_str(), snapshot_id.as_str()],
        )?;

        // ثبت انتیتی پایه
        tx.execute(
            "INSERT OR IGNORE INTO canonical_entities (entity_id, workspace_id, lineage_id, initial_name, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![entity_id.as_str(), workspace_id.as_str(), lineage_id.as_str(), canonical_name, now],
        )?;

        // ثبت نسخه تاریخی کانونیکال
        tx.execute(
            "INSERT OR REPLACE INTO canonical_entity_versions (
                entity_id, snapshot_id, symbol_id, lineage_id, canonical_signature, canonical_namespace, canonical_name, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entity_id.as_str(),
                snapshot_id.as_str(),
                symbol_id.as_str(),
                lineage_id.as_str(),
                canonical_signature,
                canonical_namespace,
                canonical_name,
                now
            ],
        )?;

        tx.commit()?;
        Ok(entity_id)
    }

    /// بازیابی تاریخچه تکامل یک CanonicalEntity بر اساس DAG اسنپ‌شات‌ها
    pub fn get_canonical_entity_history(
        &self,
        entity_id: &CanonicalEntityId,
    ) -> Result<Vec<CanonicalEntityVersion>> {
        let mut stmt = self.conn.prepare(
            "SELECT cev.entity_id, cev.snapshot_id, cev.symbol_id, cev.lineage_id,
                    cev.canonical_signature, cev.canonical_namespace, cev.canonical_name, cev.created_at
             FROM canonical_entity_versions cev
             JOIN snapshots s ON cev.snapshot_id = s.snapshot_id
             WHERE cev.entity_id = ?1
             ORDER BY s.created_at ASC, cev.created_at ASC",
        )?;

        let rows = stmt.query_map([entity_id.as_str()], |row| {
            Ok(CanonicalEntityVersion {
                entity_id: CanonicalEntityId::new(row.get::<_, String>(0)?),
                snapshot_id: SnapshotId::new(row.get::<_, String>(1)?),
                symbol_id: SymbolId::new(row.get::<_, String>(2)?),
                lineage_id: LineageId::new(row.get::<_, String>(3)?),
                canonical_signature: row.get(4)?,
                canonical_namespace: row.get(5)?,
                canonical_name: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;

        let mut history = Vec::new();
        for r in rows.flatten() {
            history.push(r);
        }
        Ok(history)
    }

    /// ثبت متادیتای ردیابی و گزارش اجرای تسک‌های خودکار در پایگاه داده
    #[allow(clippy::too_many_arguments)]
    pub fn record_audit_entry(
        &self,
        task_id: &str,
        workspace_id: &WorkspaceId,
        intent: &str,
        target_symbol: &str,
        target_file: &str,
        base_snapshot_id: Option<&SnapshotId>,
        committed_snapshot_id: Option<&SnapshotId>,
        healing_attempts: usize,
        status: &str,
        error_payload: Option<&str>,
    ) -> Result<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO forge_audit_trail (
                task_id, workspace_id, intent, target_symbol, target_file,
                base_snapshot_id, committed_snapshot_id, healing_attempts, status, error_payload, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                task_id,
                workspace_id.as_str(),
                intent,
                target_symbol,
                target_file,
                base_snapshot_id.map(|s| s.as_str()),
                committed_snapshot_id.map(|s| s.as_str()),
                healing_attempts as i64,
                status,
                error_payload,
                now
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// واکشی تاریخچه و لاگ‌های ردیابی بر اساس Task ID
    pub fn get_task_audit_history(&self, task_id: &str) -> Result<Vec<AuditTrailRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, workspace_id, intent, target_symbol, target_file,
                    base_snapshot_id, committed_snapshot_id, healing_attempts, status, error_payload, created_at
             FROM forge_audit_trail
             WHERE task_id = ?1
             ORDER BY created_at ASC",
        )?;

        let rows = stmt.query_map([task_id], |row| {
            Ok(AuditTrailRecord {
                id: row.get(0)?,
                task_id: row.get(1)?,
                workspace_id: row.get(2)?,
                intent: row.get(3)?,
                target_symbol: row.get(4)?,
                target_file: row.get(5)?,
                base_snapshot_id: row.get(6)?,
                committed_snapshot_id: row.get(7)?,
                healing_attempts: row.get::<_, i64>(8)? as usize,
                status: row.get(9)?,
                error_payload: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?;

        let mut history = Vec::new();
        for r in rows.flatten() {
            history.push(r);
        }
        Ok(history)
    }

    pub fn index_canonical_entities(&self, records: &[CanonicalSymbolRecord]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for r in records {
            tx.execute(
                "INSERT OR REPLACE INTO code_symbols (
                    id, file_id, module_namespace, file_path, name, kind, scope_path, qualified_scope,
                    signature, canonical_signature, body_hash, visibility, lineage_id,
                    start_line, end_line, start_byte, end_byte
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, '', 'Public', ?11, 1, 1, 0, 0)",
                params![
                    r.symbol_id.as_str(),
                    r.file_id.as_str(),
                    r.module_namespace,
                    r.file_id.as_str(),
                    r.name,
                    r.kind.as_str(),
                    r.scope_path,
                    r.qualified_scope,
                    r.canonical_signature,
                    r.canonical_signature,
                    r.lineage_id.as_str(),
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// واکشی مستقیم و جامع تمام موجودیت‌های کانونیکال ثبت‌شده در سیستم
    pub fn get_canonical_entities(&self) -> Result<Vec<CanonicalSymbolRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT ce.entity_id, 
                    cev.symbol_id, 
                    ce.lineage_id, 
                    COALESCE(cs.file_id, 'file_root'), 
                    cev.canonical_namespace, 
                    cev.canonical_name, 
                    COALESCE(cs.kind, 'function'), 
                    cev.canonical_signature, 
                    COALESCE(cs.scope_path, ''), 
                    COALESCE(cs.qualified_scope, cev.canonical_namespace || '::' || cev.canonical_name)
             FROM canonical_entities ce
             JOIN canonical_entity_versions cev ON ce.entity_id = cev.entity_id
             LEFT JOIN code_symbols cs ON cev.symbol_id = cs.id
             GROUP BY ce.entity_id
             ORDER BY cev.created_at ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let kind_str: String = row.get(6)?;
            let kind = match kind_str.to_lowercase().as_str() {
                "struct" => SymbolKind::Struct,
                "trait" => SymbolKind::Trait,
                "interface" => SymbolKind::Interface,
                "enum" => SymbolKind::Enum,
                "type_alias" => SymbolKind::TypeAlias,
                "method" => SymbolKind::Method,
                _ => SymbolKind::Function,
            };

            Ok(CanonicalSymbolRecord {
                entity_id: CanonicalEntityId::new(row.get::<_, String>(0)?),
                symbol_id: SymbolId::new(row.get::<_, String>(1)?),
                lineage_id: LineageId::new(row.get::<_, String>(2)?),
                file_id: FileId::new(row.get::<_, String>(3)?),
                module_namespace: row.get(4)?,
                name: row.get(5)?,
                kind,
                canonical_signature: row.get(7)?,
                scope_path: row.get(8)?,
                qualified_scope: row.get(9)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows.flatten() {
            list.push(r);
        }
        Ok(list)
    }

    pub fn index_file_semantics(&self, payload: &ParsedFilePayload) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        tx.execute(
            "INSERT OR REPLACE INTO code_files (file_id, file_path, content_hash, lineage_parent_id, last_indexed_at)
             VALUES (?1, ?2, ?3, NULL, ?4)",
            params![&payload.file_id.0, &payload.path, "semantic_stage", now],
        )?;

        for sym in &payload.symbols {
            let lineage_opt = sym.lineage_id.as_ref().map(|l| l.0.as_str());
            tx.execute(
                "INSERT INTO code_symbols (
                    id, file_id, module_namespace, file_path, name, kind, scope_path, qualified_scope,
                    signature, canonical_signature, body_hash, visibility, lineage_id,
                    start_line, end_line, start_byte, end_byte
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    sym.id.0,
                    sym.file_id.0,
                    sym.module_namespace,
                    payload.path,
                    sym.name,
                    format!("{:?}", sym.kind),
                    sym.qualified_scope,
                    sym.qualified_scope,
                    sym.canonical_signature,
                    sym.canonical_signature,
                    sym.body_hash,
                    format!("{:?}", sym.visibility),
                    lineage_opt,
                    sym.start_line as i64,
                    sym.end_line as i64,
                    sym.start_byte as i64,
                    sym.end_byte as i64,
                ],
            )?;
        }

        for scope in &payload.scopes {
            let bindings_json =
                serde_json::to_string(&scope.visible_bindings).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT OR REPLACE INTO code_scopes (scope_id, file_id, parent_scope_id, visible_bindings_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    scope.scope_id.0,
                    scope.file_id.0,
                    scope.parent_scope_id.as_ref().map(|s| &s.0),
                    bindings_json,
                ],
            )?;
        }

        for r in &payload.raw_references {
            let access_chain_json =
                serde_json::to_string(&r.access_chain).unwrap_or_else(|_| "[]".to_string());
            tx.execute(
                "INSERT INTO code_raw_references (
                    file_id, scope_id, caller_symbol_id, callee_raw_name, receiver_name,
                    is_member_access, access_chain_json, ref_kind, byte_start, byte_end, line
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    r.file_id.0,
                    r.scope_id.0,
                    r.caller_symbol_id.as_ref().map(|s| &s.0),
                    r.callee_raw_name,
                    r.receiver_name,
                    if r.is_member_access { 1 } else { 0 },
                    access_chain_json,
                    format!("{:?}", r.ref_kind),
                    r.byte_range.0 as i64,
                    r.byte_range.1 as i64,
                    r.line as i64,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn index_file(&self, file_path: &str, parsed: &RawParsedFile) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let file_id = self.get_or_create_file_id(file_path);

        tx.execute(
            "INSERT OR IGNORE INTO code_files (file_id, file_path, content_hash, lineage_parent_id, last_indexed_at)
             VALUES (?1, ?2, ?3, NULL, ?4)",
            params![&file_id.0, file_path, "syntax_stage", now],
        )?;

        for imp in &parsed.imports {
            tx.execute(
                "INSERT INTO code_imports (file_id, file_path, source_module, imported_name, alias, line)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    &file_id.0,
                    file_path,
                    imp.source_module,
                    imp.imported_name,
                    imp.alias,
                    imp.line as i64,
                ],
            )?;
        }

        for r in &parsed.references {
            tx.execute(
                "INSERT INTO code_references (file_path, caller_scope, callee_name, line, byte_offset)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    file_path,
                    r.caller_scope,
                    r.callee_name,
                    r.line as i64,
                    r.byte_offset as i64,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn index_file_incremental(&mut self, file_path: &str, content: &str) -> Result<bool> {
        let new_hash = Self::compute_hash(content);

        let existing_hash: Option<String> = self
            .conn
            .query_row(
                "SELECT content_hash FROM code_files WHERE file_path = ?1",
                [file_path],
                |row| row.get(0),
            )
            .ok();

        if let Some(hash) = existing_hash {
            if hash == new_hash {
                return Ok(false);
            }
        }

        let file_id = self.discover_persistent_file_id(file_path, &new_hash);

        let parsed_classic = AstEngine::parse_file_syntax(file_path, content);
        let parsed_semantic: ParsedFilePayload =
            AstEngine::parse_file_semantics(file_id.clone(), file_path, content);

        let tx = self.conn.transaction()?;

        tx.execute(
            "DELETE FROM code_raw_references WHERE file_id = ?1",
            [&file_id.0],
        )?;
        tx.execute("DELETE FROM code_scopes WHERE file_id = ?1", [&file_id.0])?;
        tx.execute(
            "DELETE FROM code_references WHERE file_path = ?1",
            [file_path],
        )?;
        tx.execute(
            "DELETE FROM code_imports WHERE file_path = ?1 OR file_id = ?2",
            params![file_path, &file_id.0],
        )?;
        tx.execute(
            "DELETE FROM code_symbols WHERE file_path = ?1 OR file_id = ?2",
            params![file_path, &file_id.0],
        )?;
        tx.execute(
            "DELETE FROM code_files WHERE file_path = ?1 OR file_id = ?2",
            params![file_path, &file_id.0],
        )?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        tx.execute(
            "INSERT INTO code_files (file_id, file_path, content_hash, lineage_parent_id, last_indexed_at)
             VALUES (?1, ?2, ?3, NULL, ?4)",
            params![&file_id.0, file_path, new_hash, now],
        )?;

        for sym in &parsed_semantic.symbols {
            let lineage_opt = sym.lineage_id.as_ref().map(|l| l.0.as_str());
            tx.execute(
                "INSERT INTO code_symbols (
                    id, file_id, module_namespace, file_path, name, kind, scope_path, qualified_scope,
                    signature, canonical_signature, body_hash, visibility, lineage_id,
                    start_line, end_line, start_byte, end_byte
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    sym.id.0,
                    sym.file_id.0,
                    sym.module_namespace,
                    file_path,
                    sym.name,
                    format!("{:?}", sym.kind),
                    sym.qualified_scope,
                    sym.qualified_scope,
                    sym.canonical_signature,
                    sym.canonical_signature,
                    sym.body_hash,
                    format!("{:?}", sym.visibility),
                    lineage_opt,
                    sym.start_line as i64,
                    sym.end_line as i64,
                    sym.start_byte as i64,
                    sym.end_byte as i64,
                ],
            )?;
        }

        for scope in &parsed_semantic.scopes {
            let bindings_json =
                serde_json::to_string(&scope.visible_bindings).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO code_scopes (scope_id, file_id, parent_scope_id, visible_bindings_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    scope.scope_id.0,
                    scope.file_id.0,
                    scope.parent_scope_id.as_ref().map(|s| &s.0),
                    bindings_json,
                ],
            )?;
        }

        for imp in &parsed_classic.imports {
            tx.execute(
                "INSERT INTO code_imports (file_id, file_path, source_module, imported_name, alias, line)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    &file_id.0,
                    file_path,
                    imp.source_module,
                    imp.imported_name,
                    imp.alias,
                    imp.line as i64,
                ],
            )?;
        }

        for r in &parsed_classic.references {
            tx.execute(
                "INSERT INTO code_references (file_path, caller_scope, callee_name, line, byte_offset)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    file_path,
                    r.caller_scope,
                    r.callee_name,
                    r.line as i64,
                    r.byte_offset as i64,
                ],
            )?;
        }

        for r in &parsed_semantic.raw_references {
            let access_chain_json =
                serde_json::to_string(&r.access_chain).unwrap_or_else(|_| "[]".to_string());
            tx.execute(
                "INSERT INTO code_raw_references (
                    file_id, scope_id, caller_symbol_id, callee_raw_name, receiver_name,
                    is_member_access, access_chain_json, ref_kind, byte_start, byte_end, line
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    r.file_id.0,
                    r.scope_id.0,
                    r.caller_symbol_id.as_ref().map(|s| &s.0),
                    r.callee_raw_name,
                    r.receiver_name,
                    if r.is_member_access { 1 } else { 0 },
                    access_chain_json,
                    format!("{:?}", r.ref_kind),
                    r.byte_range.0 as i64,
                    r.byte_range.1 as i64,
                    r.line as i64,
                ],
            )?;
        }

        tx.commit()?;
        Ok(true)
    }

    pub fn get_total_symbols_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM code_symbols", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}
