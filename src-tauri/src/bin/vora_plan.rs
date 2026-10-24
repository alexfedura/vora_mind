use std::env;
use vora_lib::forge::planner::DynamicPlanner;
use vora_lib::forge::project::ProjectContext;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("=======================================================");
        eprintln!(" 📋 VORA DYNAMIC SEMANTIC PLANNER CLI v1.0");
        eprintln!("=======================================================");
        eprintln!("Usage: cargo run --bin vora_plan -- <SYMBOL_NAME> [--repo <PATH>]");
        eprintln!("=======================================================");
        std::process::exit(1);
    }

    let mut target_symbol = String::new();
    let mut repo_path = ".".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--repo" | "-r" => {
                if i + 1 < args.len() {
                    repo_path = args[i + 1].clone();
                    i += 1;
                }
            }
            arg if !arg.starts_with('-') && target_symbol.is_empty() => {
                target_symbol = arg.to_string();
            }
            _ => {}
        }
        i += 1;
    }

    if target_symbol.is_empty() {
        eprintln!("Error: Target symbol name is required.");
        std::process::exit(1);
    }

    println!(
        "[vora-plan] Indexing codebase from Git SSOT at '{}'...",
        repo_path
    );

    let context = ProjectContext::load_from_repo(&repo_path)?;
    let planner = DynamicPlanner::new(&context, &repo_path);
    let plan = planner.create_plan(&target_symbol);

    println!("\n=======================================================");
    println!(" 🚀 EXECUTION PLAN FOR: '{}'", plan.root_symbol);
    println!(" Total Sequential Steps: {}", plan.total_steps);
    println!("=======================================================");

    for step in &plan.steps {
        println!("\n👉 STEP #{}: [{}]", step.step_order, step.symbol_name);
        println!("   File: {}", step.file_path);
        println!("   Reason: {}", step.reason);
        println!("   Depth: {}", step.cascade_depth);

        if let Some(slice) = &step.code_slice {
            println!("   --- Sliced Code Preview ---");
            let line_count = slice.target_code.lines().count();
            for line in slice.target_code.lines().take(6) {
                println!("   | {}", line);
            }
            if line_count > 6 {
                println!("   | ... [rest of code omitted for brevity]");
            }
        }
    }

    println!("\n✅ Plan ready. Feed these scoped slices to your LLM prompt sequentially.");
    Ok(())
}
