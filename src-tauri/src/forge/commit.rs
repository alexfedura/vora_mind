use crate::forge::differ::{SemanticDiffReport, SemanticDiffer};
use crate::forge::index::CodeIndex;
use crate::forge::model::{SemanticChange, SnapshotId, WorkspaceId};
use crate::forge::overlay::CodeOverlayManager;
use crate::forge::patcher_v2::SurgicalPatcher;
use crate::forge::planner::ValidatedExecutionPlan;
use crate::forge::ssot::TrackedChange;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDelta {
    pub symbol_name: String,
    pub change_type: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDelta {
    pub file_path: String,
    pub symbol_deltas: Vec<SymbolDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRationale {
    pub intent: String,
    pub confidence_score: f32,
    pub file_deltas: Vec<FileDelta>,
}

impl CommitRationale {
    pub fn to_git_message(&self) -> String {
        let mut msg = format!("refactor(forge): {}\n\n", self.intent);
        msg.push_str("Semantic Impact Audit:\n");
        for file in &self.file_deltas {
            msg.push_str(&format!("- File: {}\n", file.file_path));
            for sym in &file.symbol_deltas {
                msg.push_str(&format!(
                    "  * [{}] {} - {}\n",
                    sym.change_type, sym.symbol_name, sym.detail
                ));
            }
        }
        msg.push_str(&format!("\nConfidence Score: {:.2}", self.confidence_score));
        msg
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstCommitPayload {
    pub rationale: CommitRationale,
    pub generated_message: String,
}

impl AstCommitPayload {
    pub fn build(changes: &[TrackedChange], rationale: CommitRationale) -> Option<Self> {
        if changes.is_empty() {
            return None;
        }
        let generated_message = rationale.to_git_message();
        Some(Self {
            rationale,
            generated_message,
        })
    }

    pub fn build_semantic_commit(
        changes: &[SemanticChange],
        rationale: CommitRationale,
        task_id: impl Into<String>,
    ) -> Option<SemanticCommitPayload> {
        SemanticCommitPayload::build_semantic_commit(changes, rationale, task_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCommitPayload {
    pub semantic_changes: Vec<SemanticChange>,
    pub rationale: CommitRationale,
    pub task_id: String,
}

impl SemanticCommitPayload {
    pub fn build_semantic_commit(
        changes: &[SemanticChange],
        rationale: CommitRationale,
        task_id: impl Into<String>,
    ) -> Option<Self> {
        if changes.is_empty() {
            return None;
        }
        Some(Self {
            semantic_changes: changes.to_vec(),
            rationale,
            task_id: task_id.into(),
        })
    }

    pub fn to_git_message(&self) -> String {
        format_semantic_git_message(self)
    }
}

pub fn format_semantic_git_message(payload: &SemanticCommitPayload) -> String {
    let mut msg = format!("refactor(semantic): {}\n\n", payload.rationale.intent);
    msg.push_str(&format!("Task-ID: {}\n", payload.task_id));
    msg.push_str("Semantic Mutations:\n");

    for change in &payload.semantic_changes {
        match change {
            SemanticChange::BodyChanged {
                symbol_id,
                old_body_hash,
                new_body_hash,
                ..
            } => {
                msg.push_str(&format!(
                    " - [BodyChanged] {} ({} -> {})\n",
                    symbol_id.as_str(),
                    &old_body_hash[..old_body_hash.len().min(7)],
                    &new_body_hash[..new_body_hash.len().min(7)]
                ));
            }
            SemanticChange::SignatureChanged {
                old_symbol_id,
                new_symbol_id,
                old_signature,
                new_signature,
                ..
            } => {
                msg.push_str(&format!(
                    " - [SignatureChanged] {} -> {}\n     old: {}\n     new: {}\n",
                    old_symbol_id.as_str(),
                    new_symbol_id.as_str(),
                    old_signature,
                    new_signature
                ));
            }
            SemanticChange::SymbolRenamed {
                old_name, new_name, ..
            } => {
                msg.push_str(&format!(
                    " - [SymbolRenamed] {} -> {}\n",
                    old_name, new_name
                ));
            }
            SemanticChange::SymbolAdded {
                symbol_id,
                signature,
                ..
            } => {
                msg.push_str(&format!(
                    " - [SymbolAdded] {} ({})\n",
                    symbol_id.as_str(),
                    signature
                ));
            }
            SemanticChange::SymbolRemoved { symbol_id, .. } => {
                msg.push_str(&format!(" - [SymbolRemoved] {}\n", symbol_id.as_str()));
            }
            _ => {
                msg.push_str(" - [OtherSemanticMutation]\n");
            }
        }
    }

    msg.push_str(&format!(
        "\nConfidence Score: {:.2}",
        payload.rationale.confidence_score
    ));
    msg
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPatch {
    pub file_path: String,
    pub symbol_id: String,
    pub new_symbol_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCommitResult {
    pub is_committed: bool,
    pub commit_message: String,
    pub patched_files: Vec<String>,
    pub diff_report: SemanticDiffReport,
    pub snapshot_id: Option<SnapshotId>,
}

pub struct ForgeTransactionManager<'a> {
    pub base_commit: String,
    pub task_id: String,
    pub overlay: &'a mut CodeOverlayManager,
    pub staged_base_files: HashMap<String, String>,
    pub staged_patches: Vec<TransactionPatch>,
}

impl<'a> ForgeTransactionManager<'a> {
    pub fn new(
        base_commit: impl Into<String>,
        task_id: impl Into<String>,
        overlay: &'a mut CodeOverlayManager,
    ) -> Self {
        Self {
            base_commit: base_commit.into(),
            task_id: task_id.into(),
            overlay,
            staged_base_files: HashMap::new(),
            staged_patches: Vec::new(),
        }
    }

    pub fn stage_base_file(&mut self, file_path: &str, content: &str) {
        self.staged_base_files
            .insert(file_path.to_string(), content.to_string());
    }

    pub fn stage_patch(&mut self, file_path: &str, symbol_id: &str, new_symbol_code: &str) {
        self.staged_patches.push(TransactionPatch {
            file_path: file_path.to_string(),
            symbol_id: symbol_id.to_string(),
            new_symbol_code: new_symbol_code.to_string(),
        });
    }

    pub fn validate_and_simulate(
        &mut self,
    ) -> Result<(SemanticDiffReport, HashMap<String, String>), String> {
        let mut old_snapshots = Vec::new();
        let mut mutated_contents: HashMap<String, String> = HashMap::new();

        for (path, content) in &self.staged_base_files {
            old_snapshots.push(SemanticDiffer::create_file_snapshot(path, content));
            mutated_contents.insert(path.clone(), content.clone());
        }

        for patch in &self.staged_patches {
            let current_content = mutated_contents.get(&patch.file_path).ok_or_else(|| {
                format!(
                    "Base file content for '{}' not staged in transaction",
                    patch.file_path
                )
            })?;

            let patch_res = SurgicalPatcher::apply_patch_to_source(
                &patch.file_path,
                current_content,
                &patch.symbol_id,
                &patch.new_symbol_code,
                true,
            )?;

            if !patch_res.is_syntax_valid {
                return Err(format!(
                    "Syntax validation failed for patch on symbol '{}' in '{}'",
                    patch.symbol_id, patch.file_path
                ));
            }

            let start_byte = patch_res.original_byte_range.0;
            let end_byte = patch_res.original_byte_range.1;

            if start_byte > current_content.len() || end_byte > current_content.len() {
                return Err(format!(
                    "Range out of bounds for symbol '{}' in '{}'",
                    patch.symbol_id, patch.file_path
                ));
            }

            let before = &current_content[..start_byte];
            let after = &current_content[end_byte..];
            let new_content = format!("{}{}{}", before, patch.new_symbol_code, after);

            mutated_contents.insert(patch.file_path.clone(), new_content);
        }

        let mut new_snapshots = Vec::new();
        for (path, content) in &mutated_contents {
            new_snapshots.push(SemanticDiffer::create_file_snapshot(path, content));
        }

        let diff_report = SemanticDiffer::diff_project_snapshots(&old_snapshots, &new_snapshots);
        Ok((diff_report, mutated_contents))
    }

    pub fn commit_transaction(
        mut self,
        index: &mut CodeIndex,
        intent: &str,
    ) -> Result<TransactionCommitResult, String> {
        let (diff_report, mutated_contents) = self.validate_and_simulate()?;

        let mut file_deltas = Vec::new();
        for path in mutated_contents.keys() {
            file_deltas.push(FileDelta {
                file_path: path.clone(),
                symbol_deltas: vec![SymbolDelta {
                    symbol_name: "TransactionMutation".to_string(),
                    change_type: "MUTATION".to_string(),
                    detail: format!("Applied {} surgical patches", self.staged_patches.len()),
                }],
            });
        }

        let rationale = CommitRationale {
            intent: intent.to_string(),
            confidence_score: 1.0,
            file_deltas,
        };

        // اتصال مستقیم به SemanticCommitPayload به عنوان SSOT
        let semantic_payload = SemanticCommitPayload::build_semantic_commit(
            &diff_report.changes,
            rationale.clone(),
            self.task_id.clone(),
        );

        let final_commit_msg = semantic_payload
            .map(|p| p.to_git_message())
            .unwrap_or_else(|| rationale.to_git_message());

        let workspace_id: WorkspaceId = index
            .connection()
            .query_row(
                "SELECT workspace_id FROM workspaces ORDER BY rowid DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| {
                index
                    .get_or_create_workspace("default_workspace")
                    .expect("Failed to get or create fallback workspace")
            });

        let files_tuple: Vec<(&str, &[u8])> = mutated_contents
            .iter()
            .map(|(p, c)| (p.as_str(), c.as_bytes()))
            .collect();

        let new_snapshot_id = index
            .commit_identity_snapshot(&workspace_id, None, &files_tuple)
            .map_err(|e| e.to_string())?;

        let staged_files: Vec<String> = mutated_contents.keys().cloned().collect();

        for (path, content) in mutated_contents {
            let file_id = index.get_or_create_file_id(&path);
            self.overlay.stage_file_mutation(file_id, &path, content);
        }

        let _ = index.connection().execute(
            "UPDATE workspaces SET current_snapshot_id = ?1 WHERE workspace_id = ?2",
            rusqlite::params![&new_snapshot_id.0, &workspace_id.0],
        );

        // A4: ممیزی و راستی‌آزمایی قطعی پس از تعهد ترکنش (Full Post-Commit Invariant Verification)
        Self::verify_post_commit_invariants(index, &workspace_id, &new_snapshot_id, &staged_files)?;

        Ok(TransactionCommitResult {
            is_committed: true,
            commit_message: final_commit_msg,
            patched_files: staged_files,
            diff_report,
            snapshot_id: Some(new_snapshot_id),
        })
    }

    /// A4: راستی‌آزمایی پایدار بودن اسنپ‌شات و تداوم هویت شجره در پایگاه داده
    pub fn verify_post_commit_invariants(
        index: &CodeIndex,
        workspace_id: &WorkspaceId,
        snapshot_id: &SnapshotId,
        staged_files: &[String],
    ) -> Result<(), String> {
        let conn = index.connection();

        // ۱. بررسی ثبت اسنپ‌شات در جدول workspaces
        let current_snap: String = conn
            .query_row(
                "SELECT current_snapshot_id FROM workspaces WHERE workspace_id = ?1",
                [&workspace_id.0],
                |r| r.get(0),
            )
            .map_err(|e| format!("Post-commit invariant failed: Cannot find workspace: {}", e))?;

        if current_snap != snapshot_id.0 {
            return Err(format!(
                "Post-commit invariant failed: Workspace snapshot mismatch (expected '{}', got '{}')",
                snapshot_id.0, current_snap
            ));
        }

        // ۲. اطمینان از وجود فایل‌های تغییریافته درون اسنپ‌شات
        for file in staged_files {
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM snapshot_files WHERE snapshot_id = ?1 AND path_at_snapshot = ?2",
                    rusqlite::params![&snapshot_id.0, file],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if !exists {
                return Err(format!(
                    "Post-commit invariant failed: File '{}' missing in snapshot '{}'",
                    file, snapshot_id.0
                ));
            }
        }

        Ok(())
    }

    /// A3 & A5: کامیت با اعتبارسنجی قطعی پلن معتبر شده (Validated Plan Execution Gate)
    pub fn commit_validated_transaction(
        self,
        index: &mut CodeIndex,
        intent: &str,
        validated_plan: &ValidatedExecutionPlan,
    ) -> Result<TransactionCommitResult, String> {
        if validated_plan.total_steps == 0 || validated_plan.verified_mutations_count == 0 {
            return Err(
                "ValidatedPlan rejection: No verified steps to commit in atomic transaction"
                    .to_string(),
            );
        }
        self.commit_transaction(index, intent)
    }

    pub fn rollback(self) {
        self.overlay.discard_transaction();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_transaction_success_and_rollback() {
        let mut overlay = CodeOverlayManager::new("base_commit_1", "task_001");
        let mut tx_manager =
            ForgeTransactionManager::new("base_commit_1", "task_001", &mut overlay);

        let file_path = "src/math/calc.rs";
        let base_content = "pub fn calculate_tax(amount: f64) -> f64 {\n    amount * 0.09\n}";

        tx_manager.stage_base_file(file_path, base_content);
        tx_manager.stage_patch(
            file_path,
            "calculate_tax",
            "pub fn calculate_tax(amount: f64) -> f64 {\n    amount * 0.10\n}",
        );

        let sim_result = tx_manager.validate_and_simulate();
        assert!(sim_result.is_ok());

        let (diff_report, mutated_map) = sim_result.unwrap();
        assert_eq!(mutated_map.len(), 1);
        assert!(!diff_report.changes.is_empty());

        let mut index = CodeIndex::open_in_memory().unwrap();
        let commit_res = tx_manager
            .commit_transaction(&mut index, "Increase tax rate to 10%")
            .unwrap();

        assert!(commit_res.is_committed);
        assert_eq!(commit_res.patched_files.len(), 1);
        assert!(commit_res.snapshot_id.is_some());
    }
}
