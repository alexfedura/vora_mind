use vora_lib::forge::commit::ForgeTransactionManager;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::*;
use vora_lib::forge::overlay::CodeOverlayManager;

#[test]
fn test_p11_multi_file_atomic_transaction_and_lineage_preservation() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to open in-memory database");
    let ws_id = index
        .get_or_create_workspace("ws_p11_multi_file")
        .expect("Failed to create workspace");

    let file_a_path = "src/auth/token.rs";
    let file_a_code = r#"
pub fn validate_token(token_str: &str) -> bool {
    !token_str.is_empty()
}
"#;

    let file_b_path = "src/auth/service.rs";
    let file_b_code = r#"
use crate::auth::token::validate_token;

pub fn handle_login(req_token: &str) -> bool {
    validate_token(req_token)
}
"#;

    // ۱. ثبت اولیه وضعیت فایل‌ها در اسنپ‌شات اولیه
    let initial_files = [
        (file_a_path, file_a_code.as_bytes()),
        (file_b_path, file_b_code.as_bytes()),
    ];
    let snap_1 = index
        .commit_identity_snapshot(&ws_id, None, &initial_files)
        .expect("Failed to commit initial snapshot");

    let lin_token = LineageId::new("lin_validate_token");
    let lin_service = LineageId::new("lin_handle_login");
    let sym_token_v1 = SymbolId::new("sym_token_v1");
    let sym_service_v1 = SymbolId::new("sym_service_v1");

    let canon_token_id = index
        .register_canonical_entity(
            &ws_id,
            &snap_1,
            &lin_token,
            &sym_token_v1,
            "validate_token",
            "auth::token",
            "pub fn validate_token(token_str: &str) -> bool",
        )
        .expect("Failed to register validate_token canonical");

    let canon_service_id = index
        .register_canonical_entity(
            &ws_id,
            &snap_1,
            &lin_service,
            &sym_service_v1,
            "handle_login",
            "auth::service",
            "pub fn handle_login(req_token: &str) -> bool",
        )
        .expect("Failed to register handle_login canonical");

    // ۲. تعریف پچ‌های چندفایلی (Multi-File Refactoring)
    let new_token_code = r#"pub fn validate_token(token_str: &str, strict: bool) -> bool {
    if strict && token_str.len() < 8 {
        return false;
    }
    !token_str.is_empty()
}"#;

    let new_service_code = r#"pub fn handle_login(req_token: &str) -> bool {
    validate_token(req_token, true)
}"#;

    let mut overlay = CodeOverlayManager::new("commit_base_001", "task_p11_cascade");
    let mut tx_manager =
        ForgeTransactionManager::new("commit_base_001", "task_p11_cascade", &mut overlay);

    tx_manager.stage_base_file(file_a_path, file_a_code);
    tx_manager.stage_base_file(file_b_path, file_b_code);

    tx_manager.stage_patch(file_a_path, "validate_token", new_token_code);
    tx_manager.stage_patch(file_b_path, "handle_login", new_service_code);

    let (diff_report, patched_files) = tx_manager
        .validate_and_simulate()
        .expect("Validation and simulation should succeed for multi-file patch");

    assert_eq!(patched_files.len(), 2);
    assert!(!diff_report.changes.is_empty());

    // ۳. اعمال قطعی تراکنش اتمیک
    let commit_res = tx_manager
        .commit_transaction(
            &mut index,
            "Refactor token validation with strict policy across files",
        )
        .expect("Atomic multi-file commit must succeed");

    assert!(commit_res.is_committed);
    assert_eq!(commit_res.patched_files.len(), 2);

    // ۴. تایید پایداری موجودیت‌های کانونیکال پس از ریفکتور چندفایلی
    let history_token = index
        .get_canonical_entity_history(&canon_token_id)
        .expect("Token history query failed");
    assert!(!history_token.is_empty());

    let history_service = index
        .get_canonical_entity_history(&canon_service_id)
        .expect("Service history query failed");
    assert!(!history_service.is_empty());
}

#[test]
fn test_p11_multi_file_rollback_on_invalid_syntax_in_any_step() {
    let _index = CodeIndex::open_in_memory().expect("Failed to open in-memory database");

    let file_a_path = "src/math/calc.rs";
    let file_a_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";

    let file_b_path = "src/math/api.rs";
    let file_b_code = "pub fn execute_add() -> i32 { add(1, 2) }";

    let mut overlay = CodeOverlayManager::new("commit_base_002", "task_fail_test");
    let mut tx_manager =
        ForgeTransactionManager::new("commit_base_002", "task_fail_test", &mut overlay);

    tx_manager.stage_base_file(file_a_path, file_a_code);
    tx_manager.stage_base_file(file_b_path, file_b_code);

    // فایل ۱ معتبر است، ولی فایل ۲ دارای سینتکس خراب است
    tx_manager.stage_patch(
        file_a_path,
        "add",
        "pub fn add(a: i32, b: i32) -> i32 { a + b + 1 }",
    );
    tx_manager.stage_patch(
        file_b_path,
        "execute_add",
        "pub fn execute_add() -> { UNCLOSED SYNTAX ",
    );

    let sim_result = tx_manager.validate_and_simulate();
    assert!(
        sim_result.is_err(),
        "Simulation must fail if any step has invalid syntax"
    );

    tx_manager.rollback();
    assert!(
        overlay.state().dirty_files.is_empty(),
        "Overlay must be clean after rollback"
    );
}

#[test]
fn test_p11_01_three_tier_cross_file_atomic_success() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_p11_01")
        .expect("Failed to create workspace");

    let f_model = "src/models/user.rs";
    let c_model = "pub struct User { pub id: u64, pub name: String }\npub fn new_user(id: u64, name: &str) -> User { User { id, name: name.to_string() } }";

    let f_repo = "src/repos/user_repo.rs";
    let c_repo = "use crate::models::user::{User, new_user};\npub fn get_user_by_id(id: u64) -> User { new_user(id, \"default\") }";

    let f_service = "src/services/user_service.rs";
    let c_service = "use crate::repos::user_repo::get_user_by_id;\npub fn load_profile(id: u64) -> String { get_user_by_id(id).name }";

    let initial_files = [
        (f_model, c_model.as_bytes()),
        (f_repo, c_repo.as_bytes()),
        (f_service, c_service.as_bytes()),
    ];

    let snap_v1 = index
        .commit_identity_snapshot(&ws_id, None, &initial_files)
        .expect("Snapshot v1 failed");

    let mut overlay = CodeOverlayManager::new("v1", "task_three_tier");
    let mut tx = ForgeTransactionManager::new("v1", "task_three_tier", &mut overlay);

    tx.stage_base_file(f_model, c_model);
    tx.stage_base_file(f_repo, c_repo);
    tx.stage_base_file(f_service, c_service);

    let patch_model = "pub fn new_user(id: u64, name: &str) -> User { User { id, name: format!(\"user_{}\", name) } }";
    let patch_repo = "pub fn get_user_by_id(id: u64) -> User { new_user(id, \"verified\") }";
    let patch_service = "pub fn load_profile(id: u64) -> String { format!(\"Profile: {}\", get_user_by_id(id).name) }";

    tx.stage_patch(f_model, "new_user", patch_model);
    tx.stage_patch(f_repo, "get_user_by_id", patch_repo);
    tx.stage_patch(f_service, "load_profile", patch_service);

    let commit_res = tx
        .commit_transaction(
            &mut index,
            "refactor(user): propagate verified user name formatting across 3 tiers",
        )
        .expect("Three-tier multi-file transaction must succeed");

    assert!(commit_res.is_committed);
    let snap_v2 = commit_res.snapshot_id.expect("Snapshot v2 must exist");
    assert_ne!(snap_v1.0, snap_v2.0);

    let conn = index.connection();
    let mut stmt = conn
        .prepare("SELECT path_at_snapshot FROM snapshot_files WHERE snapshot_id = ?1 ORDER BY path_at_snapshot ASC")
        .unwrap();

    let files: Vec<String> = stmt
        .query_map([&snap_v2.0], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(files.len(), 3);
    assert_eq!(files[0], f_model);
    assert_eq!(files[1], f_repo);
    assert_eq!(files[2], f_service);
}

#[test]
fn test_p11_02_failure_injection_at_tier_three_leaves_zero_state_changes() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_p11_02")
        .expect("Failed to create workspace");

    let f1 = "src/core/config.rs";
    let c1 = "pub fn load_port() -> u16 { 8080 }";

    let f2 = "src/core/server.rs";
    let c2 = "pub fn start_server() -> u16 { load_port() }";

    let f3 = "src/core/metrics.rs";
    let c3 = "pub fn report_metrics() -> bool { true }";

    let snap_v1 = index
        .commit_identity_snapshot(
            &ws_id,
            None,
            &[
                (f1, c1.as_bytes()),
                (f2, c2.as_bytes()),
                (f3, c3.as_bytes()),
            ],
        )
        .expect("Initial snapshot failed");

    let mut overlay = CodeOverlayManager::new("v1", "task_fail_tier3");
    let mut tx = ForgeTransactionManager::new("v1", "task_fail_tier3", &mut overlay);

    tx.stage_base_file(f1, c1);
    tx.stage_base_file(f2, c2);
    tx.stage_base_file(f3, c3);

    tx.stage_patch(f1, "load_port", "pub fn load_port() -> u16 { 9090 }");
    tx.stage_patch(
        f2,
        "start_server",
        "pub fn start_server() -> u16 { load_port() + 1 }",
    );

    tx.stage_patch(
        f3,
        "report_metrics",
        "pub fn report_metrics() -> bool { broken_syntax(",
    );

    let res = tx.commit_transaction(&mut index, "Broken multi-file refactor");
    assert!(res.is_err(), "Transaction must abort when tier 3 fails");

    let current_snap: String = index
        .connection()
        .query_row(
            "SELECT current_snapshot_id FROM workspaces WHERE workspace_id = ?1",
            [&ws_id.0],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(current_snap, snap_v1.0);
}

#[test]
fn test_p11_03_lineage_and_canonical_durability_in_multi_patch() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_p11_03")
        .expect("Failed to create workspace");

    let f_api = "src/api/handler.rs";
    let c_api = "pub fn handle_request() -> u32 { 200 }";

    let snap_v1 = index
        .commit_identity_snapshot(&ws_id, None, &[(f_api, c_api.as_bytes())])
        .expect("Initial commit failed");

    let lin_api = LineageId::new("lin_handle_request");
    let sym_api_v1 = SymbolId::new("sym_handle_v1");

    index
        .record_lineage_version(&ws_id, &snap_v1, &lin_api, &sym_api_v1, Some(1.0))
        .unwrap();

    let canon_id_v1 = index
        .register_canonical_entity(
            &ws_id,
            &snap_v1,
            &lin_api,
            &sym_api_v1,
            "handle_request",
            "api::handler",
            "pub fn handle_request() -> u32",
        )
        .unwrap();

    let mut overlay = CodeOverlayManager::new("v1", "task_upgrade");
    let mut tx = ForgeTransactionManager::new("v1", "task_upgrade", &mut overlay);

    tx.stage_base_file(f_api, c_api);
    tx.stage_patch(
        f_api,
        "handle_request",
        "pub fn handle_request() -> u32 { 204 }",
    );

    let snap_v2 = tx
        .commit_transaction(&mut index, "Upgrade response code to 204")
        .unwrap()
        .snapshot_id
        .unwrap();

    let sym_api_v2 = SymbolId::new("sym_handle_v2");
    index
        .record_lineage_version(&ws_id, &snap_v2, &lin_api, &sym_api_v2, Some(0.99))
        .unwrap();

    let canon_id_v2 = index
        .register_canonical_entity(
            &ws_id,
            &snap_v2,
            &lin_api,
            &sym_api_v2,
            "handle_request",
            "api::handler",
            "pub fn handle_request() -> u32",
        )
        .unwrap();

    assert_eq!(
        canon_id_v1.0, canon_id_v2.0,
        "Canonical entity identity must be preserved across multi-file evolution"
    );
}

/// P11.5: آزمون استرس زنجیره‌ای چندفایلی ۳ لایه همراه با مقایسه AST Diff و یکپارچگی شجره کانونیکال
#[test]
fn test_p11_04_three_tier_cascading_ast_diff_stress() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_p11_stress")
        .expect("Failed to create workspace");

    let f_crypto = "src/core/crypto.rs";
    let c_crypto_v1 = "pub fn hash_payload(data: &str) -> String { format!(\"hash_{}\", data) }";

    let f_session = "src/auth/session.rs";
    let c_session_v1 = "use crate::core::crypto::hash_payload;\npub fn create_session(user_id: &str) -> String { hash_payload(user_id) }";

    let f_handler = "src/api/handlers.rs";
    let c_handler_v1 = "use crate::auth::session::create_session;\npub fn login_handler(username: &str) -> String { create_session(username) }";

    let initial_files = [
        (f_crypto, c_crypto_v1.as_bytes()),
        (f_session, c_session_v1.as_bytes()),
        (f_handler, c_handler_v1.as_bytes()),
    ];

    let snap_v1 = index
        .commit_identity_snapshot(&ws_id, None, &initial_files)
        .expect("Snapshot v1 failed");

    // ثبت شجره کانونیکال هر ۳ لایه
    let lin_crypto = LineageId::new("lin_hash_payload");
    let lin_session = LineageId::new("lin_create_session");
    let lin_handler = LineageId::new("lin_login_handler");

    let sym_c1 = SymbolId::new("sym_c1");
    let sym_s1 = SymbolId::new("sym_s1");
    let sym_h1 = SymbolId::new("sym_h1");

    let canon_c = index
        .register_canonical_entity(
            &ws_id,
            &snap_v1,
            &lin_crypto,
            &sym_c1,
            "hash_payload",
            "core::crypto",
            "pub fn hash_payload(data: &str) -> String",
        )
        .unwrap();
    let canon_s = index
        .register_canonical_entity(
            &ws_id,
            &snap_v1,
            &lin_session,
            &sym_s1,
            "create_session",
            "auth::session",
            "pub fn create_session(user_id: &str) -> String",
        )
        .unwrap();
    let canon_h = index
        .register_canonical_entity(
            &ws_id,
            &snap_v1,
            &lin_handler,
            &sym_h1,
            "login_handler",
            "api::handlers",
            "pub fn login_handler(username: &str) -> String",
        )
        .unwrap();

    // جهش امضای تابع لایه ۱ و انتشار آبشاری به لایه‌های ۲ و ۳
    let c_crypto_v2 = "pub fn hash_payload(data: &str, salt: u64) -> String { format!(\"hash_{}_{}\", data, salt) }";
    let c_session_v2 =
        "pub fn create_session(user_id: &str) -> String { hash_payload(user_id, 42) }";
    let c_handler_v2 = "pub fn login_handler(username: &str) -> String { format!(\"session:{}\", create_session(username)) }";

    let mut overlay = CodeOverlayManager::new("v1", "task_cascade_stress");
    let mut tx = ForgeTransactionManager::new("v1", "task_cascade_stress", &mut overlay);

    tx.stage_base_file(f_crypto, c_crypto_v1);
    tx.stage_base_file(f_session, c_session_v1);
    tx.stage_base_file(f_handler, c_handler_v1);

    tx.stage_patch(f_crypto, "hash_payload", c_crypto_v2);
    tx.stage_patch(f_session, "create_session", c_session_v2);
    tx.stage_patch(f_handler, "login_handler", c_handler_v2);

    let (diff_report, _) = tx
        .validate_and_simulate()
        .expect("Multi-file AST validation failed");
    assert_eq!(
        diff_report.changes.len(),
        3,
        "AST Differ must detect exactly 3 semantic mutations"
    );

    let commit_res = tx
        .commit_transaction(
            &mut index,
            "refactor(crypto): propagate salt parameter across 3 tiers",
        )
        .expect("Atomic commit failed");

    assert!(commit_res.is_committed);
    let snap_v2 = commit_res.snapshot_id.unwrap();

    // راستی‌آزمایی پایداری هر ۳ موجودیت کانونیکال در اسنپ‌شات جدید
    let sym_c2 = SymbolId::new("sym_c2");
    let sym_s2 = SymbolId::new("sym_s2");
    let sym_h2 = SymbolId::new("sym_h2");

    let canon_c2 = index
        .register_canonical_entity(
            &ws_id,
            &snap_v2,
            &lin_crypto,
            &sym_c2,
            "hash_payload",
            "core::crypto",
            "pub fn hash_payload(data: &str, salt: u64) -> String",
        )
        .unwrap();
    let canon_s2 = index
        .register_canonical_entity(
            &ws_id,
            &snap_v2,
            &lin_session,
            &sym_s2,
            "create_session",
            "auth::session",
            "pub fn create_session(user_id: &str) -> String",
        )
        .unwrap();
    let canon_h2 = index
        .register_canonical_entity(
            &ws_id,
            &snap_v2,
            &lin_handler,
            &sym_h2,
            "login_handler",
            "api::handlers",
            "pub fn login_handler(username: &str) -> String",
        )
        .unwrap();

    assert_eq!(canon_c.0, canon_c2.0);
    assert_eq!(canon_s.0, canon_s2.0);
    assert_eq!(canon_h.0, canon_h2.0);
}
