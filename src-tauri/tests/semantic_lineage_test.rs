use rusqlite::Connection;
use vora_lib::forge::ast_engine::AstEngine;
use vora_lib::forge::differ::SemanticDiffer;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::lineage::LineageEngine;
use vora_lib::forge::model::{
    FileId, LineageId, LineageResolution, SemanticChange, SymbolIdentity, WorkspaceId,
};

fn prepare_test_symbols(
    path: &str,
    code: &str,
    fixed_lineage: Option<LineageId>,
) -> Vec<SymbolIdentity> {
    let fid = FileId::generate();
    let parsed = AstEngine::parse_file_semantics(fid, path, code);
    let lineage = fixed_lineage.unwrap_or_else(LineageId::generate_persistent);
    parsed
        .symbols
        .into_iter()
        .map(|mut s| {
            s.lineage_id = Some(lineage.clone());
            s
        })
        .collect()
}

/// بررسی شروط تغییرناپذیری تبار در پایگاه داده (Lineage Invariants Validator)
fn assert_lineage_invariants(conn: &Connection, workspace_id: &WorkspaceId) {
    let orphan_lineages: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM symbol_lineages sl LEFT JOIN workspaces w ON sl.workspace_id = w.workspace_id WHERE w.workspace_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        orphan_lineages, 0,
        "Lineage Invariant: Orphan symbol_lineages record found"
    );

    let orphan_versions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM symbol_lineage_versions slv LEFT JOIN symbol_lineages sl ON slv.lineage_id = sl.lineage_id WHERE sl.lineage_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        orphan_versions, 0,
        "Lineage Invariant: Dangling symbol_lineage_versions record found"
    );

    let invalid_ws_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM symbol_lineages WHERE workspace_id != ?1",
            [workspace_id.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        invalid_ws_count, 0,
        "Lineage Invariant: Workspace cross-contamination detected"
    );
}

#[test]
fn test_l01_lineage_preservation_on_body_mutation() {
    let code_v1 = "pub fn execute_calc(x: u32) -> u32 { x * 2 }";
    let code_v2 = "pub fn execute_calc(x: u32) -> u32 { x * 4 + 10 }";

    let fid = FileId::generate();
    let mut parsed_v1 = AstEngine::parse_file_semantics(fid.clone(), "src/calc.rs", code_v1);
    assert!(
        parsed_v1.symbols[0].lineage_id.is_none(),
        "AST must never assign lineage"
    );

    let lin_id = LineageId::generate_persistent();
    parsed_v1.symbols[0].lineage_id = Some(lin_id.clone());

    let parsed_v2 = AstEngine::parse_file_semantics(fid, "src/calc.rs", code_v2);
    let engine = LineageEngine::default();
    let resolutions = engine.resolve_lineages(&parsed_v1.symbols, &parsed_v2.symbols);

    assert_eq!(resolutions.len(), 1);
    match &resolutions[0] {
        LineageResolution::Matched {
            lineage_id,
            confidence,
            ..
        } => {
            assert_eq!(lineage_id, &lin_id);
            assert!(*confidence >= 0.85);
        }
        _ => panic!("Expected Matched lineage resolution"),
    }
}

#[test]
fn test_l02_lineage_tracking_on_signature_and_rename_mutation() {
    let code_v1 = "pub fn process_data(val: &str) -> bool { !val.is_empty() }";
    let code_v2 = "pub fn parse_and_process_data(val: &str) -> bool { !val.is_empty() }";

    let fid = FileId::generate();
    let mut parsed_v1 = AstEngine::parse_file_semantics(fid.clone(), "src/parser.rs", code_v1);
    let lin_id = LineageId::generate_persistent();
    parsed_v1.symbols[0].lineage_id = Some(lin_id.clone());

    let parsed_v2 = AstEngine::parse_file_semantics(fid, "src/parser.rs", code_v2);
    let engine = LineageEngine::default();
    let resolutions = engine.resolve_lineages(&parsed_v1.symbols, &parsed_v2.symbols);

    assert_eq!(resolutions.len(), 1);
    match &resolutions[0] {
        LineageResolution::Matched { lineage_id, .. } => {
            assert_eq!(lineage_id, &lin_id);
        }
        _ => panic!("Expected Matched lineage resolution on rename"),
    }
}

#[test]
fn test_l03_lineage_branching_on_new_symbol_addition() {
    let code_v1 = "pub fn original_method() -> i32 { 10 }";
    let code_v2 =
        "pub fn original_method() -> i32 { 10 }\npub fn new_isolated_helper() -> i32 { 20 }";

    let common_lineage = LineageId::generate_persistent();
    let syms_v1 = prepare_test_symbols("src/lib.rs", code_v1, Some(common_lineage.clone()));

    let fid = FileId::generate();
    let parsed_v2 = AstEngine::parse_file_semantics(fid, "src/lib.rs", code_v2);
    let syms_v2: Vec<SymbolIdentity> = parsed_v2
        .symbols
        .into_iter()
        .enumerate()
        .map(|(i, mut s)| {
            if i == 0 {
                s.lineage_id = Some(common_lineage.clone());
            }
            s
        })
        .collect();

    let engine = LineageEngine::default();
    let resolutions = engine.resolve_lineages(&syms_v1, &syms_v2);

    assert_eq!(resolutions.len(), 2);
    assert!(resolutions
        .iter()
        .any(|r| matches!(r, LineageResolution::Matched { .. })));
    assert!(resolutions
        .iter()
        .any(|r| matches!(r, LineageResolution::New { .. })));
}

#[test]
fn test_l04_duplicate_body_ambiguity_no_auto_bind() {
    let code_v1 = "pub fn worker_a() -> i32 { 42 }\npub fn worker_b() -> i32 { 42 }";
    let code_v2 = "pub fn worker_c() -> i32 { 42 }";

    let fid = FileId::generate();
    let parsed_v1 = AstEngine::parse_file_semantics(fid.clone(), "src/workers.rs", code_v1);
    let parsed_v2 = AstEngine::parse_file_semantics(fid, "src/workers.rs", code_v2);

    let engine = LineageEngine::default();
    let resolutions = engine.resolve_lineages(&parsed_v1.symbols, &parsed_v2.symbols);

    let matched_count = resolutions
        .iter()
        .filter(|r| matches!(r, LineageResolution::Matched { .. }))
        .count();
    assert_eq!(
        matched_count, 0,
        "Must never bind unpersisted candidates without historical lineage"
    );
}

#[test]
fn test_l05_move_and_rename_lineage_preservation() {
    let code_v1 = "pub fn calculate_score() -> u32 { 100 }";
    let code_v2 = "pub fn compute_final_score() -> u32 { 100 }";

    let common_lineage = LineageId::generate_persistent();
    let snap1 = SemanticDiffer::create_file_snapshot("src/old/game.rs", code_v1);
    let snap2 = SemanticDiffer::create_file_snapshot_with_id(
        snap1.file_id.clone(),
        "src/new/engine.rs",
        code_v2,
    );

    let mut snap1_with_lin = snap1;
    snap1_with_lin.symbols[0].lineage_id = Some(common_lineage.clone());

    let mut snap2_with_lin = snap2;
    snap2_with_lin.symbols[0].lineage_id = Some(common_lineage);

    let report = SemanticDiffer::diff_project_snapshots(&[snap1_with_lin], &[snap2_with_lin]);
    assert_eq!(report.file_moves.len(), 1);

    let rename_event = report
        .changes
        .iter()
        .find(|c| matches!(c, SemanticChange::SymbolRenamed { .. }));
    assert!(
        rename_event.is_some(),
        "Differ must output SymbolRenamed mutation backed by Lineage resolution"
    );
}

#[test]
fn test_l06_reorder_symbols_preserves_independent_lineage() {
    let code_v1 = "pub fn first() -> i32 { 1 }\npub fn second() -> i32 { 2 }";
    let code_v2 = "pub fn second() -> i32 { 2 }\npub fn first() -> i32 { 1 }";

    let lin_first = LineageId::generate_persistent();
    let lin_second = LineageId::generate_persistent();

    let fid1 = FileId::generate();
    let mut parsed_v1 = AstEngine::parse_file_semantics(fid1, "src/order.rs", code_v1);
    parsed_v1.symbols[0].lineage_id = Some(lin_first.clone());
    parsed_v1.symbols[1].lineage_id = Some(lin_second.clone());

    let fid2 = FileId::generate();
    let parsed_v2 = AstEngine::parse_file_semantics(fid2, "src/order.rs", code_v2);

    let engine = LineageEngine::default();
    let resolutions = engine.resolve_lineages(&parsed_v1.symbols, &parsed_v2.symbols);

    assert_eq!(resolutions.len(), 2);
    for res in resolutions {
        match res {
            LineageResolution::Matched {
                lineage_id,
                old_symbol_id,
                new_symbol_id,
                ..
            } => {
                let old_sym = parsed_v1
                    .symbols
                    .iter()
                    .find(|s| s.id == old_symbol_id)
                    .unwrap();
                let new_sym = parsed_v2
                    .symbols
                    .iter()
                    .find(|s| s.id == new_symbol_id)
                    .unwrap();
                assert_eq!(old_sym.name, new_sym.name);
                assert_eq!(lineage_id, old_sym.lineage_id.clone().unwrap());
            }
            _ => panic!("Reordered symbols must all match cleanly"),
        }
    }
}

#[test]
fn test_l07_multi_generation_history_persistence_in_db() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lineage_id = LineageId::generate_persistent();
    let file_path = "src/calc.rs";

    // نسل اول
    let code_1 = "pub fn calculate(x: i32) -> i32 { x }";
    let snap_1 = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, code_1.as_bytes())])
        .unwrap();

    index.index_file_incremental(file_path, code_1).unwrap();
    let fid_1 = index.get_or_create_file_id(file_path);
    let mut parsed_1 = AstEngine::parse_file_semantics(fid_1, file_path, code_1);
    parsed_1.symbols[0].lineage_id = Some(lineage_id.clone());
    index.index_file_semantics(&parsed_1).unwrap();
    let sym_1 = parsed_1.symbols[0].id.clone();
    index
        .record_lineage_version(&ws_id, &snap_1, &lineage_id, &sym_1, Some(1.0))
        .unwrap();

    // نسل دوم
    let code_2 = "pub fn compute(x: i32) -> i32 { x * 2 }";
    let snap_2 = index
        .commit_identity_snapshot(&ws_id, Some(&snap_1), &[(file_path, code_2.as_bytes())])
        .unwrap();

    index.index_file_incremental(file_path, code_2).unwrap();
    let fid_2 = index.get_or_create_file_id(file_path);
    let mut parsed_2 = AstEngine::parse_file_semantics(fid_2, file_path, code_2);
    parsed_2.symbols[0].lineage_id = Some(lineage_id.clone());
    index.index_file_semantics(&parsed_2).unwrap();
    let sym_2 = parsed_2.symbols[0].id.clone();
    index
        .record_lineage_version(&ws_id, &snap_2, &lineage_id, &sym_2, Some(0.95))
        .unwrap();

    // نسل سوم
    let code_3 = "pub fn compute_v3(x: i64) -> i64 { x * 4 }";
    let snap_3 = index
        .commit_identity_snapshot(&ws_id, Some(&snap_2), &[(file_path, code_3.as_bytes())])
        .unwrap();

    index.index_file_incremental(file_path, code_3).unwrap();
    let fid_3 = index.get_or_create_file_id(file_path);
    let mut parsed_3 = AstEngine::parse_file_semantics(fid_3, file_path, code_3);
    parsed_3.symbols[0].lineage_id = Some(lineage_id.clone());
    index.index_file_semantics(&parsed_3).unwrap();
    let sym_3 = parsed_3.symbols[0].id.clone();
    index
        .record_lineage_version(&ws_id, &snap_3, &lineage_id, &sym_3, Some(0.90))
        .unwrap();

    let history = index.get_lineage_history(&lineage_id).unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].0, snap_1);
    assert_eq!(history[0].1, sym_1);
    assert_eq!(history[1].0, snap_2);
    assert_eq!(history[1].1, sym_2);
    assert_eq!(history[2].0, snap_3);
    assert_eq!(history[2].1, sym_3);

    assert_lineage_invariants(index.connection(), &ws_id);
}

#[test]
fn test_l08_move_rename_and_signature_mutation() {
    let code_v1 = "pub fn transform(val: i32) -> i32 { val + 1 }";
    let code_v2 =
        "pub fn apply_transformation(val: i32, multiplier: i32) -> i32 { val * multiplier }";

    let common_lineage = LineageId::generate_persistent();
    let snap1 = SemanticDiffer::create_file_snapshot("src/legacy/ops.rs", code_v1);
    let snap2 = SemanticDiffer::create_file_snapshot_with_id(
        snap1.file_id.clone(),
        "src/modern/ops_v2.rs",
        code_v2,
    );

    let mut snap1_with_lin = snap1;
    snap1_with_lin.symbols[0].lineage_id = Some(common_lineage.clone());

    let mut snap2_with_lin = snap2;
    snap2_with_lin.symbols[0].lineage_id = Some(common_lineage);

    let report = SemanticDiffer::diff_project_snapshots(&[snap1_with_lin], &[snap2_with_lin]);
    assert_eq!(report.file_moves.len(), 1, "Must detect move");
    assert!(!report.changes.is_empty(), "Must detect semantic mutations");
}

#[test]
fn test_l09_delete_and_replacement_isolation() {
    let code_v1 = "pub fn old_service() -> bool { true }";
    let code_v2 = "pub fn entirely_different_worker(val: f64) -> String { val.to_string() }";

    let fid1 = FileId::generate();
    let mut parsed_v1 = AstEngine::parse_file_semantics(fid1, "src/service.rs", code_v1);
    parsed_v1.symbols[0].lineage_id = Some(LineageId::generate_persistent());

    let fid2 = FileId::generate();
    let parsed_v2 = AstEngine::parse_file_semantics(fid2, "src/service.rs", code_v2);

    let engine = LineageEngine::default();
    let resolutions = engine.resolve_lineages(&parsed_v1.symbols, &parsed_v2.symbols);

    assert!(
        resolutions
            .iter()
            .any(|r| matches!(r, LineageResolution::Removed { .. })),
        "Must mark old symbol as Removed"
    );
    assert!(
        resolutions
            .iter()
            .any(|r| matches!(r, LineageResolution::New { .. })),
        "Must mark replacement as New lineage"
    );
}

#[test]
fn test_l10_cloned_file_allocates_isolated_lineage() {
    let code = "pub fn common_utility() -> &'static str { \"ok\" }";

    let fid_orig = FileId::generate();
    let mut parsed_orig_v1 = AstEngine::parse_file_semantics(fid_orig.clone(), "src/util.rs", code);
    let lin_orig = LineageId::generate_persistent();
    parsed_orig_v1.symbols[0].lineage_id = Some(lin_orig.clone());

    let parsed_orig_v2 = AstEngine::parse_file_semantics(fid_orig, "src/util.rs", code);
    let fid_clone = FileId::generate();
    let parsed_clone = AstEngine::parse_file_semantics(fid_clone, "src/util_copy.rs", code);

    let orig_symbol_id = parsed_orig_v2.symbols[0].id.clone();
    let clone_symbol_id = parsed_clone.symbols[0].id.clone();

    let mut target_syms = parsed_orig_v2.symbols;
    target_syms.extend(parsed_clone.symbols);

    std::thread::sleep(std::time::Duration::from_millis(5));

    let engine = LineageEngine::default();
    let resolutions = engine.resolve_lineages(&parsed_orig_v1.symbols, &target_syms);

    let mut matched_lineage: Option<LineageId> = None;
    let mut new_lineage: Option<LineageId> = None;

    for r in &resolutions {
        match r {
            LineageResolution::Matched {
                lineage_id,
                new_symbol_id,
                ..
            } if new_symbol_id == &orig_symbol_id => {
                matched_lineage = Some(lineage_id.clone());
            }
            LineageResolution::New {
                lineage_id,
                new_symbol_id,
            } if new_symbol_id == &clone_symbol_id => {
                new_lineage = Some(lineage_id.clone());
            }
            _ => {}
        }
    }

    assert_eq!(
        matched_lineage.as_ref(),
        Some(&lin_orig),
        "Original file symbol must preserve its lineage"
    );
    assert!(
        new_lineage.is_some(),
        "Cloned file symbol must receive a new lineage"
    );
    assert_ne!(
        new_lineage.as_ref(),
        Some(&lin_orig),
        "Cloned symbol lineage must be distinct from original"
    );
}

#[test]
fn test_l11_false_merge_death_test() {
    let code_v1 = "pub fn proc_a() -> i32 { 100 }\npub fn proc_b() -> i32 { 100 }";
    let code_v2 = "pub fn proc_unified() -> i32 { 100 }";

    let fid = FileId::generate();
    let parsed_v1 = AstEngine::parse_file_semantics(fid.clone(), "src/merge_test.rs", code_v1);
    let parsed_v2 = AstEngine::parse_file_semantics(fid, "src/merge_test.rs", code_v2);

    let engine = LineageEngine::default();
    let resolutions = engine.resolve_lineages(&parsed_v1.symbols, &parsed_v2.symbols);

    let matched_count = resolutions
        .iter()
        .filter(|r| matches!(r, LineageResolution::Matched { .. }))
        .count();
    assert_eq!(
        matched_count, 0,
        "Death Test: Two distinct symbols with identical body must NEVER false-merge into one"
    );
}

#[test]
fn test_l12_real_pipeline_lineage() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/workspace_root").unwrap();
    let engine = LineageEngine::default();

    // اسنپ‌شات اول: تابع foo در src/a.rs
    let path_s1 = "src/a.rs";
    let code_s1 = "pub fn foo(val: i32) -> i32 { val }";
    let snap_1 = index
        .commit_identity_snapshot(&ws_id, None, &[(path_s1, code_s1.as_bytes())])
        .unwrap();

    let fid_s1 = index.get_or_create_file_id(path_s1);
    let parsed_s1 = AstEngine::parse_file_semantics(fid_s1, path_s1, code_s1);

    // اولین بار: تولید خودکار تبار توسط رزولور
    let res_s1 = engine.resolve_lineages(&[], &parsed_s1.symbols);
    assert_eq!(res_s1.len(), 1);
    let l1_id = match &res_s1[0] {
        LineageResolution::New { lineage_id, .. } => lineage_id.clone(),
        _ => panic!("First snapshot must generate a new lineage"),
    };

    let mut sym_s1 = parsed_s1.symbols;
    sym_s1[0].lineage_id = Some(l1_id.clone());
    index
        .record_lineage_version(&ws_id, &snap_1, &l1_id, &sym_s1[0].id, Some(1.0))
        .unwrap();

    // اسنپ‌شات دوم: تغییر امضا و بدنه به foo(val: i64) در همان فایل (Same File + Same Name)
    let code_s2 = "pub fn foo(val: i64) -> i64 { val * 2 }";
    let snap_2 = index
        .commit_identity_snapshot(&ws_id, Some(&snap_1), &[(path_s1, code_s2.as_bytes())])
        .unwrap();

    let fid_s2 = index.get_or_create_file_id(path_s1);
    let parsed_s2 = AstEngine::parse_file_semantics(fid_s2, path_s1, code_s2);

    // حل تبار خودکار بین اسنپ‌شات ۱ و اسنپ‌شات ۲
    let res_s2 = engine.resolve_lineages(&sym_s1, &parsed_s2.symbols);
    assert_eq!(res_s2.len(), 1);
    let matched_l1_s2 = match &res_s2[0] {
        LineageResolution::Matched {
            lineage_id,
            confidence,
            ..
        } => {
            assert!(*confidence >= 0.50);
            lineage_id.clone()
        }
        _ => panic!("S2 must match to S1 lineage"),
    };
    assert_eq!(matched_l1_s2, l1_id);

    let mut sym_s2 = parsed_s2.symbols;
    sym_s2[0].lineage_id = Some(matched_l1_s2.clone());
    index
        .record_lineage_version(&ws_id, &snap_2, &matched_l1_s2, &sym_s2[0].id, Some(0.90))
        .unwrap();

    // اسنپ‌شات سوم: جابه‌جایی فایل به src/modules/b.rs با حفظ نام تابع foo(val: i64)
    let path_s3 = "src/modules/b.rs";
    let code_s3 = "pub fn foo(val: i64) -> i64 { val * 2 }";
    let snap_3 = index
        .commit_identity_snapshot(&ws_id, Some(&snap_2), &[(path_s3, code_s3.as_bytes())])
        .unwrap();

    let fid_s3 = index.get_or_create_file_id(path_s3);
    let parsed_s3 = AstEngine::parse_file_semantics(fid_s3, path_s3, code_s3);

    // حل تبار خودکار بین اسنپ‌شات ۲ و اسنپ‌شات ۳ (Move across files with matching signature/body)
    let res_s3 = engine.resolve_lineages(&sym_s2, &parsed_s3.symbols);
    assert_eq!(res_s3.len(), 1);
    let matched_l1_s3 = match &res_s3[0] {
        LineageResolution::Matched {
            lineage_id,
            confidence,
            ..
        } => {
            assert!(*confidence >= 0.50);
            lineage_id.clone()
        }
        _ => panic!("S3 must match to S2 lineage across move"),
    };
    assert_eq!(matched_l1_s3, l1_id);

    let sym_s3 = parsed_s3.symbols;
    index
        .record_lineage_version(&ws_id, &snap_3, &matched_l1_s3, &sym_s3[0].id, Some(0.95))
        .unwrap();

    // کوئری تاریخچه تبار بدون دستکاری دستی
    let history = index.get_lineage_history(&l1_id).unwrap();
    assert_eq!(
        history.len(),
        3,
        "Real lineage pipeline must retain exactly 3 historical versions"
    );
    assert_eq!(history[0].0, snap_1);
    assert_eq!(history[0].1, sym_s1[0].id);
    assert_eq!(history[1].0, snap_2);
    assert_eq!(history[1].1, sym_s2[0].id);
    assert_eq!(history[2].0, snap_3);
    assert_eq!(history[2].1, sym_s3[0].id);

    assert_lineage_invariants(index.connection(), &ws_id);
}
