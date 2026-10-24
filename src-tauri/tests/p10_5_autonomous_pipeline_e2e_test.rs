use rusqlite::Connection;
use std::fs;
use std::process::Command;
use vora_lib::forge::commit::TransactionPatch;
use vora_lib::forge::index::{CodeIndex, IDENTITY_SPINE_SCHEMA};
use vora_lib::forge::model::SemanticChange;
use vora_lib::forge::overlay::CodeOverlayManager;
use vora_lib::forge::planner::DynamicPlanner;
use vora_lib::forge::project::ProjectContext;
use vora_lib::forge::tools::ForgeToolRegistry;

#[test]
fn test_p10_5_h5_full_autonomous_pipeline_e2e() {
    let pid = std::process::id();
    let project_dir = std::env::temp_dir().join(format!("vora_e2e_pipeline_{}", pid));
    let db_path = std::env::temp_dir().join(format!("vora_e2e_db_{}.db", pid));

    let _ = fs::remove_dir_all(&project_dir);
    let _ = fs::remove_file(&db_path);

    fs::create_dir_all(project_dir.join("src")).expect("Failed to create temp project");

    // ۱. مقداردهی اولیه ریپازیتوری Git در پوشه موقت برای جلوگیری از پیمایش والد توسط Git2
    let _ = Command::new("git")
        .arg("init")
        .current_dir(&project_dir)
        .output();

    // ۲. ساخت دیتابیس و اسکیما
    let conn = Connection::open(&db_path).expect("Failed to open temp SQLite db");
    conn.execute_batch(IDENTITY_SPINE_SCHEMA)
        .expect("Failed to execute IDENTITY_SPINE_SCHEMA");
    let mut index = CodeIndex::new(&db_path).expect("Failed to create CodeIndex");

    // ۳. ایجاد فایل‌های پایه پروژه
    let calc_file = "src/calc.rs";
    let calc_path = project_dir.join(calc_file);
    let initial_calc_code = r#"
pub fn compute_sum(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    fs::write(&calc_path, initial_calc_code).expect("Failed to write calc.rs");

    // ۴. ایندکس‌گذاری اولیه پروژه در ایندکس کانونیکال
    let indexed_files =
        ForgeToolRegistry::index_workspace(project_dir.to_str().unwrap(), &mut index)
            .expect("Failed to index workspace");
    assert!(indexed_files >= 1);

    // ۵. ساخت پلن و استخراج اسلایس معنایی
    let context = ProjectContext::load_from_repo(project_dir.to_str().unwrap())
        .expect("Failed to load project context");
    let planner = DynamicPlanner::new(&context, &project_dir);
    let initial_plan = planner.create_plan("compute_sum");
    assert_eq!(initial_plan.total_steps, 1);

    // ۶. تعریف پچ کاندیدا (تغییر بدنه و اضافه کردن کامنت یا منطق جدید)
    let new_calc_code = "pub fn compute_sum(a: i32, b: i32) -> i32 {\n    // modernized internal sum\n    a.saturating_add(b)\n}";
    let patches = vec![TransactionPatch {
        file_path: calc_file.to_string(),
        symbol_id: "compute_sum".to_string(),
        new_symbol_code: new_calc_code.to_string(),
    }];

    // ۷. ارزیابی معنایی بر روی سندباکس کاندیدا (Candidate Reindexing)
    let diff_report = ForgeToolRegistry::evaluate_candidate_workspace_semantics(
        project_dir.to_str().unwrap(),
        &patches,
    )
    .expect("Failed to evaluate candidate semantics");

    assert_eq!(diff_report.changes.len(), 1);
    match &diff_report.changes[0] {
        SemanticChange::BodyChanged {
            old_body_hash,
            new_body_hash,
            ..
        } => {
            assert_ne!(old_body_hash, new_body_hash);
        }
        other => panic!(
            "Expected BodyChanged from candidate AST reindex, got {:?}",
            other
        ),
    }

    // ۸. اجرای تراکنش اتمیک همه‌یاهیچ در دیتابیس
    let mut overlay = CodeOverlayManager::new("HEAD", "e2e_task_1");
    let base_files = vec![(calc_file, initial_calc_code)];

    let tx_res = ForgeToolRegistry::execute_multi_patch_transaction(
        "HEAD",
        "e2e_task_1",
        &mut overlay,
        &mut index,
        &base_files,
        &patches,
        "Modernize sum calculation",
    )
    .expect("Failed to execute atomic patch transaction");

    assert!(tx_res.is_committed, "Transaction must commit successfully");
    assert!(
        tx_res.snapshot_id.is_some(),
        "Snapshot ID must be generated"
    );

    // پاکسازی تمیز فایل‌های موقت
    let _ = fs::remove_dir_all(&project_dir);
    let _ = fs::remove_file(&db_path);
}
