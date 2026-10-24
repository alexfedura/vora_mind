use vora_lib::forge::planner::{DynamicPlanner, StepPhase};
use vora_lib::forge::project::ProjectContext;

#[test]
fn test_p10_5_h4_plan_repair_and_expansion() {
    let context = ProjectContext::load_from_repo(".").expect("Failed to load repo context");
    assert!(
        !context.all_definitions.is_empty(),
        "Indexed definitions required"
    );

    let planner = DynamicPlanner::new(&context, "..");
    let initial_symbol = &context.all_definitions[0].name;

    // ۱. ساخت پلن اولیه
    let initial_plan = planner.create_plan(initial_symbol);
    let initial_step_count = initial_plan.total_steps;

    // ۲. شبیه‌سازی کشف نماد واقعاً ناشناخته و خارج از پلن فعلی
    let discovered_symbol = format!("dynamically_discovered_caller_{}", std::process::id());

    // ۳. اجرای بازسازی و گسترش پلن در هسته Rust
    let repaired_plan =
        planner.repair_and_expand_plan(&initial_plan, std::slice::from_ref(&discovered_symbol));

    // ۴. اعتبارسنجی گسترش پلن
    assert_eq!(
        repaired_plan.total_steps,
        initial_step_count + 1,
        "Repaired plan must expand by exactly one new step"
    );

    let added_step = repaired_plan.steps.last().unwrap();
    assert_eq!(added_step.symbol_name, discovered_symbol);
    assert_eq!(added_step.phase, StepPhase::CallSiteMigration);
    assert_eq!(added_step.step_order, initial_step_count + 1);
}
