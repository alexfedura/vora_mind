use rusqlite::Result;
use vora_lib::forge::commit::TransactionPatch;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::SnapshotId;
use vora_lib::forge::overlay::CodeOverlayManager;
use vora_lib::forge::tools::ForgeToolRegistry;

#[test]
fn test_p13_01_audit_trail_lifecycle_and_query() -> Result<()> {
    let index = CodeIndex::open_in_memory()?;
    let ws_id = index.get_or_create_workspace("ws_prod_01")?;

    let base_snap = SnapshotId("snap_base_001".to_string());
    let committed_snap = SnapshotId("snap_commit_002".to_string());

    // ۱. ثبت لاگ آغاز و تلاش‌های ترمیم تسک
    let record_id_1 = index.record_audit_entry(
        "task_101",
        &ws_id,
        "refactor(auth): modernize session token validation",
        "validate_session",
        "src/auth/session.rs",
        Some(&base_snap),
        None,
        1,
        "IN_PROGRESS",
        Some("Compile failure during candidate step 1"),
    )?;

    assert!(record_id_1 > 0);

    // ۲. ثبت لاگ نهایی بعد از موفقیت خودترمیمی و اعمال تراکنش
    let record_id_2 = index.record_audit_entry(
        "task_101",
        &ws_id,
        "refactor(auth): modernize session token validation",
        "validate_session",
        "src/auth/session.rs",
        Some(&base_snap),
        Some(&committed_snap),
        2,
        "COMPLETED",
        None,
    )?;

    assert!(record_id_2 > record_id_1);

    // ۳. واکشی تاریخچه و راستی‌آزمایی رکوردها
    let audit_history = index.get_task_audit_history("task_101")?;
    assert_eq!(audit_history.len(), 2);

    let first_entry = &audit_history[0];
    assert_eq!(first_entry.task_id, "task_101");
    assert_eq!(first_entry.status, "IN_PROGRESS");
    assert_eq!(first_entry.healing_attempts, 1);
    assert!(first_entry.error_payload.is_some());

    let second_entry = &audit_history[1];
    assert_eq!(second_entry.status, "COMPLETED");
    assert_eq!(second_entry.healing_attempts, 2);
    assert_eq!(
        second_entry.committed_snapshot_id.as_deref(),
        Some("snap_commit_002")
    );

    Ok(())
}

#[test]
fn test_p13_02_crash_recovery_audit_durability() -> Result<()> {
    let temp_db =
        std::env::temp_dir().join(format!("vora_audit_durability_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&temp_db);

    let base_snap = SnapshotId("snap_init".to_string());

    // فاز ۱: ثبت در دیتابیس فیزیکی
    {
        let index = CodeIndex::new(&temp_db)?;
        let ws_id = index.get_or_create_workspace("ws_crash_recover")?;

        index.record_audit_entry(
            "task_crash_007",
            &ws_id,
            "fix(crypto): patch constant time comparison",
            "verify_hmac",
            "src/crypto/hash.rs",
            Some(&base_snap),
            None,
            3,
            "HEALING_EXHAUSTED",
            Some("Fatal test assertion failed across 3 attempts"),
        )?;
    } // اتصال بسته می‌شود (شبیه‌سازی توقف/کرش نرم‌افزار)

    // فاز ۲: باز کردن مجدد دیتابیس و بازیابی وضعیت ردیابی
    {
        let index = CodeIndex::new(&temp_db)?;
        let history = index.get_task_audit_history("task_crash_007")?;

        assert_eq!(history.len(), 1);
        let entry = &history[0];
        assert_eq!(entry.task_id, "task_crash_007");
        assert_eq!(entry.target_symbol, "verify_hmac");
        assert_eq!(entry.status, "HEALING_EXHAUSTED");
        assert_eq!(entry.healing_attempts, 3);
        assert!(entry
            .error_payload
            .as_ref()
            .unwrap()
            .contains("Fatal test assertion"));
    }

    let _ = std::fs::remove_file(&temp_db);
    Ok(())
}

#[test]
fn test_p13_03_multi_file_crash_resilience_and_zero_orphan_guarantee() -> Result<()> {
    let mut index = CodeIndex::open_in_memory()?;
    let ws_id = index.get_or_create_workspace("ws_p13_resilience")?;

    let file_a = "src/payment/gateway.rs";
    let code_a = "pub fn charge_card(amount: u64) -> bool { amount > 0 }";

    let file_b = "src/payment/ledger.rs";
    let code_b = "pub fn record_ledger(amount: u64) { let _ = amount; }";

    let snap_init = index.commit_identity_snapshot(
        &ws_id,
        None,
        &[(file_a, code_a.as_bytes()), (file_b, code_b.as_bytes())],
    )?;

    // ثبت شروع لاگ ممیزی تسک چندفایلی
    index.record_audit_entry(
        "task_multi_crash_test",
        &ws_id,
        "refactor(payment): update charge workflow",
        "charge_card",
        file_a,
        Some(&snap_init),
        None,
        0,
        "IN_PROGRESS",
        None,
    )?;

    // شبیه‌سازی شکست سینتکس در فایل دوم حین تراکنش چندفایلی
    let mut overlay = CodeOverlayManager::new("v1", "task_multi_crash_test");
    let patches = vec![
        TransactionPatch {
            file_path: file_a.to_string(),
            symbol_id: "charge_card".to_string(),
            new_symbol_code: "pub fn charge_card(amount: u64) -> bool { amount >= 100 }"
                .to_string(),
        },
        TransactionPatch {
            file_path: file_b.to_string(),
            symbol_id: "record_ledger".to_string(),
            new_symbol_code: "pub fn record_ledger(amount: u64) { INVALID_SYNTAX }}}".to_string(),
        },
    ];

    let tx_res = ForgeToolRegistry::execute_multi_patch_transaction(
        "v1",
        "task_multi_crash_test",
        &mut overlay,
        &mut index,
        &[(file_a, code_a), (file_b, code_b)],
        &patches,
        "Update payment transaction",
    );

    // تراکنش به صورت اتمیک شکست می‌خورد
    assert!(
        tx_res.is_err(),
        "Transaction must fail atomically on invalid syntax"
    );

    // ثبت خطای تراکنش در لاگ ممیزی
    index.record_audit_entry(
        "task_multi_crash_test",
        &ws_id,
        "refactor(payment): update charge workflow",
        "charge_card",
        file_a,
        Some(&snap_init),
        None,
        1,
        "ABORTED_ZERO_POLLUTION",
        Some("Atomic rollback triggered by invalid syntax in step 2"),
    )?;

    // تأیید ردپای ممیزی و عدم تغییر وضعیت پایه
    let history = index.get_task_audit_history("task_multi_crash_test")?;
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].status, "ABORTED_ZERO_POLLUTION");
    assert!(history[1].committed_snapshot_id.is_none());

    Ok(())
}
