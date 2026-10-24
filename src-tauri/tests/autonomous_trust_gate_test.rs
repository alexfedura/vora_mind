use vora_lib::forge::commit::ForgeTransactionManager;
use vora_lib::forge::differ::SemanticDiffReport;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::*;
use vora_lib::forge::overlay::CodeOverlayManager;
use vora_lib::forge::planner::{DynamicPlanner, ValidatedExecutionPlan};
use vora_lib::forge::project::ProjectContext;

/// A1 & A3: آزمون تولید پلن معتبر شده و اعتبارسنجی تراکنش اتمیک
#[test]
fn test_trust_gate_validated_plan_lifecycle() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_trust_gate_01")
        .expect("Failed to create workspace");

    let file_path = "src/core/auth.rs";
    let base_code = "pub fn verify_credential(key: &str) -> bool { !key.is_empty() }";

    let snap_v1 = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, base_code.as_bytes())])
        .expect("Snapshot v1 failed");

    let lin_auth = LineageId::new("lin_verify_credential");
    let sym_auth_v1 = SymbolId::new("sym_auth_v1");

    index
        .register_canonical_entity(
            &ws_id,
            &snap_v1,
            &lin_auth,
            &sym_auth_v1,
            "verify_credential",
            "core::auth",
            "pub fn verify_credential(key: &str) -> bool",
        )
        .expect("Canonical registration failed");

    let mut overlay = CodeOverlayManager::new("v1", "task_trust_gate");
    let mut tx = ForgeTransactionManager::new("v1", "task_trust_gate", &mut overlay);

    tx.stage_base_file(file_path, base_code);
    let patched_code =
        "pub fn verify_credential(key: &str, min_len: usize) -> bool { key.len() >= min_len }";
    tx.stage_patch(file_path, "verify_credential", patched_code);

    let (diff_report, _) = tx.validate_and_simulate().expect("Simulation must succeed");
    assert_eq!(diff_report.changes.len(), 1);

    let context = ProjectContext::load_from_repo(".").expect("Failed to load context");
    let planner = DynamicPlanner::new(&context, ".");
    let abstract_plan = planner.create_abstract_plan("verify_credential");

    let validated_plan =
        ValidatedExecutionPlan::from_abstract_and_diff(&abstract_plan, &diff_report)
            .expect("Validated plan generation must succeed");

    assert_eq!(validated_plan.verified_mutations_count, 1);

    let commit_res = tx
        .commit_validated_transaction(
            &mut index,
            "Refactor auth with strict min_len parameter",
            &validated_plan,
        )
        .expect("Validated atomic commit must succeed");

    assert!(commit_res.is_committed);
    let snap_v2 = commit_res.snapshot_id.expect("Snapshot v2 must exist");

    // A4: تایید قطعی اینواریانت‌های پس از کامیت
    let verify_res = ForgeTransactionManager::verify_post_commit_invariants(
        &index,
        &ws_id,
        &snap_v2,
        &commit_res.patched_files,
    );
    assert!(
        verify_res.is_ok(),
        "Post-commit invariants must be 100% satisfied"
    );
}

/// A4: آزمون توقف ترکنش در صورت عدم وجود تغییرات معنایی در پلن تأییدشده
#[test]
fn test_trust_gate_rejection_on_empty_validated_plan() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to initialize database");
    let ws_id = index
        .get_or_create_workspace("ws_trust_gate_02")
        .expect("Failed to create workspace");

    let file_path = "src/core/dummy.rs";
    let base_code = "pub fn noop() {}";

    let _snap_v1 = index
        .commit_identity_snapshot(&ws_id, None, &[(file_path, base_code.as_bytes())])
        .expect("Snapshot failed");

    let context = ProjectContext::load_from_repo(".").expect("Failed to load context");
    let planner = DynamicPlanner::new(&context, ".");
    let abstract_plan = planner.create_abstract_plan("noop");

    let empty_diff = SemanticDiffReport::default();
    let validated_plan_res =
        ValidatedExecutionPlan::from_abstract_and_diff(&abstract_plan, &empty_diff);

    assert!(
        validated_plan_res.is_err(),
        "Validated plan must reject empty diffs"
    );
}
