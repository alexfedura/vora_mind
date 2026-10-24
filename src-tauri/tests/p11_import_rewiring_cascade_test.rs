use vora_lib::forge::differ::SemanticDiffer;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::*;
use vora_lib::forge::overlay::CodeOverlayManager;
use vora_lib::forge::patcher_v2::SurgicalPatcher;
use vora_lib::forge::tools::{CandidateWorkspace, ForgeToolRegistry};

#[test]
fn test_p11_import_rewiring_and_cascading_patch() {
    let mut index = CodeIndex::open_in_memory().expect("Database init failed");
    let ws_id = index
        .get_or_create_workspace("ws_p11_import_test")
        .expect("Workspace creation failed");

    // ۱. شبیه‌سازی فایل ریشه و فایل مصرف‌کننده
    let math_file = "src/math.rs";
    let math_code_v1 = "pub fn calculate_tax(amount: f64) -> f64 { amount * 0.09 }";

    let billing_file = "src/billing.rs";
    let billing_code_v1 = "use crate::math::calculate_tax;\n\npub fn process_bill(subtotal: f64) -> f64 {\n    calculate_tax(subtotal)\n}";

    let snap_v1 = index
        .commit_identity_snapshot(
            &ws_id,
            None,
            &[
                (math_file, math_code_v1.as_bytes()),
                (billing_file, billing_code_v1.as_bytes()),
            ],
        )
        .expect("Snapshot v1 failed");

    let lin_tax = LineageId::new("lin_calculate_tax");
    let sym_tax_v1 = SymbolId::new("sym_tax_v1");

    index
        .register_canonical_entity(
            &ws_id,
            &snap_v1,
            &lin_tax,
            &sym_tax_v1,
            "calculate_tax",
            "math",
            "pub fn calculate_tax(amount: f64) -> f64",
        )
        .expect("Registration failed");

    // ۲. اجرای سیم‌کشی مجدد خودکار ایمپورت‌ها در billing.rs برای تغییر نام calculate_tax به compute_vat
    let rewired_billing = SurgicalPatcher::apply_import_rewiring(
        billing_file,
        billing_code_v1,
        "calculate_tax",
        "compute_vat",
    )
    .expect("Import rewiring must succeed");

    assert!(rewired_billing.contains("use crate::math::compute_vat;"));

    // ۳. شبیه‌سازی پچ چندفایلی در Candidate Workspace
    let math_code_v2 = "pub fn compute_vat(amount: f64, rate: f64) -> f64 { amount * rate }";
    let billing_code_v2 = "use crate::math::compute_vat;\n\npub fn process_bill(subtotal: f64) -> f64 {\n    compute_vat(subtotal, 0.09)\n}";

    let temp_sandbox =
        CandidateWorkspace::create(std::path::Path::new(".")).expect("Sandbox creation failed");
    temp_sandbox.apply_patch(math_file, math_code_v2).unwrap();
    temp_sandbox
        .apply_patch(billing_file, billing_code_v2)
        .unwrap();

    // ۴. اعتبارسنجی تفاوت معنایی AST در سندباکس (تغییر نام + جهش امضا = ۲ تغییر معنایی)
    let snap_math_old = SemanticDiffer::create_file_snapshot(math_file, math_code_v1);
    let snap_math_new = SemanticDiffer::create_file_snapshot(math_file, math_code_v2);

    let diff_report = SemanticDiffer::diff_project_snapshots(&[snap_math_old], &[snap_math_new]);
    assert_eq!(diff_report.changes.len(), 2);

    let has_rename = diff_report.changes.iter().any(|c| match c {
        SemanticChange::SymbolRenamed {
            old_name, new_name, ..
        } => old_name == "calculate_tax" && new_name == "compute_vat",
        _ => false,
    });
    assert!(has_rename, "Must detect SymbolRenamed change");

    let has_sig_change = diff_report
        .changes
        .iter()
        .any(|c| matches!(c, SemanticChange::SignatureChanged { .. }));
    assert!(has_sig_change, "Must detect SignatureChanged change");

    // ۵. اجرای تراکنش اتمیک چندفایلی
    let mut overlay = CodeOverlayManager::new("v1", "task_p11_cascade");
    let patches = vec![
        vora_lib::forge::commit::TransactionPatch {
            file_path: math_file.to_string(),
            symbol_id: "calculate_tax".to_string(),
            new_symbol_code: math_code_v2.to_string(),
        },
        vora_lib::forge::commit::TransactionPatch {
            file_path: billing_file.to_string(),
            symbol_id: "process_bill".to_string(),
            new_symbol_code:
                "pub fn process_bill(subtotal: f64) -> f64 {\n    compute_vat(subtotal, 0.09)\n}"
                    .to_string(),
        },
    ];

    let tx_res = ForgeToolRegistry::execute_multi_patch_transaction(
        "v1",
        "task_p11_cascade",
        &mut overlay,
        &mut index,
        &[(math_file, math_code_v1), (billing_file, billing_code_v1)],
        &patches,
        "Cascading rename and signature update across math and billing",
    )
    .expect("Multi-file transaction must succeed");

    assert!(tx_res.is_committed);
    assert_eq!(tx_res.patched_files.len(), 2);
}
