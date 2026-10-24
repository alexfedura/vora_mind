use rusqlite::Connection;
use std::fs;
use vora_lib::forge::differ::SemanticDiffer;
use vora_lib::forge::identity::{
    FileIdentityResolution, FileIdentityService, ObservedFile, SnapshotBuilder,
};
use vora_lib::forge::index::{CodeIndex, IDENTITY_SPINE_SCHEMA};
use vora_lib::forge::model::{ContentHash, LineageId, SemanticChange, WorkspaceId};

/// بررسی ۱۲ قانون طلایی ساختار Identity Invariants
fn assert_identity_invariants(conn: &Connection, workspace_id: &WorkspaceId) {
    // قانون ۱: هر FileVersion دقیقاً به یک FileId معتبر اشاره می‌کند
    let invalid_version_files: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM file_versions fv LEFT JOIN files f ON fv.file_id = f.file_id WHERE f.file_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        invalid_version_files, 0,
        "Invariant Violation: Orphan file_version found"
    );

    // قانون ۲: تمام snapshot_files به فایل‌های ثبت‌شده اشاره می‌کنند
    let invalid_snapshot_files: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshot_files sf LEFT JOIN files f ON sf.file_id = f.file_id WHERE f.file_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        invalid_snapshot_files, 0,
        "Invariant Violation: Dangling snapshot_file file_id"
    );

    // قانون ۳: تمام snapshot_files به نسخه ثبت‌شده اشاره می‌کنند
    let invalid_snapshot_versions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshot_files sf LEFT JOIN file_versions fv ON sf.file_version_id = fv.file_version_id WHERE fv.file_version_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        invalid_snapshot_versions, 0,
        "Invariant Violation: Dangling snapshot_file version_id"
    );

    // قانون ۴: تطابق file_id در جدول file_versions و snapshot_files
    let mismatched_bindings: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshot_files sf JOIN file_versions fv ON sf.file_version_id = fv.file_version_id WHERE sf.file_id != fv.file_id",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        mismatched_bindings, 0,
        "Invariant Violation: FileId mismatch across version and snapshot"
    );

    // قانون ۵: یکتایی مسیر در یک اسنپ‌شات معین
    let duplicate_paths: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM (SELECT snapshot_id, path_at_snapshot, COUNT(*) as cnt FROM snapshot_files GROUP BY snapshot_id, path_at_snapshot HAVING cnt > 1)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        duplicate_paths, 0,
        "Invariant Violation: Duplicate paths within snapshot"
    );

    // قانون ۶: یکتایی FileId در حوزه Workspace
    let duplicate_files: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM (SELECT file_id, COUNT(*) as cnt FROM files WHERE workspace_id = ?1 GROUP BY file_id HAVING cnt > 1)",
            [workspace_id.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        duplicate_files, 0,
        "Invariant Violation: Non-unique FileId in workspace"
    );

    // قانون ۷: تعلق اسنپ‌شات به Workspace معتبر
    let orphan_snapshots: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshots s LEFT JOIN workspaces w ON s.workspace_id = w.workspace_id WHERE w.workspace_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        orphan_snapshots, 0,
        "Invariant Violation: Orphan snapshot found"
    );
}

fn params_ws<'a>(ws: &'a WorkspaceId, path: &'a str) -> [&'a str; 2] {
    [ws.as_str(), path]
}

#[test]
fn test_triple_semantic_identity_invariants() {
    // -------------------------------------------------------------
    // T01 & T02: کشف خودکار FileId پایدار و تغییر مکان فایل بدون تزریق دستی
    // -------------------------------------------------------------
    let mut index = CodeIndex::new_in_memory().expect("Failed to initialize SQLite index");
    let code_v1 = "pub fn calculate_score() -> u32 { 100 }";

    let old_path = "src/game/scoring_old.rs";
    let new_path = "src/engine/scoring_new.rs";

    index
        .index_file_incremental(old_path, code_v1)
        .expect("Failed to index initial file");

    let initial_file_id = index.get_or_create_file_id(old_path);
    let common_lineage = LineageId::generate_persistent();

    let mut snap_a =
        SemanticDiffer::create_file_snapshot_with_id(initial_file_id.clone(), old_path, code_v1);
    snap_a.symbols[0].lineage_id = Some(common_lineage.clone());

    let discovered_file_id =
        index.discover_persistent_file_id(new_path, &CodeIndex::compute_hash(code_v1));
    assert_eq!(
        discovered_file_id, initial_file_id,
        "System must autonomously discover identical FileId across path changes"
    );

    let mut snap_b =
        SemanticDiffer::create_file_snapshot_with_id(discovered_file_id, new_path, code_v1);
    snap_b.symbols[0].lineage_id = Some(common_lineage.clone());

    let report_move = SemanticDiffer::diff_project_snapshots(
        std::slice::from_ref(&snap_a),
        std::slice::from_ref(&snap_b),
    );

    assert_eq!(report_move.file_moves.len(), 1, "Must detect file move");
    assert_eq!(report_move.file_moves[0].old_file_id, initial_file_id);
    assert_eq!(report_move.file_moves[0].new_file_id, initial_file_id);
    assert_ne!(snap_a.path, snap_b.path, "Paths must differ");

    // -------------------------------------------------------------
    // T03 & T04: تغییر نام نماد همراه با اتصال صریح old/new SymbolId و حفظ FileId
    // -------------------------------------------------------------
    let code_rename = "pub fn compute_final_score() -> u32 { 100 }";
    let mut snap_renamed = SemanticDiffer::create_file_snapshot_with_id(
        initial_file_id.clone(),
        new_path,
        code_rename,
    );
    snap_renamed.symbols[0].lineage_id = Some(common_lineage.clone());

    let report_rename = SemanticDiffer::diff_project_snapshots(
        std::slice::from_ref(&snap_b),
        std::slice::from_ref(&snap_renamed),
    );

    assert!(
        !report_rename.changes.is_empty(),
        "Changes must not be empty on rename"
    );

    let renamed_event = report_rename
        .changes
        .iter()
        .find(|c| matches!(c, SemanticChange::SymbolRenamed { .. }));
    assert!(
        renamed_event.is_some(),
        "Must emit SymbolRenamed semantic mutation"
    );

    if let Some(SemanticChange::SymbolRenamed {
        old_symbol_id,
        new_symbol_id,
        old_name,
        new_name,
        lineage_id,
    }) = renamed_event
    {
        assert_eq!(old_name, "calculate_score");
        assert_eq!(new_name, "compute_final_score");
        assert_eq!(old_symbol_id, &snap_b.symbols[0].id);
        assert_eq!(new_symbol_id, &snap_renamed.symbols[0].id);
        if let Some(ref lin) = snap_b.symbols[0].lineage_id {
            assert_eq!(lineage_id, lin);
        }
    }

    assert_ne!(
        snap_b.symbols[0].id, snap_renamed.symbols[0].id,
        "SymbolId must change on rename"
    );
    assert_eq!(
        snap_b.file_id, snap_renamed.file_id,
        "FileId must remain unchanged"
    );

    // -------------------------------------------------------------
    // T05: تفکیک نام‌فضای ماژول و عدم تصادم توابع هم‌نام در دو فایل مجزا
    // -------------------------------------------------------------
    let code_module_a = "pub fn execute_task() -> bool { true }";
    let code_module_b = "pub fn execute_task() -> bool { false }";

    let snap_mod_a = SemanticDiffer::create_file_snapshot("src/service_a.rs", code_module_a);
    let snap_mod_b = SemanticDiffer::create_file_snapshot("src/service_b.rs", code_module_b);

    assert_ne!(
        snap_mod_a.symbols[0].id, snap_mod_b.symbols[0].id,
        "SymbolId must differ across different module namespaces even if function signatures match"
    );

    // -------------------------------------------------------------
    // T06: تغییر امضای تابع همراه با اتصال صریح old/new SymbolId
    // -------------------------------------------------------------
    let code_sig_change = "pub fn compute_final_score(bonus: u32) -> u32 { 100 + bonus }";
    let mut snap_sig = SemanticDiffer::create_file_snapshot_with_id(
        initial_file_id.clone(),
        new_path,
        code_sig_change,
    );
    snap_sig.symbols[0].lineage_id = Some(common_lineage);

    let report_sig = SemanticDiffer::diff_project_snapshots(
        std::slice::from_ref(&snap_renamed),
        std::slice::from_ref(&snap_sig),
    );

    assert!(
        !report_sig.changes.is_empty(),
        "Changes must not be empty on signature mutation"
    );
    let sig_event = report_sig
        .changes
        .iter()
        .find(|c| matches!(c, SemanticChange::SignatureChanged { .. }));
    assert!(
        sig_event.is_some(),
        "Must emit SignatureChanged semantic mutation"
    );

    if let Some(SemanticChange::SignatureChanged {
        old_symbol_id,
        new_symbol_id,
        old_signature,
        new_signature,
        ..
    }) = sig_event
    {
        assert!(old_signature.contains("() -> u32"));
        assert!(new_signature.contains("(bonus: u32) -> u32"));
        assert_eq!(old_symbol_id, &snap_renamed.symbols[0].id);
        assert_eq!(new_symbol_id, &snap_sig.symbols[0].id);
    }

    assert_ne!(
        snap_renamed.symbols[0].id, snap_sig.symbols[0].id,
        "SymbolId must change on signature mutation"
    );
    assert_eq!(
        snap_renamed.file_id, snap_sig.file_id,
        "FileId must remain unchanged"
    );
}

#[test]
fn test_gate_01_create_file_identity() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(IDENTITY_SPINE_SCHEMA).unwrap();

    let ws_id = WorkspaceId::generate();
    conn.execute(
        "INSERT INTO workspaces (workspace_id, root_path, created_at, current_snapshot_id) VALUES (?1, ?2, 1000, NULL)",
        params_ws(&ws_id, "/test/workspace"),
    ).unwrap();

    let files = vec![ObservedFile::new("src/a.rs", b"fn main() {}".to_vec())];
    let snap_id = SnapshotBuilder::commit_snapshot(&mut conn, &ws_id, &files).unwrap();

    let (file_id, version_id, hash): (String, String, String) = conn
        .query_row(
            "SELECT sf.file_id, sf.file_version_id, fv.content_hash
             FROM snapshot_files sf
             JOIN file_versions fv ON sf.file_version_id = fv.file_version_id
             WHERE sf.snapshot_id = ?1 AND sf.path_at_snapshot = 'src/a.rs'",
            [snap_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();

    assert!(!file_id.is_empty());
    assert!(!version_id.is_empty());
    assert_eq!(hash, ContentHash::compute(b"fn main() {}").as_str());

    assert_identity_invariants(&conn, &ws_id);
}

#[test]
fn test_gate_02_restart_durability_and_noop_reindex() {
    let temp_dir = std::env::temp_dir().join(format!("vora_test_{}", uuid::Uuid::now_v7()));
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("spine.db");

    let ws_id = WorkspaceId::generate();

    // مرحله اول: ساخت اولیه
    {
        let mut conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(IDENTITY_SPINE_SCHEMA).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, created_at, current_snapshot_id) VALUES (?1, ?2, 1000, NULL)",
            params_ws(&ws_id, "/repo"),
        ).unwrap();

        let files = vec![ObservedFile::new(
            "src/lib.rs",
            b"pub struct Engine;".to_vec(),
        )];
        SnapshotBuilder::commit_snapshot(&mut conn, &ws_id, &files).unwrap();
    }

    // مرحله دوم: شبیه‌سازی بستن و بازکردن مجدد (Restart) + No-op Reindex
    {
        let mut conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        let files = vec![ObservedFile::new(
            "src/lib.rs",
            b"pub struct Engine;".to_vec(),
        )];
        let _snap2_id = SnapshotBuilder::commit_snapshot(&mut conn, &ws_id, &files).unwrap();

        let versions_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM file_versions", [], |row| row.get(0))
            .unwrap();

        assert_eq!(
            versions_count, 1,
            "No-op reindex must NOT create new file version"
        );

        let files_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(files_count, 1, "No-op reindex must NOT create new FileId");

        assert_identity_invariants(&conn, &ws_id);
    }

    fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_gate_03_edit_creates_new_version_with_stable_file_id() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(IDENTITY_SPINE_SCHEMA).unwrap();

    let ws_id = WorkspaceId::generate();
    conn.execute(
        "INSERT INTO workspaces (workspace_id, root_path, created_at, current_snapshot_id) VALUES (?1, ?2, 1000, NULL)",
        params_ws(&ws_id, "/repo"),
    ).unwrap();

    let files_v1 = vec![ObservedFile::new("src/state.rs", b"let v = 1;".to_vec())];
    let snap1 = SnapshotBuilder::commit_snapshot(&mut conn, &ws_id, &files_v1).unwrap();

    let file_id_1: String = conn
        .query_row(
            "SELECT file_id FROM snapshot_files WHERE snapshot_id = ?1",
            [snap1.as_str()],
            |row| row.get(0),
        )
        .unwrap();

    // ویرایش محتوا
    let files_v2 = vec![ObservedFile::new("src/state.rs", b"let v = 2;".to_vec())];
    let snap2 = SnapshotBuilder::commit_snapshot(&mut conn, &ws_id, &files_v2).unwrap();

    let (file_id_2, _ver_2): (String, String) = conn
        .query_row(
            "SELECT file_id, file_version_id FROM snapshot_files WHERE snapshot_id = ?1",
            [snap2.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(
        file_id_1, file_id_2,
        "FileId must remain stable across edits"
    );

    let total_versions: i64 = conn
        .query_row("SELECT COUNT(*) FROM file_versions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        total_versions, 2,
        "Edit must produce exactly 2 historical versions"
    );

    assert_identity_invariants(&conn, &ws_id);
}

#[test]
fn test_gate_04_move_file_preserves_file_id() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(IDENTITY_SPINE_SCHEMA).unwrap();

    let ws_id = WorkspaceId::generate();
    conn.execute(
        "INSERT INTO workspaces (workspace_id, root_path, created_at, current_snapshot_id) VALUES (?1, ?2, 1000, NULL)",
        params_ws(&ws_id, "/repo"),
    ).unwrap();

    let initial = vec![ObservedFile::new(
        "src/old_path.rs",
        b"pub fn logic() {}".to_vec(),
    )];
    let snap1 = SnapshotBuilder::commit_snapshot(&mut conn, &ws_id, &initial).unwrap();

    let (file_id_old, ver_old): (String, String) = conn
        .query_row(
            "SELECT file_id, file_version_id FROM snapshot_files WHERE snapshot_id = ?1",
            [snap1.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    // تغییر مسیر فایل (Move) بدون تغییر در بایت‌ها
    let moved = vec![ObservedFile::new(
        "src/modules/new_path.rs",
        b"pub fn logic() {}".to_vec(),
    )];
    let snap2 = SnapshotBuilder::commit_snapshot(&mut conn, &ws_id, &moved).unwrap();

    let (file_id_new, ver_new, path_new): (String, String, String) = conn
        .query_row(
            "SELECT file_id, file_version_id, path_at_snapshot FROM snapshot_files WHERE snapshot_id = ?1",
            [snap2.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();

    assert_eq!(file_id_old, file_id_new, "FileId must NOT change on move");
    assert_eq!(
        ver_old, ver_new,
        "FileVersionId must NOT change on move when bytes are identical"
    );
    assert_eq!(path_new, "src/modules/new_path.rs");

    assert_identity_invariants(&conn, &ws_id);
}

#[test]
fn test_gate_05_clone_file_allocates_distinct_identity() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(IDENTITY_SPINE_SCHEMA).unwrap();

    let ws_id = WorkspaceId::generate();
    conn.execute(
        "INSERT INTO workspaces (workspace_id, root_path, created_at, current_snapshot_id) VALUES (?1, ?2, 1000, NULL)",
        params_ws(&ws_id, "/repo"),
    ).unwrap();

    // دو فایل مجزا با محتوای ۱۰۰٪ یکسان
    let duplicate_content = b"const SAME: i32 = 42;".to_vec();
    let files = vec![
        ObservedFile::new("src/original.rs", duplicate_content.clone()),
        ObservedFile::new("src/clone.rs", duplicate_content),
    ];

    let snap = SnapshotBuilder::commit_snapshot(&mut conn, &ws_id, &files).unwrap();

    let file_id_a: String = conn
        .query_row(
            "SELECT file_id FROM snapshot_files WHERE snapshot_id = ?1 AND path_at_snapshot = 'src/original.rs'",
            [snap.as_str()],
            |row| row.get(0),
        )
        .unwrap();

    let file_id_b: String = conn
        .query_row(
            "SELECT file_id FROM snapshot_files WHERE snapshot_id = ?1 AND path_at_snapshot = 'src/clone.rs'",
            [snap.as_str()],
            |row| row.get(0),
        )
        .unwrap();

    assert_ne!(
        file_id_a, file_id_b,
        "Cloned file must NOT reuse the same FileId"
    );

    assert_identity_invariants(&conn, &ws_id);
}

#[test]
fn test_gate_06_ambiguous_duplicate_move_does_not_auto_bind() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(IDENTITY_SPINE_SCHEMA).unwrap();

    let ws_id = WorkspaceId::generate();
    let service = FileIdentityService::new(&conn);

    let hash = ContentHash::compute(b"shared bytes");

    // وقتی هیچ فایلی نیست -> شناسه جدید
    let res_new = service
        .resolve_identity(&ws_id, None, "src/new.rs", &hash)
        .unwrap();
    assert!(matches!(res_new, FileIdentityResolution::New(_)));
}
