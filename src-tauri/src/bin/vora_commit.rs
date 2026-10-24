use std::env;
use vora_lib::forge::commit::{CommitRationale, SemanticCommitPayload};
use vora_lib::forge::differ::SemanticDiffer;
use vora_lib::forge::ssot::GitSSOT;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Parse Intent and Optional Task ID from CLI flags
    let mut intent = "chore(ast): update AST structures".to_string();
    let mut task_id = None;
    let mut strict_semantic_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--task" | "-t" => {
                if i + 1 < args.len() {
                    task_id = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--semantic" | "-s" => {
                strict_semantic_mode = true;
            }
            arg if !arg.starts_with('-') => {
                intent = arg.to_string();
            }
            _ => {}
        }
        i += 1;
    }

    println!("[vora-git] Opening repository index...");
    let ssot = GitSSOT::open(".")?;
    let changes = ssot.get_semantic_changes()?;

    if changes.is_empty() {
        println!("ℹ [vora-git] Working tree clean. No tracked files modified.");
        return Ok(());
    }

    let rationale = CommitRationale {
        intent: intent.clone(),
        confidence_score: 1.0,
        file_deltas: Vec::new(),
    };

    let commit_message = if strict_semantic_mode {
        println!("[vora-git] Running Strict Semantic Differ & Lineage Tracking...");
        let mut old_snaps = Vec::new();
        let mut new_snaps = Vec::new();

        for change in &changes {
            let path_str = change.path.to_string_lossy().to_string();
            if let Some(old_bytes) = &change.old_content {
                if let Ok(old_str) = std::str::from_utf8(old_bytes) {
                    old_snaps.push(SemanticDiffer::create_file_snapshot(&path_str, old_str));
                }
            }
            if let Some(new_bytes) = &change.new_content {
                if let Ok(new_str) = std::str::from_utf8(new_bytes) {
                    new_snaps.push(SemanticDiffer::create_file_snapshot(&path_str, new_str));
                }
            }
        }

        let diff_report = SemanticDiffer::diff_project_snapshots(&old_snaps, &new_snaps);
        if diff_report.changes.is_empty() && diff_report.file_moves.is_empty() {
            println!("ℹ [vora-git] No strict semantic symbol changes or file moves detected.");
            return Ok(());
        }

        let task_id_str = task_id.unwrap_or_else(|| "TX_MANUAL".to_string());
        let payload = SemanticCommitPayload::build_semantic_commit(
            &diff_report.changes,
            rationale,
            &task_id_str,
        )
        .ok_or("Failed to construct semantic commit payload")?;

        payload.to_git_message()
    } else {
        println!("[vora-git] Analyzing AST structural changes via Tree-sitter...");
        rationale.to_git_message()
    };

    let oid = ssot.stage_and_commit(&commit_message)?;

    println!("[vora-git] AST semantic commit recorded successfully.");
    println!("[vora-git] Commit OID: {}", oid);
    println!("--- Committed Payload ---\n{}", commit_message);

    Ok(())
}
