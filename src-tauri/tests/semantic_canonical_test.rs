use vora_lib::forge::adapter::{RustSemanticAdapter, SemanticAdapter, TypeScriptSemanticAdapter};
use vora_lib::forge::ast_engine::AstEngine;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::{CanonicalEntityId, LineageId, SnapshotId, WorkspaceId};

#[test]
fn test_c01_canonical_entity_id_deterministic_blake3() {
    let ws = WorkspaceId::new("ws_core_01");
    let lin = LineageId::new("lin_calc_88");

    let id1 = CanonicalEntityId::from_lineage(&ws, &lin);
    let id2 = CanonicalEntityId::from_lineage(&ws, &lin);

    assert_eq!(id1, id2);
    assert!(id1.as_str().starts_with("ent_"));
    assert_eq!(id1.as_str().len(), 4 + 64);
}

#[test]
fn test_c02_cross_language_semantic_signature_equivalence() {
    let rust_code = "pub async fn calculate_sum(a: f64, items: Vec<String>) -> f64 { a }";
    let ts_code = "async function calculate_sum(a: number, items: string[]): number { return a; }";

    let mut rust_engine = AstEngine::new_rust();
    let rust_tree = rust_engine.parse_file_tree(rust_code).unwrap();
    let rust_adapter = RustSemanticAdapter;
    let rust_fn_node = rust_tree.root_node().child(0).unwrap();
    let rust_sig = rust_adapter.parse_structured_signature(&rust_fn_node, rust_code.as_bytes());

    let mut ts_engine = AstEngine::new_typescript();
    let ts_tree = ts_engine.parse_file_tree(ts_code).unwrap();
    let ts_adapter = TypeScriptSemanticAdapter;
    let ts_fn_node = ts_tree.root_node().child(0).unwrap();
    let ts_sig = ts_adapter.parse_structured_signature(&ts_fn_node, ts_code.as_bytes());

    assert_eq!(rust_sig.is_async, ts_sig.is_async);
    assert_eq!(rust_sig.return_type, ts_sig.return_type);
    assert_eq!(rust_sig.parameters.len(), ts_sig.parameters.len());
    assert_eq!(
        rust_sig.parameters[0].param_type,
        ts_sig.parameters[0].param_type
    );
    assert_eq!(
        rust_sig.parameters[1].param_type,
        ts_sig.parameters[1].param_type
    );
}

#[test]
fn test_c03_atomic_canonical_registration_and_history() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lineage_id = LineageId::generate_persistent();
    let file_path = "src/math.rs";

    // ۱. اسنپ‌شات اول
    let snap_1 = index
        .commit_identity_snapshot(
            &ws_id,
            None,
            &[(file_path, b"pub fn add(x: i32) -> i32 { x }")],
        )
        .unwrap();
    let fid = index.get_or_create_file_id(file_path);
    let parsed_1 =
        AstEngine::parse_file_semantics(fid.clone(), file_path, "pub fn add(x: i32) -> i32 { x }");
    let sym_1 = parsed_1.symbols[0].id.clone();

    let entity_id = index
        .register_canonical_entity(
            &ws_id,
            &snap_1,
            &lineage_id,
            &sym_1,
            "add",
            "crate::math",
            "pub fn add(x: i32) -> i32",
        )
        .unwrap();

    // ۲. اسنپ‌شات دوم (تکامل امضا و نام با حفظ Lineage)
    let snap_2 = index
        .commit_identity_snapshot(
            &ws_id,
            Some(&snap_1),
            &[(file_path, b"pub fn add_v2(x: i64) -> i64 { x }")],
        )
        .unwrap();
    let parsed_2 =
        AstEngine::parse_file_semantics(fid, file_path, "pub fn add_v2(x: i64) -> i64 { x }");
    let sym_2 = parsed_2.symbols[0].id.clone();

    let entity_id_2 = index
        .register_canonical_entity(
            &ws_id,
            &snap_2,
            &lineage_id,
            &sym_2,
            "add_v2",
            "crate::math",
            "pub fn add_v2(x: i64) -> i64",
        )
        .unwrap();

    assert_eq!(
        entity_id, entity_id_2,
        "CanonicalEntityId must remain perfectly stable across mutations"
    );

    // ۳. بازخوانی تاریخچه تکامل انتیتی
    let history = index.get_canonical_entity_history(&entity_id).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].snapshot_id, snap_1);
    assert_eq!(history[0].canonical_name, "add");
    assert_eq!(history[1].snapshot_id, snap_2);
    assert_eq!(history[1].canonical_name, "add_v2");
}

#[test]
fn test_c04_foreign_key_protection_on_unregistered_snapshot() {
    let index = CodeIndex::new_in_memory().unwrap();
    let ws_id = WorkspaceId::new("ws_test");
    let lineage_id = LineageId::new("lin_test");
    let fake_snapshot = SnapshotId::new("snap_does_not_exist");
    let fake_sym = vora_lib::forge::model::SymbolId::new("sym_1");

    // ثبت روی اسنپ‌شات ناموجود باید به دلیل Foreign Key با خطا مواجه شود
    let result = index.register_canonical_entity(
        &ws_id,
        &fake_snapshot,
        &lineage_id,
        &fake_sym,
        "calculate",
        "crate::calc",
        "fn calculate()",
    );

    assert!(
        result.is_err(),
        "Must strictly reject registration against non-existent snapshot"
    );
}
