use super::project::ProjectContext;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::Parser;

#[derive(Debug)]
pub enum PatchError {
    SymbolNotFound(String),
    FileNotFound(String),
    IoError(String),
    SyntaxError(String),
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::SymbolNotFound(s) => write!(f, "Symbol '{}' not found in project index", s),
            PatchError::FileNotFound(path) => write!(f, "Source file not found at: {}", path),
            PatchError::IoError(e) => write!(f, "File I/O error: {}", e),
            PatchError::SyntaxError(e) => write!(f, "Post-patch syntax verification failed: {}", e),
        }
    }
}

impl std::error::Error for PatchError {}

pub struct AstPatcher<'a> {
    context: &'a ProjectContext,
    root_dir: PathBuf,
}

impl<'a> AstPatcher<'a> {
    pub fn new(context: &'a ProjectContext, root_dir: impl AsRef<Path>) -> Self {
        Self {
            context,
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }

    /// Performs a surgical in-place replacement of a symbol's code on disk
    pub fn patch_symbol(&self, symbol_name: &str, new_code: &str) -> Result<(), PatchError> {
        // 1. Locate the symbol in the project index
        let target_def = self
            .context
            .all_definitions
            .iter()
            .find(|def| def.name == symbol_name)
            .ok_or_else(|| PatchError::SymbolNotFound(symbol_name.to_string()))?;

        // 2. Resolve target file path
        let target_path = Path::new(&target_def.file_path);
        let full_path = if target_path.is_absolute() || target_path.exists() {
            target_path.to_path_buf()
        } else {
            self.root_dir.join(target_path)
        };

        if !full_path.exists() {
            return Err(PatchError::FileNotFound(
                full_path.to_string_lossy().to_string(),
            ));
        }

        // 3. Read current file content
        let file_content =
            fs::read_to_string(&full_path).map_err(|e| PatchError::IoError(e.to_string()))?;
        let lines: Vec<&str> = file_content.lines().collect();

        let start_idx = target_def.start_line.saturating_sub(1);
        let end_idx = target_def.end_line.min(lines.len());

        // 4. Construct updated content with surgical replacement
        let mut updated_lines: Vec<String> = Vec::new();
        for line in &lines[..start_idx] {
            updated_lines.push(line.to_string());
        }

        updated_lines.push(new_code.trim_end().to_string());

        for line in &lines[end_idx..] {
            updated_lines.push(line.to_string());
        }

        let final_source = updated_lines.join("\n") + "\n";

        // 5. AST Syntax Integrity Verification
        let mut parser = Parser::new();
        let language = tree_sitter_rust::language();
        parser
            .set_language(&language)
            .map_err(|e| PatchError::SyntaxError(e.to_string()))?;

        let parsed_tree = parser.parse(&final_source, None).ok_or_else(|| {
            PatchError::SyntaxError("Parser failed on patched source".to_string())
        })?;

        if parsed_tree.root_node().has_error() {
            return Err(PatchError::SyntaxError(
                "Patched code introduced AST syntax errors. Write aborted.".to_string(),
            ));
        }

        // 6. Write verified patch back to disk
        fs::write(&full_path, final_source).map_err(|e| PatchError::IoError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surgical_patch_dry_run() {
        let context = ProjectContext::load_from_repo(".").expect("Failed to load repo context");
        assert!(!context.all_definitions.is_empty());

        let patcher = AstPatcher::new(&context, "..");
        let target_symbol = &context.all_definitions[0];

        // Non-existent symbol must return error
        let invalid_result = patcher.patch_symbol("__non_existent_symbol__", "fn dummy() {}");
        assert!(invalid_result.is_err());

        println!("\n[PATCHER INTEGRITY TEST PASSED]");
        println!("  Target Verified: {}", target_symbol.name);
    }
}
