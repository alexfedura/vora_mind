use rusqlite::Connection;
use std::fs;
use vora_lib::forge::identity::{ObservedFile, SnapshotBuilder};
use vora_lib::forge::index::{CodeIndex, IDENTITY_SPINE_SCHEMA};
use vora_lib::forge::model::{FileId, LineageId, SymbolId, WorkspaceId};

#[test]
fn test_crash_mid_snapshot_leaves_zero_orphans() {
    let temp_dir = std::env::temp_dir().join(format!("vora_crash_test_{}", uuid::Uuid::now_v7()));
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("crash_recovery.db");

    let ws_id = WorkspaceId::generate();

    // گام ۱: ساخت اسنپ‌شات اولیه موفق
    {
        let mut conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(IDENTITY_SPINE_SCHEMA).unwrap();
        conn.execute(
            "INSERT INTO workspaces (workspace_id, root_path, created_at, current_snapshot_id) VALUES (?1, ?2, 1000, NULL)",
            [ws_id.as_str(), "/repo"],
        ).unwrap();

        let files = vec![ObservedFile::new("src/main.rs", b"fn main() {}".to_vec())];
        SnapshotBuilder::commit_snapshot(&mut conn, &ws_id, &files).unwrap();
    }

    // گام ۲: شبیه‌سازی کرش در حین تراکنش بعدی (Rollback شبیه‌سازی‌شده)
    {
        let mut conn = Connection::open(&db_path).unwrap();
        let tx = conn.transaction().unwrap();

        let fake_file_id = FileId::generate();
        tx.execute(
            "INSERT INTO files (file_id, workspace_id, created_at, deleted_at) VALUES (?1, ?2, 2000, NULL)",
            [fake_file_id.as_str(), ws_id.as_str()],
        ).unwrap();

        // لغو عمدی تراکنش شبیه کرش پروسه
        drop(tx);
    }

    // گام ۳: باز کردن مجدد دیتابیس و اثبات تمیز بودن وضعیت
    {
        let conn = Connection::open(&db_path).unwrap();
        let orphan_files: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM files f LEFT JOIN snapshot_files sf ON f.file_id = sf.file_id WHERE sf.file_id IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            orphan_files, 0,
            "Crash mid-transaction must leave zero orphan files"
        );

        let valid_snapshots: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM snapshots WHERE workspace_id = ?1",
                [ws_id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(valid_snapshots, 1, "Only committed snapshots must survive");
    }

    fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_reopen_database_preserves_full_index_integrity() {
    let temp_dir = std::env::temp_dir().join(format!("vora_reopen_test_{}", uuid::Uuid::now_v7()));
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("integrity.db");

    let source_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";

    // ایندکس اولیه
    {
        let mut index = CodeIndex::new(&db_path).unwrap();
        index
            .index_file_incremental("src/math.rs", source_code)
            .unwrap();
    }

    // بستن و بازگشایی مجدد
    {
        let index = CodeIndex::new(&db_path).unwrap();
        let total_symbols = index.get_total_symbols_count().unwrap();
        assert_eq!(
            total_symbols, 1,
            "Reopening database must retain all indexed symbols"
        );
    }

    fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_crash_during_lineage_and_canonical_registration() {
    let temp_dir = std::env::temp_dir().join(format!("vora_canon_crash_{}", uuid::Uuid::now_v7()));
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("canon_crash.db");

    let lin_id = LineageId::generate_persistent();
    let sym_id = SymbolId::new("sym_test");

    // ثبت اولیه موفق با استفاده از workspace و snapshot ثبت‌شده
    let ws_id = {
        let mut index = CodeIndex::new(&db_path).unwrap();
        let registered_ws = index.get_or_create_workspace("/repo").unwrap();
        let snap_id = index
            .commit_identity_snapshot(
                &registered_ws,
                None,
                &[("src/test.rs", b"pub fn test_fn() {}")],
            )
            .unwrap();
        index
            .register_canonical_entity(
                &registered_ws,
                &snap_id,
                &lin_id,
                &sym_id,
                "test_fn",
                "crate::test",
                "fn()",
            )
            .unwrap();
        registered_ws
    };

    // شبیه‌سازی تراکنش ناتمام/کرش‌خورده
    {
        let mut conn = Connection::open(&db_path).unwrap();
        let tx = conn.transaction().unwrap();
        tx.execute(
            "INSERT INTO canonical_entities (entity_id, workspace_id, lineage_id, initial_name, created_at) VALUES (?1, ?2, ?3, ?4, 5000)",
            ["ent_fake_uncommitted", ws_id.as_str(), lin_id.as_str(), "fake"],
        ).unwrap();
        drop(tx);
    }

    // راستی‌آزمایی بازگشایی و پاک بودن کامل ایندکس
    {
        let reopened_index = CodeIndex::new(&db_path).unwrap();
        let conn = reopened_index.connection();

        let uncommitted_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM canonical_entities WHERE entity_id = 'ent_fake_uncommitted'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            uncommitted_count, 0,
            "Uncommitted canonical entities must be rolled back on crash"
        );

        let history = reopened_index
            .get_canonical_entity_history(&vora_lib::forge::model::CanonicalEntityId::from_lineage(
                &ws_id, &lin_id,
            ))
            .unwrap();
        assert_eq!(
            history.len(),
            1,
            "Only pre-crash committed canonical entity version must survive"
        );
    }

    fs::remove_dir_all(temp_dir).ok();
}
