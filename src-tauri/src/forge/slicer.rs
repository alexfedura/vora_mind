use super::project::ProjectContext;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSlice {
    pub target_symbol: String,
    pub target_file: String,
    pub target_code: String,
    pub related_definitions: Vec<String>,
    pub global_skeleton: String,
}

pub struct ScopeSlicer<'a> {
    context: &'a ProjectContext,
    root_dir: PathBuf,
}

impl<'a> ScopeSlicer<'a> {
    pub fn new(context: &'a ProjectContext, root_dir: impl AsRef<Path>) -> Self {
        Self {
            context,
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }

    /// استخراج قطعه کد دقیق و کم‌حجم برای یک نماد مشخص
    pub fn slice_for_symbol(&self, symbol_name: &str) -> Option<CodeSlice> {
        // ۱. پیدا کردن نماد بر اساس نام یا CanonicalEntityId
        let target_def = self.context.all_definitions.iter().find(|def| {
            def.name == symbol_name
                || def.canonical_entity_id.as_ref().map(|c| c.as_str()) == Some(symbol_name)
        })?;

        // ۲. خواندن کد منبع فایل از روی دیسک
        let target_path = Path::new(&target_def.file_path);
        let full_path = if target_path.is_absolute() || target_path.exists() {
            target_path.to_path_buf()
        } else {
            self.root_dir.join(target_path)
        };

        let file_content = fs::read_to_string(&full_path)
            .or_else(|_| fs::read_to_string(target_path))
            .ok()?;

        let lines: Vec<&str> = file_content.lines().collect();

        let start_idx = target_def.start_line.saturating_sub(1).min(lines.len());
        let end_idx = target_def.end_line.min(lines.len());
        let target_code = if start_idx < end_idx && !lines.is_empty() {
            lines[start_idx..end_idx].join("\n")
        } else {
            String::new()
        };

        // ۳. استخراج تعاریف وابسته‌های مستقیم
        let mut related_definitions = Vec::new();
        for out_ref in &target_def.outbound_references {
            if let Some(defs) = self.context.graph.definitions.get(out_ref) {
                for d in defs {
                    if d.name != target_def.name {
                        related_definitions
                            .push(format!("// {} ({})\n{}", d.file_path, d.kind, d.signature));
                    }
                }
            }
        }

        // ۴. خروجی اسکلت سراسری پروژه
        let global_skeleton = self.context.generate_global_skeleton();

        Some(CodeSlice {
            target_symbol: target_def.name.clone(),
            target_file: target_def.file_path.clone(),
            target_code,
            related_definitions,
            global_skeleton,
        })
    }

    /// P11: استخراج کانتکست زنجیره‌ای برای فراخوانی‌های چند لایه (Cascading Context Assembly)
    pub fn slice_cascading_context(
        &self,
        target_symbol: &str,
        root_mutated_symbol: &str,
        cascade_depth: usize,
    ) -> Option<CodeSlice> {
        let mut base_slice = self.slice_for_symbol(target_symbol)?;

        if let Some(root_def) = self
            .context
            .all_definitions
            .iter()
            .find(|d| d.name == root_mutated_symbol)
        {
            base_slice.related_definitions.insert(
                0,
                format!(
                    "// [Root Mutation Target - Depth {} Origin] {} ({})\n{}",
                    cascade_depth, root_def.file_path, root_def.kind, root_def.signature
                ),
            );
        }

        Some(base_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_slicing_real_symbol() {
        let context = ProjectContext::load_from_repo(".").expect("Failed to load repo");
        assert!(
            !context.all_definitions.is_empty(),
            "Project must contain indexed symbols"
        );

        let slicer = ScopeSlicer::new(&context, "..");

        let sample_symbol = &context.all_definitions[0];
        let slice = slicer.slice_for_symbol(&sample_symbol.name);

        assert!(
            slice.is_some(),
            "Symbol '{}' failed to slice from disk",
            sample_symbol.name
        );

        let s = slice.unwrap();
        assert_eq!(s.target_symbol, sample_symbol.name);
        assert!(
            !s.target_code.is_empty(),
            "Target sliced code must not be empty"
        );
    }
}
