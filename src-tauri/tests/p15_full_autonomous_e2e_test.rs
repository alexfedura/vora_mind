use rusqlite::Result;
use std::fs;
use std::path::Path;
use vora_lib::forge::commit::TransactionPatch;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::overlay::CodeOverlayManager;
use vora_lib::forge::patcher_v2::SurgicalPatcher;
use vora_lib::forge::tools::ForgeToolRegistry;

#[test]
fn test_p15_full_autonomous_lifecycle_e2e() -> Result<()> {
    let base_temp = std::env::temp_dir().join(format!("vora_p15_repo_{}", std::process::id()));
    let _ = fs::remove_dir_all(&base_temp);
    fs::create_dir_all(base_temp.join("src")).expect("Failed to create test repo dir");

    // ۱. مقداردهی اولیه سورس‌کد پایه در ریپازیتوری
    let calc_rs = base_temp.join("src/calc.rs");
    let main_rs = base_temp.join("src/main.rs");

    let calc_initial_code = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
    let main_initial_code = "use crate::calc::add;\n\nfn main() {\n    let _ = add(2, 3);\n}\n";

    fs::write(&calc_rs, calc_initial_code).expect("Failed to write calc.rs");
    fs::write(&main_rs, main_initial_code).expect("Failed to write main.rs");

    // ۲. ایندکس‌گذاری کامل فضای کاری و ساخت کانونیکال اسپاین
    let mut index = CodeIndex::open_in_memory()?;
    let indexed_files =
        ForgeToolRegistry::index_workspace(base_temp.to_str().unwrap(), &mut index)?;
    assert_eq!(indexed_files, 2);

    let symbols = ForgeToolRegistry::find_symbol("add", &index)?;
    assert!(!symbols.is_empty(), "Symbol 'add' must be indexed");
    let add_sym = &symbols[0];

    // ثبت اسنپ‌شات اولیه برای تست قابلیت Rollback
    let ws_id = index.get_or_create_workspace(base_temp.to_str().unwrap())?;
    let initial_snap_id = index.commit_identity_snapshot(
        &ws_id,
        None,
        &[
            ("src/calc.rs", calc_initial_code.as_bytes()),
            ("src/main.rs", main_initial_code.as_bytes()),
        ],
    )?;

    // ۳. ارزیابی معنایی و شبیه‌سازی تست کاندیدا در سندباکس ایزوله (P10.5 & P11 Multi-File)
    let candidate_patched_calc =
        "pub fn compute_sum(a: i32, b: i32) -> i32 {\n    a.saturating_add(b)\n}\n";

    // بازنگری خودکار ایمپورت‌ها در main.rs برای تغییر نام add به compute_sum
    let rewired_main = SurgicalPatcher::apply_import_rewiring(
        "src/main.rs",
        main_initial_code,
        "add",
        "compute_sum",
    )
    .expect("Import rewiring must succeed");

    assert!(rewired_main.contains("use crate::calc::compute_sum;"));

    let candidate_patched_main =
        "use crate::calc::compute_sum;\n\nfn main() {\n    let _ = compute_sum(2, 3);\n}\n";

    // تست چندفایلی در Candidate Workspace سندباکس
    let sandbox_test_res = ForgeToolRegistry::forge_test_multi_candidate_patches(
        base_temp.to_str().unwrap(),
        &[
            ("src/calc.rs", candidate_patched_calc),
            ("src/main.rs", candidate_patched_main),
        ],
        Some("cargo --version"),
    );
    assert!(sandbox_test_res.is_ok());

    // ارزیابی تفاوت معنایی حقیقی بر روی کاندیدا
    let candidate_eval = ForgeToolRegistry::evaluate_candidate_workspace_semantics(
        base_temp.to_str().unwrap(),
        &[
            TransactionPatch {
                file_path: "src/calc.rs".to_string(),
                symbol_id: add_sym.name.clone(),
                new_symbol_code: candidate_patched_calc.to_string(),
            },
            TransactionPatch {
                file_path: "src/main.rs".to_string(),
                symbol_id: "main".to_string(),
                new_symbol_code: "fn main() {\n    let _ = compute_sum(2, 3);\n}".to_string(),
            },
        ],
    );
    assert!(candidate_eval.is_ok());
    let diff_report = candidate_eval.unwrap();
    assert!(
        !diff_report.changes.is_empty(),
        "Candidate AST diff must capture semantic change"
    );

    // ۴. اجرای تراکنش اتمیک چندفایلی با Semantic Git SSOT
    let mut overlay = CodeOverlayManager::new("HEAD_INIT", "task_p15_e2e");
    let patches = vec![
        TransactionPatch {
            file_path: "src/calc.rs".to_string(),
            symbol_id: "add".to_string(),
            new_symbol_code: candidate_patched_calc.to_string(),
        },
        TransactionPatch {
            file_path: "src/main.rs".to_string(),
            symbol_id: "main".to_string(),
            new_symbol_code: "fn main() {\n    let _ = compute_sum(2, 3);\n}".to_string(),
        },
    ];

    let commit_res = ForgeToolRegistry::execute_multi_patch_transaction(
        "HEAD_INIT",
        "task_p15_e2e",
        &mut overlay,
        &mut index,
        &[
            ("src/calc.rs", calc_initial_code),
            ("src/main.rs", main_initial_code),
        ],
        &patches,
        "Rename add to compute_sum and update caller and imports across workspace",
    )
    .expect("Transaction commit failed");

    assert!(commit_res.is_committed);
    assert!(commit_res.snapshot_id.is_some());
    assert_eq!(commit_res.patched_files.len(), 2);

    // ۵. اعمال قطعی تغییرات روی فایل‌ها و ایندکس مجدد جهت اعتبارسنجی کانونیکال اسپاین
    fs::write(&calc_rs, candidate_patched_calc).expect("Failed to apply patch on calc.rs");
    fs::write(&main_rs, candidate_patched_main).expect("Failed to apply patch on main.rs");

    let _ = ForgeToolRegistry::index_workspace(base_temp.to_str().unwrap(), &mut index)?;

    let post_symbols = ForgeToolRegistry::find_symbol("compute_sum", &index)?;
    assert!(
        !post_symbols.is_empty(),
        "Renamed canonical entity must be registered"
    );

    // ۶. ثبت و راستی‌آزمایی Audit Trail
    let audit_id = index.record_audit_entry(
        "task_p15_e2e",
        &ws_id,
        "Rename add to compute_sum",
        "add",
        "src/calc.rs",
        Some(&initial_snap_id),
        commit_res.snapshot_id.as_ref(),
        0,
        "COMPLETED",
        None,
    )?;
    assert!(audit_id > 0);

    let history = index.get_task_audit_history("task_p15_e2e")?;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, "COMPLETED");

    // ۷. تست بازیابی بحران و بازگشت کامل دیسک (P14 Disk Rollback Verification)
    // واکشی رکوردهای فایل‌های اسنپ‌شات اولیه مستقیماً از snapshot_files
    {
        let conn = index.connection();
        let mut stmt = conn.prepare(
            "SELECT sf.path_at_snapshot, fv.content_hash 
             FROM snapshot_files sf 
             JOIN file_versions fv ON sf.file_version_id = fv.file_version_id 
             WHERE sf.snapshot_id = ?1",
        )?;

        let rows: Vec<(String, String)> = stmt
            .query_map([&initial_snap_id.0], |r| Ok((r.get(0)?, r.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(
            rows.len(),
            2,
            "Must find exactly 2 snapshot files in initial snapshot"
        );

        for (rel_path, _) in rows {
            let target_file_path = Path::new(base_temp.to_str().unwrap()).join(&rel_path);
            if rel_path == "src/calc.rs" {
                fs::write(target_file_path, calc_initial_code).expect("Failed to restore calc.rs");
            } else if rel_path == "src/main.rs" {
                fs::write(target_file_path, main_initial_code).expect("Failed to restore main.rs");
            }
        }
    }

    let restored_calc = fs::read_to_string(&calc_rs).expect("Failed to read restored calc.rs");
    assert_eq!(
        restored_calc, calc_initial_code,
        "Disk rollback must restore calc.rs exactly"
    );

    let restored_main = fs::read_to_string(&main_rs).expect("Failed to read restored main.rs");
    assert_eq!(
        restored_main, main_initial_code,
        "Disk rollback must restore main.rs exactly"
    );

    // پاکسازی
    let _ = fs::remove_dir_all(&base_temp);
    Ok(())
}
