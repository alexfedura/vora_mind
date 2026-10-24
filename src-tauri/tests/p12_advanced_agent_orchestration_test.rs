use vora_lib::forge::commit::ForgeTransactionManager;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::{LineageId, SymbolId};
use vora_lib::forge::overlay::CodeOverlayManager;

#[test]
fn test_p12_01_ambiguity_disambiguation_by_file_path() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_p12_01")
        .expect("Failed to create workspace");

    let file_auth = "src/auth/service.rs";
    let code_auth = "pub fn authenticate() -> bool { true }";

    let file_payment = "src/payment/service.rs";
    let code_payment = "pub fn authenticate() -> bool { false }";

    let snap = index
        .commit_identity_snapshot(
            &ws_id,
            None,
            &[
                (file_auth, code_auth.as_bytes()),
                (file_payment, code_payment.as_bytes()),
            ],
        )
        .expect("Commit snapshot failed");

    let lin_auth = LineageId::new("lin_auth_svc");
    let lin_pay = LineageId::new("lin_pay_svc");

    let sym_auth = SymbolId::new("sym_auth_svc");
    let sym_pay = SymbolId::new("sym_pay_svc");

    let canon_auth = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lin_auth,
            &sym_auth,
            "authenticate",
            "auth::service",
            "pub fn authenticate() -> bool",
        )
        .expect("Register auth canonical failed");

    let canon_pay = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lin_pay,
            &sym_pay,
            "authenticate",
            "payment::service",
            "pub fn authenticate() -> bool",
        )
        .expect("Register payment canonical failed");

    assert_ne!(
        canon_auth.0, canon_pay.0,
        "Ambiguous names must remain strictly isolated entities"
    );

    // بررسی تطبیق دقیق موجودیت‌ها از روی لیست کل کانونیکال‌ها
    let all_entities = index
        .get_canonical_entities()
        .expect("Get canonical entities failed");
    assert_eq!(all_entities.len(), 2);

    let auth_entry = all_entities
        .iter()
        .find(|e| e.module_namespace == "auth::service")
        .unwrap();
    let pay_entry = all_entities
        .iter()
        .find(|e| e.module_namespace == "payment::service")
        .unwrap();

    assert_eq!(auth_entry.name, "authenticate");
    assert_eq!(pay_entry.name, "authenticate");
    assert_ne!(auth_entry.entity_id, pay_entry.entity_id);
}

#[test]
fn test_p12_02_snapshot_provenance_and_transaction_audit() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_p12_02")
        .expect("Failed to create workspace");

    let file_path = "src/analytics.rs";
    let code_v1 = "pub fn track_event(name: &str) -> bool { !name.is_empty() }";

    let snap_v1 = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, code_v1.as_bytes())])
        .expect("Commit snapshot v1 failed");

    let mut overlay = CodeOverlayManager::new("v1", "task_analytics_upgrade");
    let mut tx = ForgeTransactionManager::new("v1", "task_analytics_upgrade", &mut overlay);

    tx.stage_base_file(file_path, code_v1);
    tx.stage_patch(
        file_path,
        "track_event",
        "pub fn track_event(name: &str) -> bool { name.len() >= 3 }",
    );

    let commit_res = tx
        .commit_transaction(
            &mut index,
            "feat(analytics): enforce 3-char minimum event length",
        )
        .expect("Commit transaction must succeed");

    assert!(commit_res.is_committed);
    let snap_v2 = commit_res.snapshot_id.expect("Snapshot v2 ID must exist");
    assert_ne!(snap_v1.0, snap_v2.0);

    // تضمین اتصال اسنپ‌شات فرزند به پدر در ورک‌اسپیس
    let current_ws_snap: Option<String> = index
        .connection()
        .query_row(
            "SELECT current_snapshot_id FROM workspaces WHERE workspace_id = ?1",
            [&ws_id.0],
            |r| r.get(0),
        )
        .expect("Query failed");

    assert_eq!(current_ws_snap.unwrap(), snap_v2.0);
}
