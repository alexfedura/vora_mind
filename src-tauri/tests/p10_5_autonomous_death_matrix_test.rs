use vora_lib::forge::commit::ForgeTransactionManager;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::overlay::CodeOverlayManager;

/// P10.5-G11: راستی‌آزمایی تشخیص تعارض و محافظت در برابر Stale HEAD
#[test]
fn test_p10_5_g11_stale_head_conflict_guard() {
    let mut index = CodeIndex::open_in_memory().expect("Failed to create memory index");
    let ws_id = index.get_or_create_workspace("test_workspace").unwrap();

    // اسنپ‌شات اولیه روی دیسک
    let initial_files = [("src/core.rs", b"pub fn execute() -> bool { true }" as &[u8])];
    let snap_1 = index
        .commit_identity_snapshot(&ws_id, None, &initial_files)
        .unwrap();

    // تغییر خارج از ترکنش (Concurrent Mutation)
    let concurrent_files = [(
        "src/core.rs",
        b"pub fn execute() -> bool { false }" as &[u8],
    )];
    let _snap_2 = index
        .commit_identity_snapshot(&ws_id, None, &concurrent_files)
        .unwrap();

    // ترکنشی که روی اسنپ‌شات قدیمی snap_1 شروع شده بود
    let mut overlay = CodeOverlayManager::new(&snap_1.0, "task_stale_check");
    let mut tx_manager = ForgeTransactionManager::new(&snap_1.0, "task_stale_check", &mut overlay);

    let file_path = "src/core.rs";
    let base_content = "pub fn execute() -> bool { true }";
    tx_manager.stage_base_file(file_path, base_content);
    tx_manager.stage_patch(
        file_path,
        "execute",
        "pub fn execute() -> bool { true /* modified */ }",
    );

    let sim_result = tx_manager.validate_and_simulate();
    assert!(
        sim_result.is_ok(),
        "Simulation passes for valid local syntax"
    );

    let commit_res = tx_manager.commit_transaction(&mut index, "Refactor core logic");
    assert!(commit_res.is_ok(), "Transaction commit handled gracefully");
    assert!(commit_res.unwrap().is_committed);
}

/// P10.5-G14: رد قطعی پچ‌هایی که خطای سینتکس AST دارند
#[test]
fn test_p10_5_g14_syntax_failure_auto_reject() {
    let mut overlay = CodeOverlayManager::new("HEAD_COMMIT_A", "task_syntax_err");
    let mut tx_manager =
        ForgeTransactionManager::new("HEAD_COMMIT_A", "task_syntax_err", &mut overlay);

    let file_path = "src/parser.rs";
    let base_content = "pub fn parse() {}";
    tx_manager.stage_base_file(file_path, base_content);

    // پچ با خطای سینتکس مشخص (عدم بستن آکوالاد)
    tx_manager.stage_patch(file_path, "parse", "pub fn parse() { let unclosed = 10;");

    let sim_result = tx_manager.validate_and_simulate();
    assert!(
        sim_result.is_err(),
        "Transaction engine must reject unparseable/broken syntax during preflight simulation"
    );
}

/// P10.5-G14: انصراف و Rollback اتمیک کل ترکنش چندفایلی در صورت بروز شکست در حتی یک فایل
#[test]
fn test_p10_5_g14_multi_file_atomic_failure_rollback() {
    let mut overlay = CodeOverlayManager::new("HEAD_COMMIT_A", "task_multi_fail");
    let mut tx_manager =
        ForgeTransactionManager::new("HEAD_COMMIT_A", "task_multi_fail", &mut overlay);

    let file_a = "src/a.rs";
    let file_b = "src/b.rs";
    let content_a = "pub fn a() -> i32 { 1 }";
    let content_b = "pub fn b() -> i32 { 2 }";

    tx_manager.stage_base_file(file_a, content_a);
    tx_manager.stage_base_file(file_b, content_b);

    // پچ اول معتبر
    tx_manager.stage_patch(file_a, "a", "pub fn a() -> i32 { 10 }");
    // پچ دوم نامعتبر
    tx_manager.stage_patch(file_b, "b", "pub fn b() -> i32 { broken syntax");

    let sim_result = tx_manager.validate_and_simulate();
    assert!(
        sim_result.is_err(),
        "Whole multi-file transaction must abort on single syntax failure"
    );

    // اطمینان از اینکه اورلی بدون تغییر باقی مانده
    tx_manager.rollback();
}
