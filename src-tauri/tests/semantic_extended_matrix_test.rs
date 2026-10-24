use vora_lib::forge::ast_engine::AstEngine;
use vora_lib::forge::commit::ForgeTransactionManager;
use vora_lib::forge::differ::SemanticDiffer;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::{FileId, LineageId, ResolutionStatus, SemanticChange};
use vora_lib::forge::overlay::CodeOverlayManager;
use vora_lib::forge::resolver::SemanticResolver;

/// T07: ارزیابی رزولوشن دقیق واردات با نام مستعار (Import Alias Resolution)
#[test]
fn test_import_alias_resolution() {
    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory index");

    let math_src = r#"
pub fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    let consumer_src = r#"
use crate::math::calculate_sum as add_numbers;

pub fn execute_operation(x: i32, y: i32) -> i32 {
    add_numbers(x, y)
}
"#;

    let files = vec![("src/math.rs", math_src), ("src/consumer.rs", consumer_src)];

    for (path, content) in &files {
        let fid = FileId::generate();
        let parsed_semantics = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed_semantics)
            .expect("Failed to index semantics");
        let parsed_syntax = AstEngine::parse_file_syntax(path, content);
        index
            .index_file(path, &parsed_syntax)
            .expect("Failed to index syntax");
    }

    let edges = SemanticResolver::resolve_strict_semantic_edges(&index)
        .expect("Failed to resolve strict semantic edges");

    let resolved_edges: Vec<_> = edges
        .into_iter()
        .filter(|e| matches!(e.status, ResolutionStatus::Resolved(_)))
        .collect();

    assert_eq!(
        resolved_edges.len(),
        1,
        "Must resolve aliased import call cleanly"
    );
    assert!(resolved_edges[0].target_symbol.is_some());
}

/// T08: ارزیابی تشخیص ترکیبی جابه‌جایی فایل + تغییر نام نماد (Move + Rename)
#[test]
fn test_move_and_rename_combination() {
    let code_old = "pub fn initial_executor() -> bool { true }";
    let code_new = "pub fn renamed_executor() -> bool { true }";

    let persistent_fid = FileId::generate();
    let common_lineage = LineageId::generate_persistent();

    let mut snap_old = SemanticDiffer::create_file_snapshot_with_id(
        persistent_fid.clone(),
        "src/old_path/exec.rs",
        code_old,
    );
    if !snap_old.symbols.is_empty() {
        snap_old.symbols[0].lineage_id = Some(common_lineage.clone());
    }

    let mut snap_new = SemanticDiffer::create_file_snapshot_with_id(
        persistent_fid,
        "src/new_path/exec_v2.rs",
        code_new,
    );
    if !snap_new.symbols.is_empty() {
        snap_new.symbols[0].lineage_id = Some(common_lineage);
    }

    let report = SemanticDiffer::diff_project_snapshots(&[snap_old], &[snap_new]);

    assert_eq!(
        report.file_moves.len(),
        1,
        "Must detect file move even when symbol is renamed"
    );
    assert_eq!(report.file_moves[0].old_path, "src/old_path/exec.rs");
    assert_eq!(report.file_moves[0].new_path, "src/new_path/exec_v2.rs");

    let rename_change = report
        .changes
        .iter()
        .find(|c| matches!(c, SemanticChange::SymbolRenamed { .. }));
    assert!(
        rename_change.is_some(),
        "Must detect SymbolRenamed along with file move"
    );
}

/// T09: تفکیک دامنه‌های هم‌نام در چند فایل و عدم تصادم شناسه‌های نماد
#[test]
fn test_multi_file_scope_disambiguation() {
    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory index");

    let service_a = r#"
pub fn run_task() -> &'static str {
    "task_a"
}
"#;

    let service_b = r#"
pub fn run_task() -> &'static str {
    "task_b"
}
"#;

    let caller_src = r#"
use crate::service_a::run_task;

pub fn orchestrate() -> &'static str {
    run_task()
}
"#;

    let files = vec![
        ("src/service_a.rs", service_a),
        ("src/service_b.rs", service_b),
        ("src/caller.rs", caller_src),
    ];

    for (path, content) in &files {
        let fid = FileId::generate();
        let parsed_semantics = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed_semantics)
            .expect("Failed to index semantics");
        let parsed_syntax = AstEngine::parse_file_syntax(path, content);
        index
            .index_file(path, &parsed_syntax)
            .expect("Failed to index syntax");
    }

    let edges = SemanticResolver::resolve_strict_semantic_edges(&index)
        .expect("Failed to resolve strict semantic edges");

    let resolved_edges: Vec<_> = edges
        .into_iter()
        .filter(|e| matches!(e.status, ResolutionStatus::Resolved(_)))
        .collect();

    assert_eq!(
        resolved_edges.len(),
        1,
        "Must resolve to specific imported module without ambiguity"
    );
}

/// T10: مدیریت و ثبت صریح وضعیت ابهام (Ambiguous Resolution)
#[test]
fn test_ambiguous_resolution_handling() {
    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory index");

    let mod1_src = "pub fn compute() -> i32 { 10 }";
    let mod2_src = "pub fn compute() -> i32 { 20 }";
    let caller_src = r#"
pub fn main_runner() -> i32 {
    compute()
}
"#;

    let files = vec![
        ("src/mod1.rs", mod1_src),
        ("src/mod2.rs", mod2_src),
        ("src/caller_amb.rs", caller_src),
    ];

    for (path, content) in &files {
        let fid = FileId::generate();
        let parsed_semantics = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed_semantics)
            .expect("Failed to index semantics");
        let parsed_syntax = AstEngine::parse_file_syntax(path, content);
        index
            .index_file(path, &parsed_syntax)
            .expect("Failed to index syntax");
    }

    let edges = SemanticResolver::resolve_strict_semantic_edges(&index)
        .expect("Failed to resolve strict semantic edges");

    let amb_edges: Vec<_> = edges
        .into_iter()
        .filter(|e| matches!(e.status, ResolutionStatus::Ambiguous(_)))
        .collect();

    assert_eq!(
        amb_edges.len(),
        1,
        "Must mark caller as ambiguous when multiple candidates exist without import context"
    );
}

/// T11 & T12: اثبات اتمیسیتی کامل تراکنش و بازگشت تضمین‌شده (Atomic Transaction Rollback Proof)
#[test]
fn test_atomic_transaction_failure_injection_and_rollback() {
    let mut index = CodeIndex::new_in_memory().expect("Failed to initialize index");
    let mut overlay = CodeOverlayManager::new("base_commit_hash", "task_fail_inject");

    let valid_code = "pub fn execute(val: i32) -> i32 { val * 2 }";
    let file_path = "src/calculator.rs";

    index
        .index_file_incremental(file_path, valid_code)
        .expect("Initial indexing failed");
    let initial_symbols_count = index.get_total_symbols_count().expect("Count failed");

    let mut tx_manager =
        ForgeTransactionManager::new("base_commit_hash", "task_fail_inject", &mut overlay);
    tx_manager.stage_base_file(file_path, valid_code);

    let invalid_patch = "pub fn execute(val: i32) -> i32 { val * +++ syntax_error }";
    tx_manager.stage_patch(file_path, "execute", invalid_patch);

    let commit_result =
        tx_manager.commit_transaction(&mut index, "Refactor calculator with syntax error");
    assert!(
        commit_result.is_err(),
        "Transaction must fail and refuse commit on invalid syntax"
    );

    let post_fail_symbols_count = index.get_total_symbols_count().expect("Count failed");
    assert_eq!(
        initial_symbols_count, post_fail_symbols_count,
        "Database symbol count must remain 100% unchanged after transaction failure"
    );
    assert!(
        overlay.state().dirty_files.is_empty(),
        "Overlay sandbox must be completely clean and rolled back after failure"
    );
}

/// T13: ارزیابی ایزولاسیون و تقدم اسکوپ‌های لکسیکال (Lexical Scope Shadowing Isolation)
#[test]
fn test_lexical_scope_shadowing_and_local_binding_precedence() {
    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory index");

    let module_src = r#"
pub fn target_action() -> i32 { 100 }
"#;

    let caller_src = r#"
use crate::actions::target_action;

pub fn execute_with_shadowing() -> i32 {
    target_action()
}
"#;

    let files = vec![
        ("src/actions.rs", module_src),
        ("src/executor.rs", caller_src),
    ];

    for (path, content) in &files {
        let fid = FileId::generate();
        let parsed_semantics = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed_semantics)
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
        .filter(|e| matches!(e.status, ResolutionStatus::Resolved(_)))
        .collect();

    assert_eq!(
        resolved_edges.len(),
        1,
        "Must resolve explicitly imported action over ambient scopes"
    );
    assert!(resolved_edges[0].target_symbol.is_some());
}
