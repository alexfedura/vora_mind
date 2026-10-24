use std::fs;
use vora_lib::forge::commit::TransactionPatch;
use vora_lib::forge::model::SemanticChange;
use vora_lib::forge::tools::ForgeToolRegistry;

#[test]
fn test_p10_5_h3_candidate_workspace_reindex_and_diff() {
    let base_temp = std::env::temp_dir().join(format!("vora_h3_reindex_{}", std::process::id()));
    let _ = fs::remove_dir_all(&base_temp);
    fs::create_dir_all(base_temp.join("src")).expect("Failed to create temp project");

    // ۱. ایجاد سورس پایه ریپازیتوری
    let calc_rs = base_temp.join("src/calc.rs");
    let initial_code = r#"
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#;
    fs::write(&calc_rs, initial_code).expect("Failed to write calc.rs");

    // ۲. تعریف پچ کاندیدا که هم امضا و هم بدنه را جهش می‌دهد
    let patches = vec![TransactionPatch {
        file_path: "src/calc.rs".to_string(),
        symbol_id: "multiply".to_string(),
        new_symbol_code:
            "pub fn multiply(a: i32, b: i32, factor: i32) -> i32 {\n    a * b * factor\n}"
                .to_string(),
    }];

    // ۳. اجرای ارزیابی تفاوت معنایی مستقیماً از روی بازایندکس سندباکس کاندیدا
    let diff_report = ForgeToolRegistry::evaluate_candidate_workspace_semantics(
        base_temp.to_str().unwrap(),
        &patches,
    )
    .expect("Failed to evaluate candidate workspace semantics");

    // ۴. اعتبارسنجی: تفاوت معنایی باید مستقیماً SignatureChanged را با پارامتر جدید factor تشخیص داده باشد
    assert_eq!(
        diff_report.changes.len(),
        1,
        "Exactly one semantic change must be detected"
    );
    match &diff_report.changes[0] {
        SemanticChange::SignatureChanged {
            old_signature,
            new_signature,
            ..
        } => {
            assert!(old_signature.contains("a: i32, b: i32"));
            assert!(new_signature.contains("factor: i32"));
        }
        other => panic!(
            "Expected SignatureChanged from candidate AST reindex, got {:?}",
            other
        ),
    }

    // ۵. اطمینان از عدم آلودگی فایل پایه
    let base_read = fs::read_to_string(&calc_rs).unwrap();
    assert_eq!(
        base_read, initial_code,
        "Base repository file must remain completely untouched"
    );

    let _ = fs::remove_dir_all(&base_temp);
}
