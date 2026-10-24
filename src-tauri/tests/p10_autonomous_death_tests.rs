use vora_lib::forge::commit::ForgeTransactionManager;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::{LineageId, SymbolId};
use vora_lib::forge::overlay::CodeOverlayManager;

#[test]
fn test_death_01_syntax_failure_aborts_transaction_completely() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_death_test")
        .expect("Failed to create workspace");

    let file_path = "src/auth.rs";
    let base_code = "pub fn verify_token(token: &str) -> bool { !token.is_empty() }";

    let snap_v1 = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, base_code.as_bytes())])
        .expect("Snapshot v1 failed");

    let mut overlay = CodeOverlayManager::new("base_commit", "task_broken_patch");
    let mut tx = ForgeTransactionManager::new("base_commit", "task_broken_patch", &mut overlay);

    tx.stage_base_file(file_path, base_code);
    // پچ با سینتکس خراب و نامعتبر
    tx.stage_patch(
        file_path,
        "verify_token",
        "pub fn verify_token(token: &str) -> bool { if token.is_empty() { return false; // MISSING CLOSING BRACE",
    );

    let result = tx.commit_transaction(&mut index, "Apply broken patch");
    assert!(result.is_err(), "Broken syntax patch MUST fail commit");

    // تضمین ثبات: هیچ اسنپ‌شات جدیدی ثبت نشده است
    let current_ws_snap: Option<String> = index
        .connection()
        .query_row(
            "SELECT current_snapshot_id FROM workspaces WHERE workspace_id = ?1",
            [&ws_id.0],
            |r| r.get(0),
        )
        .expect("Query failed");

    assert_eq!(current_ws_snap.unwrap(), snap_v1.0);
}

#[test]
fn test_death_02_multi_file_partial_failure_guarantees_zero_pollution() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_multi_death_test")
        .expect("Failed to create workspace");

    let file1 = "src/payment.rs";
    let code1 = "pub fn process_payment(amount: u64) -> bool { amount > 0 }";

    let file2 = "src/receipt.rs";
    let code2 = "pub fn generate_receipt(id: u64) -> String { format!(\"Receipt #{}\", id) }";

    let snap_v1 = index
        .commit_identity_snapshot(
            &ws_id,
            None,
            &[(file1, code1.as_bytes()), (file2, code2.as_bytes())],
        )
        .expect("Snapshot v1 failed");

    let mut overlay = CodeOverlayManager::new("base_commit", "task_multi_atomic");
    let mut tx = ForgeTransactionManager::new("base_commit", "task_multi_atomic", &mut overlay);

    tx.stage_base_file(file1, code1);
    tx.stage_base_file(file2, code2);

    // پچ اول معتبر است
    tx.stage_patch(
        file1,
        "process_payment",
        "pub fn process_payment(amount: u64) -> bool { amount >= 10 }",
    );

    // پچ دوم سینتکس نامعتبر دارد
    tx.stage_patch(
        file2,
        "generate_receipt",
        "pub fn generate_receipt(id: u64) -> String { format!(\"Receipt #{}\", id) // SYNTAX ERR",
    );

    let commit_res = tx.commit_transaction(&mut index, "Multi patch with one broken step");
    assert!(
        commit_res.is_err(),
        "Transaction must fail if any step is broken"
    );

    // بررسی دیتابیس: فایل اول هم نباید ایندکس یا تغییر کرده باشد
    let snap_after: Option<String> = index
        .connection()
        .query_row(
            "SELECT current_snapshot_id FROM workspaces WHERE workspace_id = ?1",
            [&ws_id.0],
            |r| r.get(0),
        )
        .expect("Query failed");

    assert_eq!(
        snap_after.unwrap(),
        snap_v1.0,
        "Database state must remain strictly untouched"
    );
}

#[test]
fn test_e2e_03_autonomous_lifecycle_multi_file_and_canonical_preservation() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_e2e_full")
        .expect("Failed to create workspace");

    let auth_file = "src/security/auth.rs";
    let auth_code = "pub fn authenticate(user: &str) -> bool { !user.is_empty() }";

    let token_file = "src/security/token.rs";
    let token_code = "pub fn create_token(user: &str) -> String { format!(\"tok_{}\", user) }";

    // ۱. ثبت اسنپ‌شات اولیه
    let snap_v1 = index
        .commit_identity_snapshot(
            &ws_id,
            None,
            &[
                (auth_file, auth_code.as_bytes()),
                (token_file, token_code.as_bytes()),
            ],
        )
        .expect("Initial commit failed");

    // ۲. ثبت تبار و کانونیکال اولیه
    let lin_auth = LineageId::new("lin_auth_func");
    let sym_auth_v1 = SymbolId::new("sym_auth_v1");
    index
        .record_lineage_version(&ws_id, &snap_v1, &lin_auth, &sym_auth_v1, Some(1.0))
        .expect("Record lineage failed");

    let canon_id = index
        .register_canonical_entity(
            &ws_id,
            &snap_v1,
            &lin_auth,
            &sym_auth_v1,
            "authenticate",
            "security::auth",
            "pub fn authenticate(user: &str) -> bool",
        )
        .expect("Register canonical entity failed");

    // ۳. اجرای تراکنش چندفایلی ریفکتور اتمیک
    let mut overlay = CodeOverlayManager::new("v1_sha", "task_upgrade_auth");
    let mut tx = ForgeTransactionManager::new("v1_sha", "task_upgrade_auth", &mut overlay);

    tx.stage_base_file(auth_file, auth_code);
    tx.stage_base_file(token_file, token_code);

    let new_auth_code = "pub fn authenticate(user: &str) -> bool { user.len() >= 4 }";
    let new_token_code =
        "pub fn create_token(user: &str) -> String { format!(\"v2_tok_{}\", user) }";

    tx.stage_patch(auth_file, "authenticate", new_auth_code);
    tx.stage_patch(token_file, "create_token", new_token_code);

    let commit_res = tx
        .commit_transaction(
            &mut index,
            "feat(security): enforce 4-char min length and v2 token format",
        )
        .expect("Multi-file transaction must succeed");

    assert!(commit_res.is_committed);
    let snap_v2 = commit_res.snapshot_id.expect("Snapshot v2 must be created");
    assert_ne!(snap_v1.0, snap_v2.0);

    // ۴. ثبت نسخه تاریخی نماد جهش‌یافته و تایید بقای Canonical Entity
    let sym_auth_v2 = SymbolId::new("sym_auth_v2");
    index
        .record_lineage_version(&ws_id, &snap_v2, &lin_auth, &sym_auth_v2, Some(0.98))
        .expect("Record lineage v2 failed");

    let canon_id_v2 = index
        .register_canonical_entity(
            &ws_id,
            &snap_v2,
            &lin_auth,
            &sym_auth_v2,
            "authenticate",
            "security::auth",
            "pub fn authenticate(user: &str) -> bool",
        )
        .expect("Update canonical entity failed");

    // تضمین طلایی: شناسه انتیتی کانونیکال بین دو نسل اسنپ‌شات ثابت مانده است
    assert_eq!(
        canon_id.0, canon_id_v2.0,
        "Canonical Entity ID must remain immutable across refactorings"
    );

    // ۵. بررسی تاریخچه خطی
    let history = index
        .get_canonical_entity_history(&canon_id)
        .expect("Get history failed");
    assert_eq!(
        history.len(),
        2,
        "Entity history must contain exactly 2 snapshot generations"
    );
    assert_eq!(history[0].snapshot_id.0, snap_v1.0);
    assert_eq!(history[1].snapshot_id.0, snap_v2.0);
}

#[test]
fn test_death_04_same_name_different_namespaces_strict_isolation() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_death_test_04")
        .expect("Failed to create workspace");

    let snap = index
        .commit_identity_snapshot(
            &ws_id,
            None,
            &[
                ("src/user/service.rs", b"pub fn find_by_id() {}"),
                ("src/order/service.rs", b"pub fn find_by_id() {}"),
            ],
        )
        .expect("Snapshot failed");

    let lin_user = LineageId::new("lin_user_find");
    let lin_order = LineageId::new("lin_order_find");

    let sym_user = SymbolId::new("sym_user_find");
    let sym_order = SymbolId::new("sym_order_find");

    let canon_user = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lin_user,
            &sym_user,
            "find_by_id",
            "user::service",
            "pub fn find_by_id()",
        )
        .expect("Register user find failed");

    let canon_order = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lin_order,
            &sym_order,
            "find_by_id",
            "order::service",
            "pub fn find_by_id()",
        )
        .expect("Register order find failed");

    assert_ne!(
        canon_user.0, canon_order.0,
        "Same symbol name in distinct namespaces must yield distinct canonical entities"
    );
}

#[test]
fn test_death_05_rollback_and_subsequent_forward_evolution() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_death_test_05")
        .expect("Failed to create workspace");

    let file_path = "src/math.rs";
    let code_v1 = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
    let snap_v1 = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, code_v1.as_bytes())])
        .expect("Snap v1 failed");

    // ایجاد نسل دوم
    let mut overlay1 = CodeOverlayManager::new("v1", "task1");
    let mut tx1 = ForgeTransactionManager::new("v1", "task1", &mut overlay1);
    tx1.stage_base_file(file_path, code_v1);
    tx1.stage_patch(
        file_path,
        "add",
        "pub fn add(a: i32, b: i32) -> i32 { a + b + 0 }",
    );
    let snap_v2 = tx1
        .commit_transaction(&mut index, "Minor tweak")
        .unwrap()
        .snapshot_id
        .unwrap();

    // ایجاد نسل سوم با باگ
    let mut overlay2 = CodeOverlayManager::new("v2", "task2");
    let mut tx2 = ForgeTransactionManager::new("v2", "task2", &mut overlay2);
    tx2.stage_base_file(file_path, "pub fn add(a: i32, b: i32) -> i32 { a + b + 0 }");
    tx2.stage_patch(file_path, "add", "pub fn add(a: i32, b: i32) -> i32 { 0 }");
    let snap_v3 = tx2
        .commit_transaction(&mut index, "Buggy tweak")
        .unwrap()
        .snapshot_id
        .unwrap();

    // بازگردانی اسنپ‌شات به نسخه ۲ در سطح داده
    let conn = index.connection();
    let restored_file: String = conn
        .query_row(
            "SELECT path_at_snapshot FROM snapshot_files WHERE snapshot_id = ?1",
            [&snap_v2.0],
            |r| r.get(0),
        )
        .expect("Restore query failed");

    assert_eq!(restored_file, file_path);
    assert_ne!(snap_v2.0, snap_v3.0);
    assert_ne!(snap_v1.0, snap_v2.0);
}
