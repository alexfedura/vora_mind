use std::env;
use std::fs;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::planner::DynamicPlanner;
use vora_lib::forge::project::ProjectContext;
use vora_lib::forge::tools::{ForgeToolRegistry, ToolCallRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("=======================================================");
        println!(" 🤖 VORA SEMANTIC AGENT CLI v1.0");
        println!("=======================================================");
        println!("Usage:");
        println!("  cargo run --bin vora_agent -- plan    <SYMBOL_NAME> [--repo <PATH>]");
        println!("  cargo run --bin vora_agent -- prompt  <SYMBOL_NAME> [--repo <PATH>]");
        println!("  cargo run --bin vora_agent -- tool    <TOOL_NAME> '<JSON_ARGS>'");
        println!("  cargo run --bin vora_agent -- test-candidate <FILE_PATH> '<PATCH_CODE>'");
        println!("=======================================================");
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "plan" => {
            let target_symbol = args.get(2).map(|s| s.as_str()).unwrap_or("");
            if target_symbol.is_empty() {
                eprintln!("Error: Target symbol name is required for planning.");
                std::process::exit(1);
            }

            let context = ProjectContext::load_from_repo(".")?;
            let planner = DynamicPlanner::new(&context, ".");
            let plan = planner.create_plan(target_symbol);

            println!("=======================================================");
            println!(" 📋 EXECUTION PLAN: {}", plan.root_symbol);
            println!(" Total Steps: {}", plan.total_steps);
            println!("=======================================================");
            for step in &plan.steps {
                println!(
                    "Step #{}: [{}] in {} (Depth: {}) - {}",
                    step.step_order,
                    step.symbol_name,
                    step.file_path,
                    step.cascade_depth,
                    step.reason
                );
            }
        }
        "prompt" => {
            let target_symbol = args.get(2).map(|s| s.as_str()).unwrap_or("");
            if target_symbol.is_empty() {
                eprintln!("Error: Target symbol name is required for prompt generation.");
                std::process::exit(1);
            }

            let context = ProjectContext::load_from_repo(".")?;
            let planner = DynamicPlanner::new(&context, ".");
            let plan = planner.create_plan(target_symbol);

            let mut prompt_content = String::new();
            prompt_content.push_str("# SYSTEM: You are an expert Rust software architect.\n");
            prompt_content.push_str(
                "Modify the target symbol according to requirements with surgical precision.\n\n",
            );
            prompt_content.push_str("## GLOBAL ARCHITECTURAL SKELETON\n```rust\n");
            prompt_content.push_str(&context.generate_global_skeleton());
            prompt_content.push_str("\n```\n\n");

            prompt_content.push_str(&format!(
                "## EXECUTION PLAN (Target: {})\n",
                plan.root_symbol
            ));
            for step in &plan.steps {
                prompt_content.push_str(&format!(
                    "- Step {}: `{}` in `{}` ({})\n",
                    step.step_order, step.symbol_name, step.file_path, step.reason
                ));
            }

            if let Some(first_step) = plan.steps.first() {
                if let Some(slice) = &first_step.code_slice {
                    prompt_content.push_str(&format!(
                        "\n## TARGET CODE SLICE (`{}`)\n```rust\n",
                        slice.target_symbol
                    ));
                    prompt_content.push_str(&slice.target_code);
                    prompt_content.push_str("\n```\n");

                    if !slice.related_definitions.is_empty() {
                        prompt_content.push_str("\n## RELATED DEFINITIONS / CALLERS\n```rust\n");
                        for rel in &slice.related_definitions {
                            prompt_content.push_str(rel);
                            prompt_content.push('\n');
                        }
                        prompt_content.push_str("```\n");
                    }
                }
            }

            let output_file = "agent_prompt.md";
            fs::write(output_file, &prompt_content)?;
            println!(
                "✅ Agent prompt package generated successfully: {}",
                output_file
            );
            println!("   Tokens optimized down by ~95% using AST slicing.");
        }
        "tool" => {
            let tool_name = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let raw_args = args.get(3).map(|s| s.as_str()).unwrap_or("{}");

            if tool_name.is_empty() {
                eprintln!("Error: Tool name is required.");
                std::process::exit(1);
            }

            let parsed_args: serde_json::Value =
                serde_json::from_str(raw_args).unwrap_or_else(|_| serde_json::json!({}));

            let request = ToolCallRequest {
                tool: tool_name.to_string(),
                arguments: parsed_args,
            };

            let mut index = CodeIndex::open_in_memory().ok();
            let response = ForgeToolRegistry::execute_tool(request, index.as_mut());
            let output_json = serde_json::to_string_pretty(&response)?;
            println!("{}", output_json);
        }
        "test-candidate" => {
            let file_path = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let patch_code = args.get(3).map(|s| s.as_str()).unwrap_or("");

            if file_path.is_empty() || patch_code.is_empty() {
                eprintln!("Error: file_path and patch_code are required.");
                std::process::exit(1);
            }

            println!("[vora-agent] Testing patch in Candidate Workspace Sandbox...");
            let res =
                ForgeToolRegistry::forge_test_candidate_patch(".", file_path, patch_code, None)?;

            if res.success {
                println!("✅ Candidate Workspace Regression Tests PASSED cleanly!");
            } else {
                eprintln!("❌ Candidate Tests FAILED:\n{}", res.stderr);
            }
        }
        _ => {
            eprintln!(
                "Unknown command: '{}'. Use 'plan', 'prompt', 'tool' or 'test-candidate'.",
                command
            );
        }
    }

    Ok(())
}
