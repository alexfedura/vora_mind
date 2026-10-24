use vora_lib::forge::commit::ForgeTransactionManager;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::overlay::CodeOverlayManager;

#[test]
fn test_p14_snapshot_restoration_and_content_retrieval() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize in-memory database");
    let ws_id = index
        .get_or_create_workspace("ws_p14_test")
        .expect("Failed to create workspace");

    let file_path = "src/storage/engine.rs";
    let initial_content = "pub fn read_block() -> u32 { 1024 }";

    // ۱. ثبت اسنپ‌شات اولیه
    let initial_files = [(file_path, initial_content.as_bytes())];
    let snap_v1 = index
        .commit_identity_snapshot(&ws_id, None, &initial_files)
        .expect("Failed to commit initial snapshot");

    // ۲. اعمال پچ جراحی و ایجاد اسنپ‌شات دوم
    let mut overlay = CodeOverlayManager::new("commit_v1", "task_update_engine");
    let mut tx_manager =
        ForgeTransactionManager::new("commit_v1", "task_update_engine", &mut overlay);

    tx_manager.stage_base_file(file_path, initial_content);
    tx_manager.stage_patch(
        file_path,
        "read_block",
        "pub fn read_block() -> u32 { 2048 }",
    );

    let commit_res = tx_manager
        .commit_transaction(&mut index, "Upgrade block size to 2048")
        .expect("Commit transaction must succeed");

    assert!(commit_res.is_committed);
    let snap_v2 = commit_res
        .snapshot_id
        .expect("Snapshot ID must be generated");

    // ۳. اعتبارسنجی واکشی اسنپ‌شات اول مستقیماً از اسکیما
    let conn = index.connection();
    let mut stmt = conn
        .prepare(
            "
            SELECT sf.path_at_snapshot, fv.content_hash
            FROM snapshot_files sf
            JOIN file_versions fv ON sf.file_version_id = fv.file_version_id
            WHERE sf.snapshot_id = ?1
            ",
        )
        .expect("Failed to prepare query");

    let restored_v1: Vec<(String, String)> = stmt
        .query_map([&snap_v1.0], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("Query execution failed")
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(restored_v1.len(), 1);
    assert_eq!(restored_v1[0].0, file_path);

    // ۴. اعتبارسنجی واکشی اسنپ‌شات دوم
    let restored_v2: Vec<(String, String)> = stmt
        .query_map([&snap_v2.0], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("Query execution failed")
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(restored_v2.len(), 1);
    assert_eq!(restored_v2[0].0, file_path);
    assert_ne!(
        restored_v1[0].1, restored_v2[0].1,
        "Content hash must evolve across snapshots"
    );
}
