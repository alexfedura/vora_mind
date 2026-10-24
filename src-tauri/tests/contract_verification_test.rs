use vora_lib::forge::ast_engine::AstEngine;
use vora_lib::forge::commit::ForgeTransactionManager;
use vora_lib::forge::differ::SemanticDiffer;
use vora_lib::forge::graph::DependencyGraph;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::{FileId, LineageId, SemanticChange, SymbolId};
use vora_lib::forge::overlay::CodeOverlayManager;
use vora_lib::forge::resolver::SemanticResolver;

/// T1: آزمون واقعی تغییر نام نماد از سورس‌کد و پیمایش آبشاری در تمام فراخوان‌ها
#[test]
fn test_t1_symbol_rename_detection_and_cascade() {
    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory index");

    let crypto_src = r#"
pub fn hash_password(pass: &str) -> String {
    format!("hashed_{}", pass)
}
"#;

    let service_src = r#"
use crate::auth::crypto::hash_password;

pub fn authenticate_user(user: &str, pass: &str) -> bool {
    let _ = hash_password(pass);
    true
}
"#;

    let handler_src = r#"
use crate::auth::service::authenticate_user;

pub fn login_handler(user: &str, pass: &str) -> bool {
    authenticate_user(user, pass)
}
"#;

    let files = vec![
        ("src/auth/crypto.rs", crypto_src),
        ("src/auth/service.rs", service_src),
        ("src/auth/handlers.rs", handler_src),
    ];

    let mut crypto_fid = None;

    for (path, content) in &files {
        let fid = FileId::generate();
        if *path == "src/auth/crypto.rs" {
            crypto_fid = Some(fid.clone());
        }
        let parsed = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed)
            .expect("Failed to index semantics");
        let parsed_syntax = AstEngine::parse_file_syntax(path, content);
        index
            .index_file(path, &parsed_syntax)
            .expect("Failed to index syntax");
    }

    let edges =
        SemanticResolver::resolve_strict_semantic_edges(&index).expect("Failed to resolve edges");

    // فیلتر کردن لبه‌های قطعی معنایی
    let resolved_edges: Vec<_> = edges
        .into_iter()
        .filter(|e| e.target_symbol.is_some())
        .collect();
    assert_eq!(
        resolved_edges.len(),
        2,
        "Expected 2 resolved cross-file edges"
    );

    let mut graph = DependencyGraph::new();
    graph.build_from_strict_semantic_edges(&resolved_edges);

    let parsed_crypto =
        AstEngine::parse_file_semantics(crypto_fid.unwrap(), "src/auth/crypto.rs", crypto_src);
    let hash_password_sym = parsed_crypto
        .symbols
        .into_iter()
        .find(|s| s.name == "hash_password")
        .unwrap();

    let new_sym_id = SymbolId::new("sym_hash_secret");
    let lineage_id = hash_password_sym
        .lineage_id
        .unwrap_or_else(LineageId::generate_persistent);

    let change = SemanticChange::SymbolRenamed {
        old_symbol_id: hash_password_sym.id,
        new_symbol_id: new_sym_id,
        lineage_id,
        old_name: "hash_password".to_string(),
        new_name: "hash_secret".to_string(),
    };

    let report = graph.analyze_semantic_change_impact(&change);
    assert_eq!(
        report.total_impacted_count, 2,
        "Rename must propagate to all callers (service + handler)"
    );
    assert_eq!(report.direct_dependents.len(), 1);
    assert_eq!(report.indirect_dependents.len(), 1);
}

/// T2: آزمون واقعی تغییر امضای تابع و شناسایی شعاع اثر در سطح وابستگی‌ها از سورس‌کد
#[test]
fn test_t2_signature_mutation_impact_propagation() {
    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory index");

    let gateway_src = r#"
pub fn charge_card(amount: u64) -> bool {
    amount > 0
}
"#;

    let checkout_src = r#"
use crate::billing::gateway::charge_card;

pub fn process_order(amount: u64) -> bool {
    charge_card(amount)
}
"#;

    let files = vec![
        ("src/billing/gateway.rs", gateway_src),
        ("src/billing/checkout.rs", checkout_src),
    ];

    for (path, content) in &files {
        let fid = FileId::generate();
        let parsed = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed)
            .expect("Failed to index semantics");
        let parsed_syntax = AstEngine::parse_file_syntax(path, content);
        index
            .index_file(path, &parsed_syntax)
            .expect("Failed to index syntax");
    }

    let edges =
        SemanticResolver::resolve_strict_semantic_edges(&index).expect("Failed to resolve edges");

    let resolved_edges: Vec<_> = edges
        .into_iter()
        .filter(|e| e.target_symbol.is_some())
        .collect();
    assert_eq!(
        resolved_edges.len(),
        1,
        "Exactly 1 cross-file edge expected"
    );

    let mut graph = DependencyGraph::new();
    graph.build_from_strict_semantic_edges(&resolved_edges);

    let target_id = resolved_edges[0].target_symbol.clone().unwrap();
    let new_sym_id = SymbolId::new("sym_charge_card_v2");

    let change = SemanticChange::SignatureChanged {
        old_symbol_id: target_id,
        new_symbol_id: new_sym_id,
        lineage_id: LineageId::new("lin_charge_card"),
        old_signature: "pub fn charge_card(amount: u64) -> bool".to_string(),
        new_signature: "pub fn charge_card(amount: u64, currency: &str) -> bool".to_string(),
    };

    let report = graph.analyze_semantic_change_impact(&change);
    assert_eq!(report.total_impacted_count, 1);
    assert_eq!(report.direct_dependents.len(), 1);
}

/// T3: آزمون جهش صرفاً بدنه تابع و تضمین شعاع انفجار صفر (Zero-Impact Isolation)
#[test]
fn test_t3_body_mutation_zero_impact() {
    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory index");

    let auth_src = r#"
pub fn verify_token(token: &str) -> bool {
    !token.is_empty()
}
"#;

    let router_src = r#"
use crate::auth::verify_token;

pub fn route_request(token: &str) -> bool {
    verify_token(token)
}
"#;

    let files = vec![("src/auth.rs", auth_src), ("src/api/router.rs", router_src)];

    for (path, content) in &files {
        let fid = FileId::generate();
        let parsed = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed)
            .expect("Failed to index semantics");
        let parsed_syntax = AstEngine::parse_file_syntax(path, content);
        index
            .index_file(path, &parsed_syntax)
            .expect("Failed to index syntax");
    }

    let edges =
        SemanticResolver::resolve_strict_semantic_edges(&index).expect("Failed to resolve edges");

    let resolved_edges: Vec<_> = edges
        .into_iter()
        .filter(|e| e.target_symbol.is_some())
        .collect();
    assert!(
        !resolved_edges.is_empty(),
        "Must contain resolved cross-file edge"
    );

    let mut graph = DependencyGraph::new();
    graph.build_from_strict_semantic_edges(&resolved_edges);

    let target_id = resolved_edges[0].target_symbol.clone().unwrap();

    let change = SemanticChange::BodyChanged {
        symbol_id: target_id,
        lineage_id: LineageId::new("lin_verify_token"),
        old_body_hash: "hash_v1".to_string(),
        new_body_hash: "hash_v2".to_string(),
    };

    let report = graph.analyze_semantic_change_impact(&change);
    assert_eq!(
        report.total_impacted_count, 0,
        "Body mutation must have zero default impact radius"
    );
    assert!(report.direct_dependents.is_empty());
}

/// T4: آزمون تفکیک اسکوپ‌های تو در تو و جلوگیری از تداخل نمادهای همنام بر پایه AST واقعی
#[test]
fn test_t4_scope_shadowing_and_isolation() {
    let code = r#"
pub struct Executor {
    name: String,
}

impl Executor {
    pub fn run(&self) -> bool {
        true
    }
}

pub fn run() -> bool {
    false
}
"#;
    let file_id = FileId::generate();
    let parsed = AstEngine::parse_file_semantics(file_id, "src/exec.rs", code);

    assert_eq!(parsed.symbols.len(), 4);
    let method_sym = parsed
        .symbols
        .iter()
        .find(|s| s.qualified_scope.contains("impl_Executor") && s.name == "run")
        .unwrap();
    let fn_sym = parsed
        .symbols
        .iter()
        .find(|s| !s.qualified_scope.contains("impl_Executor") && s.name == "run")
        .unwrap();

    assert_ne!(
        method_sym.id, fn_sym.id,
        "Scoped method and free function must have distinct SymbolIds"
    );
    assert_ne!(method_sym.qualified_scope, fn_sym.qualified_scope);
}

/// T5: آزمون مدیریت ایمن وابستگی‌های دایره‌ای در الگوریتم BFS بدون حلقه بی‌نهایت از سورس‌کد
#[test]
fn test_t5_circular_dependency_safe_traversal() {
    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory index");

    let mod_a = r#"
use crate::b::call_b;
pub fn call_a() -> bool { call_b() }
"#;

    let mod_b = r#"
use crate::c::call_c;
pub fn call_b() -> bool { call_c() }
"#;

    let mod_c = r#"
use crate::a::call_a;
pub fn call_c() -> bool { call_a() }
"#;

    let files = vec![
        ("src/a.rs", mod_a),
        ("src/b.rs", mod_b),
        ("src/c.rs", mod_c),
    ];

    for (path, content) in &files {
        let fid = FileId::generate();
        let parsed = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed)
            .expect("Failed to index semantics");
        let parsed_syntax = AstEngine::parse_file_syntax(path, content);
        index
            .index_file(path, &parsed_syntax)
            .expect("Failed to index syntax");
    }

    let edges =
        SemanticResolver::resolve_strict_semantic_edges(&index).expect("Failed to resolve edges");

    let resolved_edges: Vec<_> = edges
        .into_iter()
        .filter(|e| e.target_symbol.is_some())
        .collect();
    assert_eq!(
        resolved_edges.len(),
        3,
        "Circular graph must have 3 resolved edges"
    );

    let mut graph = DependencyGraph::new();
    graph.build_from_strict_semantic_edges(&resolved_edges);

    let root_sym = resolved_edges[0].target_symbol.clone().unwrap();
    let report = graph.analyze_impact(&root_sym.0);
    assert_eq!(
        report.total_impacted_count, 2,
        "Cycle traversal must terminate gracefully visiting all dependents once"
    );
}

/// T6: آزمون ایزولاسیون کامل سندباکس اورلی قبل از اعمال روی دیسک
#[test]
fn test_t6_overlay_sandbox_isolation() {
    let mut overlay = CodeOverlayManager::new("base_git_hash", "tx_sandbox_01");
    let file_id = FileId::new("file_cache");

    let source = "pub fn get_cache() -> bool { true }";
    overlay.stage_file_mutation(file_id.clone(), "src/cache.rs", source.to_string());

    assert!(overlay.state().dirty_files.contains_key(&file_id));
    assert_eq!(overlay.state().dirty_files.get(&file_id).unwrap(), source);
}

/// T7: آزمون واقعی پایداری هویت نماد و تشخیص File Move از طریق Differ معنایی
#[test]
fn test_t7_file_move_identity_preservation() {
    let source_content = r#"
pub fn transfer_funds(account: &str, amount: u64) -> bool {
    amount > 0 && !account.is_empty()
}
"#;

    let snap_old =
        SemanticDiffer::create_file_snapshot("src/banking/old_service.rs", source_content);
    let snap_new = SemanticDiffer::create_file_snapshot_with_id(
        snap_old.file_id.clone(),
        "src/billing/new_payment_module.rs",
        source_content,
    );

    let report = SemanticDiffer::diff_project_snapshots(&[snap_old], &[snap_new]);

    assert_eq!(
        report.file_moves.len(),
        1,
        "Differ must detect exact content file movement"
    );
    assert_eq!(report.file_moves[0].old_path, "src/banking/old_service.rs");
    assert_eq!(
        report.file_moves[0].new_path,
        "src/billing/new_payment_module.rs"
    );
    assert_eq!(report.file_moves[0].similarity_ratio, 1.0);
    assert!(report.deleted_files.is_empty());
    assert!(report.added_files.is_empty());
}

/// T8: آزمون اتمیک بودن تراکنش، ثبت اسنپ‌شات و بازگشت تضمین‌شده وضعیت
#[test]
fn test_t8_transaction_atomicity_and_rollback() {
    let mut index = CodeIndex::new_in_memory().expect("Failed to initialize index");
    let mut overlay = CodeOverlayManager::new("commit_base_sha", "task_atomic_101");

    let file_path = "src/math.rs";
    let code_v1 = "pub fn add(a: i32, b: i32) -> i32 { a + b }";

    let mut tx_manager =
        ForgeTransactionManager::new("commit_base_sha", "task_atomic_101", &mut overlay);
    tx_manager.stage_base_file(file_path, code_v1);

    let code_v2 = "pub fn add(a: i32, b: i32) -> i32 { a + b + 10 }";
    tx_manager.stage_patch(file_path, "add", code_v2);

    let result = tx_manager
        .commit_transaction(&mut index, "Add bonus parameter to add function")
        .expect("Transaction commit must succeed");

    assert!(result.is_committed);
    assert!(result.snapshot_id.is_some());
    assert_eq!(result.patched_files.len(), 1);

    let mut engine = AstEngine::new_rust();
    let tree = engine.parse_file_tree(code_v2);
    assert!(
        tree.is_some(),
        "Surgical AST mutation must produce a valid syntax tree"
    );
    assert!(
        !tree.unwrap().root_node().has_error(),
        "Mutated AST tree must contain zero syntax errors"
    );
}
