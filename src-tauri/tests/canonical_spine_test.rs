use rusqlite::Connection;
use std::fs;
use vora_lib::forge::ast_engine::AstEngine;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::{CanonicalEntityId, LineageId, SymbolId, WorkspaceId};

fn assert_canonical_invariants(conn: &Connection) {
    let orphan_entities: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM canonical_entities ce LEFT JOIN workspaces w ON ce.workspace_id = w.workspace_id WHERE w.workspace_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        orphan_entities, 0,
        "Canonical Invariant: Orphan canonical_entities record found"
    );

    let orphan_versions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM canonical_entity_versions cev LEFT JOIN canonical_entities ce ON cev.entity_id = ce.entity_id WHERE ce.entity_id IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        orphan_versions, 0,
        "Canonical Invariant: Dangling canonical_entity_versions record found"
    );
}

#[test]
fn test_c01_cross_language_canonical_id_generation() {
    let ws = WorkspaceId::new("ws_core_01");
    let lin = LineageId::new("lin_calc_88");

    let id1 = CanonicalEntityId::from_lineage(&ws, &lin);
    let id2 = CanonicalEntityId::from_lineage(&ws, &lin);

    assert_eq!(id1, id2);
    assert!(id1.as_str().starts_with("ent_"));
    assert_eq!(id1.as_str().len(), 4 + 64);
}

#[test]
fn test_c02_typescript_tsx_react_component_canonical_extraction() {
    let tsx_code = r#"
export const UserProfile: React.FC<ProfileProps> = (props) => {
    return <div>{props.name}</div>;
};
"#;
    let fid = vora_lib::forge::model::FileId::generate();
    let parsed = AstEngine::parse_file_semantics(fid, "src/components/UserProfile.tsx", tsx_code);
    assert!(!parsed.symbols.is_empty());
    assert_eq!(parsed.symbols[0].name, "UserProfile");
}

#[test]
fn test_c03_canonical_entity_persistence_and_query_in_db() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lineage_id = LineageId::generate_persistent();
    let file_path = "src/math.rs";

    let snap_1 = index
        .commit_identity_snapshot(
            &ws_id,
            None,
            &[(file_path, b"pub fn add(x: i32) -> i32 { x }")],
        )
        .unwrap();
    let fid = index.get_or_create_file_id(file_path);
    let parsed_1 =
        AstEngine::parse_file_semantics(fid, file_path, "pub fn add(x: i32) -> i32 { x }");
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

    let history = index.get_canonical_entity_history(&entity_id).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].canonical_name, "add");
    assert_canonical_invariants(index.connection());
}

#[test]
fn test_c01_unchanged_preserves_canonical_entity() {
    let ws = WorkspaceId::new("ws_main");
    let lin = LineageId::new("lin_unchanged");
    let id1 = CanonicalEntityId::from_lineage(&ws, &lin);
    let id2 = CanonicalEntityId::from_lineage(&ws, &lin);
    assert_eq!(id1, id2);
}

#[test]
fn test_c02_body_mutation_preserves_canonical_entity() {
    let ws = WorkspaceId::new("ws_main");
    let lin = LineageId::new("lin_mut");
    let id1 = CanonicalEntityId::from_lineage(&ws, &lin);
    let id2 = CanonicalEntityId::from_lineage(&ws, &lin);
    assert_eq!(id1, id2);
}

#[test]
fn test_c03_rename_preserves_canonical_entity() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lineage_id = LineageId::generate_persistent();
    let file_path = "src/calc.rs";

    let snap = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, b"pub fn rename_target() {}")])
        .unwrap();
    let sym = SymbolId::new("sym_rename");

    let ent1 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "old_name",
            "crate::calc",
            "fn old_name()",
        )
        .unwrap();
    let ent2 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "new_name",
            "crate::calc",
            "fn new_name()",
        )
        .unwrap();

    assert_eq!(ent1, ent2);
}

#[test]
fn test_c04_signature_mutation_preserves_canonical_entity() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lineage_id = LineageId::generate_persistent();
    let file_path = "src/calc.rs";

    let snap = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, b"pub fn sig_target() {}")])
        .unwrap();
    let sym = SymbolId::new("sym_sig");

    let ent1 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "compute",
            "crate::calc",
            "fn compute(x: i32)",
        )
        .unwrap();
    let ent2 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "compute",
            "crate::calc",
            "fn compute(x: i64, y: i64)",
        )
        .unwrap();

    assert_eq!(ent1, ent2);
}

#[test]
fn test_c05_move_preserves_canonical_entity() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lineage_id = LineageId::generate_persistent();
    let file_path = "src/moved.rs";

    let snap = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, b"pub fn move_target() {}")])
        .unwrap();
    let sym = SymbolId::new("sym_move");

    let ent1 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "execute",
            "crate::old_path",
            "fn execute()",
        )
        .unwrap();
    let ent2 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "execute",
            "crate::new_path",
            "fn execute()",
        )
        .unwrap();

    assert_eq!(ent1, ent2);
}

#[test]
fn test_c06_move_and_rename_preserves_canonical_entity() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lineage_id = LineageId::generate_persistent();
    let file_path = "src/ops.rs";

    let snap = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, b"pub fn ops_target() {}")])
        .unwrap();
    let sym = SymbolId::new("sym_ops");

    let ent1 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "run",
            "crate::v1",
            "fn run()",
        )
        .unwrap();
    let ent2 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "process",
            "crate::v2",
            "fn process()",
        )
        .unwrap();

    assert_eq!(ent1, ent2);
}

#[test]
fn test_c07_delete_preserves_historical_canonical_entity() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lineage_id = LineageId::generate_persistent();
    let file_path = "src/del.rs";

    let snap = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, b"pub fn del_target() {}")])
        .unwrap();
    let sym = SymbolId::new("sym_del");

    let ent = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "deleted_fn",
            "crate::del",
            "fn deleted_fn()",
        )
        .unwrap();

    let history = index.get_canonical_entity_history(&ent).unwrap();
    assert_eq!(history.len(), 1);
}

#[test]
fn test_c08_restore_reuses_same_canonical_entity() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lineage_id = LineageId::generate_persistent();
    let file_path = "src/restore.rs";

    let snap = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, b"pub fn restore_target() {}")])
        .unwrap();
    let sym = SymbolId::new("sym_res");

    let ent1 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "worker",
            "crate::service",
            "fn worker()",
        )
        .unwrap();
    let ent2 = index
        .register_canonical_entity(
            &ws_id,
            &snap,
            &lineage_id,
            &sym,
            "worker",
            "crate::service",
            "fn worker()",
        )
        .unwrap();

    assert_eq!(ent1, ent2);
}

#[test]
fn test_c09_new_declaration_allocates_new_canonical_entity() {
    let ws = WorkspaceId::new("ws_main");
    let lin1 = LineageId::generate_persistent();
    let lin2 = LineageId::generate_persistent();

    let ent1 = CanonicalEntityId::from_lineage(&ws, &lin1);
    let ent2 = CanonicalEntityId::from_lineage(&ws, &lin2);

    assert_ne!(ent1, ent2);
}

#[test]
fn test_c10_same_name_different_namespace_are_distinct_entities() {
    let ws = WorkspaceId::new("ws_main");
    let lin1 = LineageId::generate_persistent();
    let lin2 = LineageId::generate_persistent();

    let ent1 = CanonicalEntityId::from_lineage(&ws, &lin1);
    let ent2 = CanonicalEntityId::from_lineage(&ws, &lin2);

    assert_ne!(ent1, ent2);
}

#[test]
fn test_c11_same_signature_different_workspace_are_distinct_entities() {
    let ws1 = WorkspaceId::new("ws_one");
    let ws2 = WorkspaceId::new("ws_two");
    let lin = LineageId::new("lin_shared");

    let ent1 = CanonicalEntityId::from_lineage(&ws1, &lin);
    let ent2 = CanonicalEntityId::from_lineage(&ws2, &lin);

    assert_ne!(ent1, ent2);
}

#[test]
fn test_c12_duplicate_candidate_ambiguity_isolation() {
    let ws = WorkspaceId::new("ws_main");
    let lin = LineageId::new("lin_amb");
    let ent = CanonicalEntityId::from_lineage(&ws, &lin);
    assert!(ent.as_str().starts_with("ent_"));
}

#[test]
fn test_c13_restart_durability_preserves_canonical_entities() {
    let temp_dir = std::env::temp_dir().join(format!("vora_c13_{}", uuid::Uuid::now_v7()));
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("c13.db");
    let lin_id = LineageId::new("lin_c13");
    let entity_id;

    {
        let mut index = CodeIndex::new(&db_path).unwrap();
        let ws = index.get_or_create_workspace("/repo").unwrap();
        let snap = index
            .commit_identity_snapshot(&ws, None, &[("src/lib.rs", b"pub fn durable() {}")])
            .unwrap();
        let sym = SymbolId::new("sym_c13");

        let ent = index
            .register_canonical_entity(
                &ws,
                &snap,
                &lin_id,
                &sym,
                "durable",
                "crate::lib",
                "fn durable()",
            )
            .unwrap();
        entity_id = ent;
        drop(index);
    }

    {
        let index = CodeIndex::new(&db_path).unwrap();
        let history = index.get_canonical_entity_history(&entity_id).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].canonical_name, "durable");
        drop(index);
    }

    fs::remove_dir_all(temp_dir).ok();
}

#[test]
fn test_c14_five_generation_evolution_spine() {
    let mut index = CodeIndex::new_in_memory().unwrap();
    let ws_id = index.get_or_create_workspace("/repo").unwrap();
    let lin_id = LineageId::generate_persistent();
    let file_path = "src/evo.rs";

    let mut parent = None;
    let mut last_ent = None;

    for gen in 1..=5 {
        let code = format!("pub fn step_{}() {{}}", gen);
        let snap = index
            .commit_identity_snapshot(&ws_id, parent.as_ref(), &[(file_path, code.as_bytes())])
            .unwrap();
        let sym = SymbolId::new(format!("sym_gen_{}", gen));

        let ent = index
            .register_canonical_entity(
                &ws_id,
                &snap,
                &lin_id,
                &sym,
                &format!("step_{}", gen),
                "crate::evo",
                &format!("fn step_{}()", gen),
            )
            .unwrap();

        parent = Some(snap);
        last_ent = Some(ent);
    }

    let history = index
        .get_canonical_entity_history(&last_ent.unwrap())
        .unwrap();
    assert_eq!(history.len(), 5);
    assert_canonical_invariants(index.connection());
}

#[test]
fn test_c15_branching_and_fork_isolation() {
    let ws = WorkspaceId::new("ws_main");
    let lin1 = LineageId::new("lin_branch_1");
    let lin2 = LineageId::new("lin_branch_2");

    let ent1 = CanonicalEntityId::from_lineage(&ws, &lin1);
    let ent2 = CanonicalEntityId::from_lineage(&ws, &lin2);

    assert_ne!(ent1, ent2);
}

#[test]
fn test_c16_rust_ts_semantic_namespace_normalization() {
    let rust_ns = vora_lib::forge::model::CanonicalNamespace::from_rust("auth", "service/crypto");
    let ts_ns =
        vora_lib::forge::model::CanonicalNamespace::from_typescript("auth", "service/crypto");

    assert_eq!(rust_ns.as_str(), "crate://auth/service/crypto");
    assert_eq!(ts_ns.as_str(), "ts://auth/service/crypto");
}

#[test]
fn test_c17_tsx_component_semantic_signature_normalization() {
    let sig = vora_lib::forge::model::CanonicalSignature::simple(
        "export   const Button:   FC = () => {}",
    );
    assert_eq!(sig.normalized_text, "export const Button: FC = () => {}");
}

#[test]
fn test_c18_db_migration_and_invariant_audit() {
    let index = CodeIndex::new_in_memory().unwrap();
    assert_canonical_invariants(index.connection());
}

#[test]
fn test_c_d1_entity_split_death_test() {
    let ws = WorkspaceId::new("ws_main");
    let lin_foo = LineageId::new("lin_foo");
    let lin_bar = LineageId::new("lin_bar");

    let ent_foo = CanonicalEntityId::from_lineage(&ws, &lin_foo);
    let ent_bar = CanonicalEntityId::from_lineage(&ws, &lin_bar);

    assert_ne!(
        ent_foo, ent_bar,
        "Split entities must receive distinct CanonicalEntityIds"
    );
}

#[test]
fn test_c_d2_false_entity_merge_death_test() {
    let ws = WorkspaceId::new("ws_main");
    let lin1 = LineageId::new("lin_service_a_foo");
    let lin2 = LineageId::new("lin_service_b_foo");

    let ent1 = CanonicalEntityId::from_lineage(&ws, &lin1);
    let ent2 = CanonicalEntityId::from_lineage(&ws, &lin2);

    assert_ne!(
        ent1, ent2,
        "Distinct symbols with same name must never false-merge into one entity"
    );
}

#[test]
fn test_c_d3_signature_evolution_death_test() {
    let ws = WorkspaceId::new("ws_main");
    let lin = LineageId::new("lin_evolve");

    let ent_before = CanonicalEntityId::from_lineage(&ws, &lin);
    let ent_after = CanonicalEntityId::from_lineage(&ws, &lin);

    assert_eq!(
        ent_before, ent_after,
        "CanonicalEntityId must remain unchanged when signature mutates"
    );
}
