use super::ast_engine::{AstEngine, ParsedFilePayload};
use super::lineage::LineageEngine;
use super::model::{
    ContentHash, FileId, LineageResolution, OverlayId, SemanticChange, VersionedCodeState,
};
use std::collections::HashMap;

pub struct CodeOverlayManager {
    active_state: VersionedCodeState,
    cached_trees: HashMap<FileId, ParsedFilePayload>,
}

impl CodeOverlayManager {
    pub fn new(base_commit: impl Into<String>, transaction_id: impl Into<String>) -> Self {
        let tx_id = transaction_id.into();
        Self {
            active_state: VersionedCodeState {
                overlay_id: OverlayId(format!("overlay_{}", tx_id)),
                base_commit: base_commit.into(),
                transaction_id: tx_id,
                expected_content_hash: ContentHash::new(""),
                dirty_files: HashMap::new(),
            },
            cached_trees: HashMap::new(),
        }
    }

    /// ثبت تغییرات موقت فایل در اورلی و پارس درختی بدون نوشتن روی دیسک
    pub fn stage_file_mutation(
        &mut self,
        file_id: FileId,
        file_path: &str,
        new_content: String,
    ) -> &ParsedFilePayload {
        let parsed = AstEngine::parse_file_semantics(file_id.clone(), file_path, &new_content);
        self.active_state
            .dirty_files
            .insert(file_id.clone(), new_content);
        self.cached_trees.insert(file_id.clone(), parsed);
        self.cached_trees.get(&file_id).unwrap()
    }

    /// محاسبه تفاوت‌های معنایی بین وضعیت پایه و اورلی جاری بر پایه خروجی موتور تبار
    pub fn compute_semantic_deltas(
        &self,
        file_id: &FileId,
        base_symbols: &[super::model::SymbolIdentity],
    ) -> Vec<SemanticChange> {
        let mut deltas = Vec::new();

        if let Some(overlay_parsed) = self.cached_trees.get(file_id) {
            let lineage_engine = LineageEngine::default();
            let resolutions =
                lineage_engine.resolve_lineages(base_symbols, &overlay_parsed.symbols);

            for res in resolutions {
                match res {
                    LineageResolution::Matched {
                        lineage_id,
                        old_symbol_id,
                        new_symbol_id,
                        ..
                    } => {
                        let old_sym = base_symbols.iter().find(|s| s.id == old_symbol_id);
                        let new_sym = overlay_parsed
                            .symbols
                            .iter()
                            .find(|s| s.id == new_symbol_id);

                        if let (Some(old_s), Some(new_s)) = (old_sym, new_sym) {
                            if old_s.name != new_s.name {
                                deltas.push(SemanticChange::SymbolRenamed {
                                    old_symbol_id: old_s.id.clone(),
                                    new_symbol_id: new_s.id.clone(),
                                    lineage_id: lineage_id.clone(),
                                    old_name: old_s.name.clone(),
                                    new_name: new_s.name.clone(),
                                });
                            }

                            if old_s.canonical_signature != new_s.canonical_signature {
                                deltas.push(SemanticChange::SignatureChanged {
                                    old_symbol_id: old_s.id.clone(),
                                    new_symbol_id: new_s.id.clone(),
                                    lineage_id: lineage_id.clone(),
                                    old_signature: old_s.canonical_signature.clone(),
                                    new_signature: new_s.canonical_signature.clone(),
                                });
                            } else if old_s.body_hash != new_s.body_hash {
                                deltas.push(SemanticChange::BodyChanged {
                                    symbol_id: new_s.id.clone(),
                                    lineage_id: lineage_id.clone(),
                                    old_body_hash: old_s.body_hash.clone(),
                                    new_body_hash: new_s.body_hash.clone(),
                                });
                            }

                            if old_s.visibility != new_s.visibility {
                                deltas.push(SemanticChange::VisibilityChanged {
                                    symbol_id: new_s.id.clone(),
                                    lineage_id,
                                    old_vis: old_s.visibility,
                                    new_vis: new_s.visibility,
                                });
                            }
                        }
                    }
                    LineageResolution::New {
                        lineage_id,
                        new_symbol_id,
                    } => {
                        if let Some(new_s) = overlay_parsed
                            .symbols
                            .iter()
                            .find(|s| s.id == new_symbol_id)
                        {
                            deltas.push(SemanticChange::SymbolAdded {
                                symbol_id: new_s.id.clone(),
                                lineage_id,
                                signature: new_s.canonical_signature.clone(),
                            });
                        }
                    }
                    LineageResolution::Removed {
                        lineage_id,
                        old_symbol_id,
                    } => {
                        deltas.push(SemanticChange::SymbolRemoved {
                            symbol_id: old_symbol_id,
                            lineage_id,
                        });
                    }
                    LineageResolution::Ambiguous { .. } => {
                        // در صورت عدم قطعیت، از تولید تغییرات کاذب جلوگیری می‌شود
                    }
                }
            }
        }

        deltas
    }

    pub fn state(&self) -> &VersionedCodeState {
        &self.active_state
    }

    /// لغو و دور ریختن کامل تغییرات اورلی (Rollback)
    pub fn discard_transaction(&mut self) {
        self.active_state.dirty_files.clear();
        self.cached_trees.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_isolation_and_mutation() {
        let mut overlay = CodeOverlayManager::new("main_head", "tx_001");
        let file_id = FileId::new("file_test_01");

        let code = r#"
            pub fn compute_sum(a: usize, b: usize) -> usize {
                a + b
            }
        "#;

        let parsed = overlay.stage_file_mutation(file_id.clone(), "src/math.rs", code.to_string());
        assert_eq!(parsed.symbols.len(), 1);
        assert_eq!(parsed.symbols[0].name, "compute_sum");
        assert!(overlay.state().dirty_files.contains_key(&file_id));

        overlay.discard_transaction();
        assert!(overlay.state().dirty_files.is_empty());
    }
}
