use std::fs;
use vora_lib::forge::commit::TransactionPatch;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::overlay::CodeOverlayManager;
use vora_lib::forge::tools::ForgeToolRegistry;

/// P9-D1: Death Test برای تضمین اینکه اسکن فضای کاری واقعی واقعاً Canonical Registry را پر می‌کند
#[test]
fn test_p9_d1_real_workspace_canonical_population() {
    let temp_dir = std::env::temp_dir().join(format!("vora_p9_d1_{}", uuid::Uuid::now_v7()));
    let ws_path = temp_dir.join("test_workspace");
    let src_path = ws_path.join("src");
    fs::create_dir_all(&src_path).expect("Failed to create test workspace directories");

    // ۱. ایجاد فایل‌های چندزبانه در فضای کاری
    let rust_file = src_path.join("calculator.rs");
    let ts_file = src_path.join("formatter.ts");
    let tsx_file = src_path.join("Widget.tsx");

    fs::write(
        &rust_file,
        r#"
pub struct Calculator;
impl Calculator {
    pub fn compute_sum(a: i32, b: i32) -> i32 {
        a + b
    }
}
"#,
    )
    .unwrap();

    fs::write(
        &ts_file,
        r#"
export interface FormatterConfig {
    indent: number;
}
export function formatText(input: string): string {
    return input.trim();
}
"#,
    )
    .unwrap();

    fs::write(
        &tsx_file,
        r#"
export const Widget: React.FC<Props> = (props) => {
    return <div>{props.title}</div>;
};
"#,
    )
    .unwrap();

    // ۲. اجرای پایپ‌لاین اسکن فضای کاری روی دیتابیس اختصاصی
    let db_path = temp_dir.join("p9_test.db");
    let mut index = CodeIndex::new(&db_path).expect("Failed to initialize CodeIndex");

    let indexed_files = ForgeToolRegistry::index_workspace(ws_path.to_str().unwrap(), &mut index)
        .expect("Workspace indexing pipeline failed");

    assert_eq!(indexed_files, 3, "All 3 source files must be indexed");

    // ۳. اعتبارسنجی عدم مغایرت جداول پایگاه‌داده (Database Consistency Check)
    let conn = index.connection();
    let entity_count_in_db: i64 = conn
        .query_row("SELECT COUNT(*) FROM canonical_entities", [], |row| {
            row.get(0)
        })
        .unwrap();
    let version_count_in_db: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM canonical_entity_versions",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(entity_count_in_db > 0, "canonical_entities table is empty");
    assert!(
        version_count_in_db > 0,
        "canonical_entity_versions table is empty"
    );
    assert_eq!(
        entity_count_in_db, version_count_in_db,
        "Every canonical entity must have an exact corresponding version record"
    );

    // ۴. اعتبارسنجی نمادهای رهگیری‌شده
    let total_symbols = index
        .get_total_symbols_count()
        .expect("Symbol count failed");
    assert!(total_symbols >= 4, "Total indexed symbols must be >= 4");

    let canonical_entities = index
        .get_canonical_entities()
        .expect("Failed to retrieve canonical entities");

    assert!(
        !canonical_entities.is_empty(),
        "CRITICAL P9-D1 FAILURE: Total Canonical Registry Entries is 0!"
    );

    // ۵. اعتبارسنجی وجود نام‌ها در رجیستری کانونیکال
    let names: Vec<&str> = canonical_entities.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Calculator") || names.contains(&"compute_sum"));
    assert!(names.contains(&"formatText") || names.contains(&"FormatterConfig"));
    assert!(names.contains(&"Widget"));

    fs::remove_dir_all(temp_dir).ok();
}

/// P9-D2 & P9.5-F2: راستی‌آزمایی پایداری و مقایسه عمیق کل موجودیت‌های کانونیکال پس از بازگشایی مجدد پایگاه‌داده
#[test]
fn test_p9_d2_restart_canonical_persistence() {
    let temp_dir = std::env::temp_dir().join(format!("vora_p9_d2_{}", uuid::Uuid::now_v7()));
    let ws_path = temp_dir.join("test_workspace");
    fs::create_dir_all(&ws_path).unwrap();
    let db_path = temp_dir.join("p9_restart.db");

    let sample_file = ws_path.join("core.rs");
    fs::write(&sample_file, "pub fn execute_core_task() -> bool { true }").unwrap();

    let mut original_entities;

    // فاز ۱: ایندکس اولیه و بستن کانکشن
    {
        let mut index = CodeIndex::new(&db_path).unwrap();
        ForgeToolRegistry::index_workspace(ws_path.to_str().unwrap(), &mut index).unwrap();
        original_entities = index.get_canonical_entities().unwrap();
        assert!(!original_entities.is_empty());
        original_entities.sort_by(|a, b| a.entity_id.as_str().cmp(b.entity_id.as_str()));
        drop(index);
    }

    // فاز ۲: بازگشایی مجدد و راستی‌آزمایی تطابق ۱۰۰٪ شناسه‌ها و تاریخچه تمام انتیتی‌ها
    {
        let reopened_index = CodeIndex::new(&db_path).unwrap();
        let mut reopened_entities = reopened_index.get_canonical_entities().unwrap();
        reopened_entities.sort_by(|a, b| a.entity_id.as_str().cmp(b.entity_id.as_str()));

        assert_eq!(
            original_entities.len(),
            reopened_entities.len(),
            "Canonical entities count must match perfectly across restart"
        );

        for (orig, reopened) in original_entities.iter().zip(reopened_entities.iter()) {
            assert_eq!(
                orig.entity_id, reopened.entity_id,
                "Entity ID mismatch across restart"
            );
            assert_eq!(
                orig.lineage_id, reopened.lineage_id,
                "Lineage ID mismatch across restart"
            );
            assert_eq!(
                orig.name, reopened.name,
                "Entity name mismatch across restart"
            );
            assert_eq!(
                orig.canonical_signature, reopened.canonical_signature,
                "Signature mismatch across restart"
            );

            let history = reopened_index
                .get_canonical_entity_history(&reopened.entity_id)
                .unwrap();
            assert!(
                !history.is_empty(),
                "History must exist for all entities across restart"
            );
            assert_eq!(
                history[0].canonical_name, orig.name,
                "History root name mismatch"
            );
        }
    }

    fs::remove_dir_all(temp_dir).ok();
}

/// P9.5-F3 & F4: اجرای تراکنش‌های اتمیک پچ و تست Rollback بر روی رجیستری کانونیکال
#[test]
fn test_p9_f3_and_f4_patch_transaction_and_rollback_integrity() {
    let temp_dir = std::env::temp_dir().join(format!("vora_p9_f3_f4_{}", uuid::Uuid::now_v7()));
    let ws_path = temp_dir.join("test_workspace");
    fs::create_dir_all(&ws_path).unwrap();
    let db_path = temp_dir.join("p9_tx.db");

    let file_rel = "src/service.rs";
    let disk_file = ws_path.join(file_rel);
    fs::create_dir_all(disk_file.parent().unwrap()).unwrap();
    let initial_code = "pub fn execute() -> i32 { 42 }\n";
    fs::write(&disk_file, initial_code).unwrap();

    let mut index = CodeIndex::new(&db_path).unwrap();
    ForgeToolRegistry::index_workspace(ws_path.to_str().unwrap(), &mut index).unwrap();

    let initial_entities = index.get_canonical_entities().unwrap();
    let initial_count = initial_entities.len();
    assert!(initial_count >= 1);

    // ۱. شبیه‌سازی شکست تراکنش (F4): سینتکس نامعتبر و اعتبارسنجی Rollback
    {
        let mut overlay = CodeOverlayManager::new("base_c0", "task_fail");
        let invalid_patch = vec![TransactionPatch {
            file_path: file_rel.to_string(),
            symbol_id: "execute".to_string(),
            new_symbol_code: "pub fn execute() -> i32 { ??? unparseable }".to_string(),
        }];

        let result = ForgeToolRegistry::execute_multi_patch_transaction(
            "base_c0",
            "task_fail",
            &mut overlay,
            &mut index,
            &[(file_rel, initial_code)],
            &invalid_patch,
            "Broken syntax patch",
        );

        assert!(
            result.is_err(),
            "Invalid syntax patch must fail and trigger rollback"
        );
        let entities_after_fail = index.get_canonical_entities().unwrap();
        assert_eq!(
            entities_after_fail.len(),
            initial_count,
            "Database must remain untouched after failed patch"
        );
    }

    // ۲. شبیه‌سازی موفقیت تراکنش (F3): پچ معتبر و ثبت در دیتابیس
    {
        let mut overlay = CodeOverlayManager::new("base_c0", "task_success");
        let valid_patch = vec![TransactionPatch {
            file_path: file_rel.to_string(),
            symbol_id: "execute".to_string(),
            new_symbol_code: "pub fn execute() -> i32 { 100 }".to_string(),
        }];

        let result = ForgeToolRegistry::execute_multi_patch_transaction(
            "base_c0",
            "task_success",
            &mut overlay,
            &mut index,
            &[(file_rel, initial_code)],
            &valid_patch,
            "Refactor return value to 100",
        );

        assert!(
            result.is_ok(),
            "Valid patch transaction must commit cleanly"
        );
        let commit_res = result.unwrap();
        assert!(commit_res.is_committed);
        assert!(commit_res.snapshot_id.is_some());
    }

    fs::remove_dir_all(temp_dir).ok();
}
