use super::ast_engine::AstEngine;
use super::model::FileId;
use super::overlay::CodeOverlayManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    pub file_path: String,
    pub symbol_id: String,
    pub original_byte_range: (usize, usize),
    pub patched_byte_range: (usize, usize),
    pub is_syntax_valid: bool,
    pub applied_to_disk: bool,
}

pub struct SurgicalPatcher;

impl SurgicalPatcher {
    /// اعمال جراحی دقیق پچ روی یک نماد خاص در فایل و ذخیره روی دیسک (یا حالت پیش‌نمایش dry_run)
    pub fn apply_patch(
        file_path: &str,
        symbol_id: &str,
        new_symbol_code: &str,
        dry_run: bool,
    ) -> Result<PatchResult, String> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("Target file does not exist: {}", file_path));
        }

        let original_content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

        let res = Self::apply_patch_to_source(
            file_path,
            &original_content,
            symbol_id,
            new_symbol_code,
            dry_run,
        )?;

        if !dry_run && res.is_syntax_valid {
            let start_byte = res.original_byte_range.0;
            let end_byte = res.original_byte_range.1;
            let mut patched_content =
                String::with_capacity(original_content.len() + new_symbol_code.len());
            patched_content.push_str(&original_content[..start_byte]);
            patched_content.push_str(new_symbol_code);
            patched_content.push_str(&original_content[end_byte..]);

            fs::write(path, &patched_content).map_err(|e| {
                format!("Failed to write patched content to '{}': {}", file_path, e)
            })?;
        }

        Ok(res)
    }

    /// اعمال جراحی پچ بر روی محتوای خام متنی در حافظه بدون وابستگی به فایل فیزیکی دیسک
    pub fn apply_patch_to_source(
        file_path: &str,
        source_content: &str,
        symbol_id: &str,
        new_symbol_code: &str,
        _dry_run: bool,
    ) -> Result<PatchResult, String> {
        let parsed = AstEngine::parse_file_syntax(file_path, source_content);
        let target_sym = parsed
            .symbols
            .iter()
            .find(|s| s.id == symbol_id || s.name == symbol_id || s.scope_path == symbol_id)
            .ok_or_else(|| format!("Symbol '{}' not found in file '{}'", symbol_id, file_path))?;

        let start_byte = target_sym.start_byte;
        let end_byte = target_sym.end_byte;

        if start_byte > source_content.len()
            || end_byte > source_content.len()
            || start_byte > end_byte
        {
            return Err(format!(
                "Invalid byte offset range ({}-{}) for symbol",
                start_byte, end_byte
            ));
        }

        let mut patched_content =
            String::with_capacity(source_content.len() + new_symbol_code.len());
        patched_content.push_str(&source_content[..start_byte]);
        patched_content.push_str(new_symbol_code);
        patched_content.push_str(&source_content[end_byte..]);

        let mut engine = AstEngine::for_file(file_path);
        let syntax_valid = match engine.parse_file_tree(&patched_content) {
            Some(tree) => !tree.root_node().has_error(),
            None => false,
        };

        if !syntax_valid {
            return Err(
                "Syntax validation failed: The provided patch produces AST syntax errors"
                    .to_string(),
            );
        }

        let new_end_byte = start_byte + new_symbol_code.len();

        Ok(PatchResult {
            file_path: file_path.to_string(),
            symbol_id: target_sym.id.clone(),
            original_byte_range: (start_byte, end_byte),
            patched_byte_range: (start_byte, new_end_byte),
            is_syntax_valid: syntax_valid,
            applied_to_disk: false,
        })
    }

    /// P11: بازنگری و سیم‌کشی مجدد خودکار ایمپورت‌ها هنگام تغییر نام نماد (Import & Namespace Re-wiring)
    pub fn apply_import_rewiring(
        file_path: &str,
        source_content: &str,
        old_symbol_name: &str,
        new_symbol_name: &str,
    ) -> Result<String, String> {
        let mut mutated_lines = Vec::new();

        for line in source_content.lines() {
            let trimmed = line.trim_start();
            let is_import_line = trimmed.starts_with("use ")
                || trimmed.starts_with("import ")
                || trimmed.starts_with("pub use ")
                || trimmed.starts_with("from ");

            if is_import_line && line.contains(old_symbol_name) {
                let mut rewired_line = String::new();
                let mut last_idx = 0;

                for (idx, _) in line.match_indices(old_symbol_name) {
                    if idx < last_idx {
                        continue;
                    }

                    // بررسی مرز کلمه قبل
                    let is_start_boundary = idx == 0
                        || !line[..idx]
                            .chars()
                            .next_back()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_');
                    // بررسی مرز کلمه بعد
                    let after_idx = idx + old_symbol_name.len();
                    let is_end_boundary = after_idx == line.len()
                        || !line[after_idx..]
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_');

                    if is_start_boundary && is_end_boundary {
                        rewired_line.push_str(&line[last_idx..idx]);
                        rewired_line.push_str(new_symbol_name);
                        last_idx = after_idx;
                    }
                }
                rewired_line.push_str(&line[last_idx..]);
                mutated_lines.push(rewired_line);
            } else {
                mutated_lines.push(line.to_string());
            }
        }

        let result_content = mutated_lines.join("\n");

        let mut engine = AstEngine::for_file(file_path);
        let syntax_valid = match engine.parse_file_tree(&result_content) {
            Some(tree) => !tree.root_node().has_error(),
            None => false,
        };

        if !syntax_valid {
            return Err(format!(
                "Import rewiring failed: Produced AST syntax errors in '{}'",
                file_path
            ));
        }

        Ok(result_content)
    }

    /// اعمال مستقیم جراحی پچ بر روی یک مسیر ریشه مشخص (مانند دایرکتوری Candidate Workspace)
    pub fn apply_patch_to_root(
        root_dir: &Path,
        rel_file_path: &str,
        symbol_id: &str,
        new_symbol_code: &str,
    ) -> Result<PatchResult, String> {
        let full_path = root_dir.join(rel_file_path);
        if !full_path.exists() {
            return Err(format!(
                "Target file does not exist in workspace: {:?}",
                full_path
            ));
        }

        let original_content = fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read file '{:?}': {}", full_path, e))?;

        let res = Self::apply_patch_to_source(
            rel_file_path,
            &original_content,
            symbol_id,
            new_symbol_code,
            false,
        )?;

        if res.is_syntax_valid {
            let start_byte = res.original_byte_range.0;
            let end_byte = res.original_byte_range.1;
            let mut patched_content =
                String::with_capacity(original_content.len() + new_symbol_code.len());
            patched_content.push_str(&original_content[..start_byte]);
            patched_content.push_str(new_symbol_code);
            patched_content.push_str(&original_content[end_byte..]);

            fs::write(&full_path, &patched_content).map_err(|e| {
                format!(
                    "Failed to write patched candidate file '{:?}': {}",
                    full_path, e
                )
            })?;
        }

        Ok(res)
    }

    /// اعمال جراحی پچ بر روی وضعیت اورلی درون حافظه بدون لمس دیسک (Semantic Core v1.0)
    pub fn apply_patch_to_overlay(
        overlay: &mut CodeOverlayManager,
        file_id: FileId,
        file_path: &str,
        base_content: &str,
        symbol_id: &str,
        new_symbol_code: &str,
    ) -> Result<PatchResult, String> {
        let current_content = overlay
            .state()
            .dirty_files
            .get(&file_id)
            .cloned()
            .unwrap_or_else(|| base_content.to_string());

        let parsed = AstEngine::parse_file_semantics(file_id.clone(), file_path, &current_content);
        let target_sym = parsed
            .symbols
            .iter()
            .find(|s| s.id.0 == symbol_id || s.name == symbol_id || s.qualified_scope == symbol_id)
            .ok_or_else(|| format!("Symbol '{}' not found in file '{}'", symbol_id, file_path))?;

        let start_byte = target_sym.start_byte;
        let end_byte = target_sym.end_byte;

        if start_byte > current_content.len()
            || end_byte > current_content.len()
            || start_byte > end_byte
        {
            return Err(format!(
                "Invalid byte offset range ({}-{}) for symbol",
                start_byte, end_byte
            ));
        }

        let mut patched_content =
            String::with_capacity(current_content.len() + new_symbol_code.len());
        patched_content.push_str(&current_content[..start_byte]);
        patched_content.push_str(new_symbol_code);
        patched_content.push_str(&current_content[end_byte..]);

        let mut engine = AstEngine::for_file(file_path);
        let syntax_valid = match engine.parse_file_tree(&patched_content) {
            Some(tree) => !tree.root_node().has_error(),
            None => false,
        };

        if !syntax_valid {
            return Err(
                "Syntax validation failed: The patched overlay content contains AST errors"
                    .to_string(),
            );
        }

        overlay.stage_file_mutation(file_id, file_path, patched_content);

        let new_end_byte = start_byte + new_symbol_code.len();

        Ok(PatchResult {
            file_path: file_path.to_string(),
            symbol_id: target_sym.id.0.clone(),
            original_byte_range: (start_byte, end_byte),
            patched_byte_range: (start_byte, new_end_byte),
            is_syntax_valid: syntax_valid,
            applied_to_disk: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surgical_patch_in_overlay() {
        let mut overlay = CodeOverlayManager::new("base_commit", "task_patch_01");
        let file_id = FileId::new("file_pricing");
        let base_code = r#"
pub fn get_discount() -> f64 {
    0.05
}

pub fn get_tax() -> f64 {
    0.09
}
"#;

        let new_discount_fn = r#"pub fn get_discount() -> f64 {
    0.15
}"#;

        let result = SurgicalPatcher::apply_patch_to_overlay(
            &mut overlay,
            file_id.clone(),
            "src/pricing.rs",
            base_code,
            "get_discount",
            new_discount_fn,
        )
        .expect("Overlay patch failed");

        assert!(result.is_syntax_valid);
        assert!(!result.applied_to_disk);
        assert!(overlay.state().dirty_files.contains_key(&file_id));

        let mutated_code = overlay.state().dirty_files.get(&file_id).unwrap();
        assert!(mutated_code.contains("0.15"));
        assert!(mutated_code.contains("get_tax"));
    }
}
