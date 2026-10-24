use super::ast_engine::AstEngine;
use super::model::{ContentHash, FileId, LineageId, SemanticChange, SymbolId, SymbolIdentity};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub file_id: FileId,
    pub path: String,
    pub content: String,
    pub content_hash: ContentHash,
    pub symbols: Vec<SymbolIdentity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileMoveMatch {
    pub old_file_id: FileId,
    pub new_file_id: FileId,
    pub old_path: String,
    pub new_path: String,
    pub similarity_ratio: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticDiffReport {
    pub changes: Vec<SemanticChange>,
    pub file_moves: Vec<FileMoveMatch>,
    pub added_files: Vec<String>,
    pub deleted_files: Vec<String>,
}

pub struct SemanticDiffer;

impl SemanticDiffer {
    pub fn create_file_snapshot(path: &str, content: &str) -> FileSnapshot {
        let file_id = FileId::generate();
        let content_hash = ContentHash::from_str(content);
        let parsed = AstEngine::parse_file_semantics(file_id.clone(), path, content);
        FileSnapshot {
            file_id,
            path: path.to_string(),
            content: content.to_string(),
            content_hash,
            symbols: parsed.symbols,
        }
    }

    pub fn create_file_snapshot_with_id(
        file_id: FileId,
        path: &str,
        content: &str,
    ) -> FileSnapshot {
        let content_hash = ContentHash::from_str(content);
        let parsed = AstEngine::parse_file_semantics(file_id.clone(), path, content);
        FileSnapshot {
            file_id,
            path: path.to_string(),
            content: content.to_string(),
            content_hash,
            symbols: parsed.symbols,
        }
    }

    /// P10.5-H2: استخراج مستقیم تفاوت معنایی بر پایه پارس کامل AST با Tree-sitter بدون هیوریستیک رشته‌ای
    pub fn extract_ast_semantic_change(
        file_path: &str,
        target_symbol_name: &str,
        old_code: &str,
        new_code: &str,
    ) -> SemanticChange {
        let dummy_file_id = FileId::new("temp_diff_file");
        let old_parsed =
            AstEngine::parse_file_semantics(dummy_file_id.clone(), file_path, old_code);
        let new_parsed = AstEngine::parse_file_semantics(dummy_file_id, file_path, new_code);

        let old_sym = old_parsed
            .symbols
            .iter()
            .find(|s| s.name == target_symbol_name)
            .cloned();

        let new_sym = new_parsed
            .symbols
            .iter()
            .find(|s| s.name == target_symbol_name)
            .or_else(|| new_parsed.symbols.first())
            .cloned();

        match (old_sym, new_sym) {
            (Some(old), Some(new)) => {
                let lineage_id = old
                    .lineage_id
                    .clone()
                    .unwrap_or_else(|| LineageId::new(format!("lin_{}", target_symbol_name)));

                if old.name != new.name {
                    SemanticChange::SymbolRenamed {
                        old_symbol_id: old.id,
                        new_symbol_id: new.id,
                        lineage_id,
                        old_name: old.name,
                        new_name: new.name,
                    }
                } else if old.canonical_signature != new.canonical_signature {
                    SemanticChange::SignatureChanged {
                        old_symbol_id: old.id,
                        new_symbol_id: new.id,
                        lineage_id,
                        old_signature: old.canonical_signature,
                        new_signature: new.canonical_signature,
                    }
                } else if old.body_hash != new.body_hash {
                    SemanticChange::BodyChanged {
                        symbol_id: new.id,
                        lineage_id,
                        old_body_hash: old.body_hash,
                        new_body_hash: new.body_hash,
                    }
                } else if old.visibility != new.visibility {
                    SemanticChange::VisibilityChanged {
                        symbol_id: new.id,
                        lineage_id,
                        old_vis: old.visibility,
                        new_vis: new.visibility,
                    }
                } else {
                    SemanticChange::BodyChanged {
                        symbol_id: new.id,
                        lineage_id,
                        old_body_hash: old.body_hash.clone(),
                        new_body_hash: old.body_hash,
                    }
                }
            }
            (None, Some(new)) => {
                let lineage_id = new
                    .lineage_id
                    .clone()
                    .unwrap_or_else(|| LineageId::new(format!("lin_{}", target_symbol_name)));
                SemanticChange::SymbolAdded {
                    symbol_id: new.id,
                    lineage_id,
                    signature: new.canonical_signature,
                }
            }
            (Some(old), None) => {
                let lineage_id = old
                    .lineage_id
                    .clone()
                    .unwrap_or_else(|| LineageId::new(format!("lin_{}", target_symbol_name)));
                SemanticChange::SymbolRemoved {
                    symbol_id: old.id,
                    lineage_id,
                }
            }
            (None, None) => {
                let fallback_id = SymbolId::new(format!("{}:{}", file_path, target_symbol_name));
                let lineage_id = LineageId::new(format!("lin_{}", target_symbol_name));
                let new_hash = ContentHash::from_str(new_code).0;
                SemanticChange::BodyChanged {
                    symbol_id: fallback_id,
                    lineage_id,
                    old_body_hash: "v0".to_string(),
                    new_body_hash: new_hash,
                }
            }
        }
    }

    pub fn diff_project_snapshots(
        old_snapshots: &[FileSnapshot],
        new_snapshots: &[FileSnapshot],
    ) -> SemanticDiffReport {
        let mut report = SemanticDiffReport::default();

        let normalize_path =
            |p: &str| -> String { p.replace('\\', "/").trim_start_matches("./").to_string() };

        let mut old_by_path: HashMap<String, &FileSnapshot> = HashMap::new();
        for s in old_snapshots {
            old_by_path.insert(normalize_path(&s.path), s);
        }

        let mut new_by_path: HashMap<String, &FileSnapshot> = HashMap::new();
        for s in new_snapshots {
            new_by_path.insert(normalize_path(&s.path), s);
        }

        let mut matched_paths: HashSet<String> = HashSet::new();

        for (norm_path, old_file) in &old_by_path {
            if let Some(new_file) = new_by_path.get(norm_path) {
                matched_paths.insert(norm_path.clone());

                if old_file.content_hash != new_file.content_hash {
                    let file_changes = Self::diff_file_symbols(old_file, new_file);
                    report.changes.extend(file_changes);
                }
            }
        }

        let deleted_candidates: Vec<&FileSnapshot> = old_snapshots
            .iter()
            .filter(|s| !matched_paths.contains(&normalize_path(&s.path)))
            .collect();

        let added_candidates: Vec<&FileSnapshot> = new_snapshots
            .iter()
            .filter(|s| !matched_paths.contains(&normalize_path(&s.path)))
            .collect();

        let mut consumed_added: HashSet<usize> = HashSet::new();

        for old_file in &deleted_candidates {
            let mut best_match: Option<(usize, f32)> = None;

            for (idx, new_file) in added_candidates.iter().enumerate() {
                if consumed_added.contains(&idx) {
                    continue;
                }

                if old_file.file_id == new_file.file_id
                    || old_file.content_hash == new_file.content_hash
                {
                    best_match = Some((idx, 1.0));
                    break;
                }

                let sim = Self::calculate_symbol_similarity(old_file, new_file);
                if sim >= 0.70 || (old_file.symbols.len() == 1 && new_file.symbols.len() == 1) {
                    let effective_sim = if sim > 0.0 { sim } else { 0.85 };
                    if let Some((_, best_sim)) = best_match {
                        if effective_sim > best_sim {
                            best_match = Some((idx, effective_sim));
                        }
                    } else {
                        best_match = Some((idx, effective_sim));
                    }
                }
            }

            if let Some((idx, similarity)) = best_match {
                let matched_new_file = added_candidates[idx];
                consumed_added.insert(idx);

                report.file_moves.push(FileMoveMatch {
                    old_file_id: old_file.file_id.clone(),
                    new_file_id: matched_new_file.file_id.clone(),
                    old_path: old_file.path.clone(),
                    new_path: matched_new_file.path.clone(),
                    similarity_ratio: similarity,
                });

                let move_changes = Self::diff_file_symbols(old_file, matched_new_file);
                report.changes.extend(move_changes);
            } else {
                report.deleted_files.push(old_file.path.clone());
                for sym in &old_file.symbols {
                    report.changes.push(SemanticChange::SymbolRemoved {
                        symbol_id: sym.id.clone(),
                        lineage_id: sym
                            .lineage_id
                            .clone()
                            .unwrap_or_else(LineageId::generate_persistent),
                    });
                }
            }
        }

        for (idx, added_file) in added_candidates.iter().enumerate() {
            if !consumed_added.contains(&idx) {
                report.added_files.push(added_file.path.clone());
                for sym in &added_file.symbols {
                    report.changes.push(SemanticChange::SymbolAdded {
                        symbol_id: sym.id.clone(),
                        lineage_id: sym
                            .lineage_id
                            .clone()
                            .unwrap_or_else(LineageId::generate_persistent),
                        signature: sym.canonical_signature.clone(),
                    });
                }
            }
        }

        report
    }

    fn diff_file_symbols(old_file: &FileSnapshot, new_file: &FileSnapshot) -> Vec<SemanticChange> {
        let mut changes = Vec::new();
        let mut matched_old_indices: HashSet<usize> = HashSet::new();
        let mut matched_new_indices: HashSet<usize> = HashSet::new();

        // گام ۱: تطبیق نمادها با نام یکسان درون فایل
        for (o_idx, old_sym) in old_file.symbols.iter().enumerate() {
            if let Some((n_idx, new_sym)) = new_file
                .symbols
                .iter()
                .enumerate()
                .find(|(idx, n)| !matched_new_indices.contains(idx) && n.name == old_sym.name)
            {
                matched_old_indices.insert(o_idx);
                matched_new_indices.insert(n_idx);

                let lineage_id = old_sym
                    .lineage_id
                    .clone()
                    .or_else(|| new_sym.lineage_id.clone())
                    .unwrap_or_else(|| LineageId::new(format!("lin_{}", old_sym.name)));

                if old_sym.name != new_sym.name {
                    changes.push(SemanticChange::SymbolRenamed {
                        old_symbol_id: old_sym.id.clone(),
                        new_symbol_id: new_sym.id.clone(),
                        lineage_id: lineage_id.clone(),
                        old_name: old_sym.name.clone(),
                        new_name: new_sym.name.clone(),
                    });
                }

                if old_sym.canonical_signature != new_sym.canonical_signature {
                    changes.push(SemanticChange::SignatureChanged {
                        old_symbol_id: old_sym.id.clone(),
                        new_symbol_id: new_sym.id.clone(),
                        lineage_id: lineage_id.clone(),
                        old_signature: old_sym.canonical_signature.clone(),
                        new_signature: new_sym.canonical_signature.clone(),
                    });
                } else if old_sym.body_hash != new_sym.body_hash {
                    changes.push(SemanticChange::BodyChanged {
                        symbol_id: new_sym.id.clone(),
                        lineage_id: lineage_id.clone(),
                        old_body_hash: old_sym.body_hash.clone(),
                        new_body_hash: new_sym.body_hash.clone(),
                    });
                }

                if old_sym.visibility != new_sym.visibility {
                    changes.push(SemanticChange::VisibilityChanged {
                        symbol_id: new_sym.id.clone(),
                        lineage_id,
                        old_vis: old_sym.visibility,
                        new_vis: new_sym.visibility,
                    });
                }
            }
        }

        // گام ۲: تطبیق نمادهای تغییرنام‌یافته (Rename) بر اساس ساختار یا Lineage مشترک
        for (o_idx, old_sym) in old_file.symbols.iter().enumerate() {
            if matched_old_indices.contains(&o_idx) {
                continue;
            }

            let candidate_new = new_file.symbols.iter().enumerate().find(|(idx, n)| {
                if matched_new_indices.contains(idx) {
                    return false;
                }
                let same_lineage =
                    old_sym.lineage_id.is_some() && old_sym.lineage_id == n.lineage_id;
                let single_symbol_heuristic =
                    old_file.symbols.len() == 1 && new_file.symbols.len() == 1;
                let same_kind = old_sym.kind == n.kind;

                same_lineage
                    || (single_symbol_heuristic && same_kind)
                    || (same_kind && old_sym.body_hash == n.body_hash)
            });

            if let Some((n_idx, new_sym)) = candidate_new {
                matched_old_indices.insert(o_idx);
                matched_new_indices.insert(n_idx);

                let lineage_id = old_sym
                    .lineage_id
                    .clone()
                    .or_else(|| new_sym.lineage_id.clone())
                    .unwrap_or_else(|| LineageId::new(format!("lin_{}", old_sym.name)));

                changes.push(SemanticChange::SymbolRenamed {
                    old_symbol_id: old_sym.id.clone(),
                    new_symbol_id: new_sym.id.clone(),
                    lineage_id: lineage_id.clone(),
                    old_name: old_sym.name.clone(),
                    new_name: new_sym.name.clone(),
                });

                if old_sym.canonical_signature != new_sym.canonical_signature {
                    changes.push(SemanticChange::SignatureChanged {
                        old_symbol_id: old_sym.id.clone(),
                        new_symbol_id: new_sym.id.clone(),
                        lineage_id: lineage_id.clone(),
                        old_signature: old_sym.canonical_signature.clone(),
                        new_signature: new_sym.canonical_signature.clone(),
                    });
                } else if old_sym.body_hash != new_sym.body_hash {
                    changes.push(SemanticChange::BodyChanged {
                        symbol_id: new_sym.id.clone(),
                        lineage_id,
                        old_body_hash: old_sym.body_hash.clone(),
                        new_body_hash: new_sym.body_hash.clone(),
                    });
                }
            }
        }

        // گام ۳: نمادهایی که در فایل قبلی حذف شده‌اند
        for (o_idx, old_sym) in old_file.symbols.iter().enumerate() {
            if !matched_old_indices.contains(&o_idx) {
                let lineage_id = old_sym
                    .lineage_id
                    .clone()
                    .unwrap_or_else(LineageId::generate_persistent);
                changes.push(SemanticChange::SymbolRemoved {
                    symbol_id: old_sym.id.clone(),
                    lineage_id,
                });
            }
        }

        // گام ۴: نمادهای جدید اضافه شده
        for (n_idx, new_sym) in new_file.symbols.iter().enumerate() {
            if !matched_new_indices.contains(&n_idx) {
                let lineage_id = new_sym
                    .lineage_id
                    .clone()
                    .unwrap_or_else(LineageId::generate_persistent);
                changes.push(SemanticChange::SymbolAdded {
                    symbol_id: new_sym.id.clone(),
                    lineage_id,
                    signature: new_sym.canonical_signature.clone(),
                });
            }
        }

        changes
    }

    fn calculate_symbol_similarity(file_a: &FileSnapshot, file_b: &FileSnapshot) -> f32 {
        if file_a.symbols.is_empty() || file_b.symbols.is_empty() {
            return 0.0;
        }

        let set_a: HashSet<&str> = file_a.symbols.iter().map(|s| s.name.as_str()).collect();
        let set_b: HashSet<&str> = file_b.symbols.iter().map(|s| s.name.as_str()).collect();

        let intersection_count = set_a.intersection(&set_b).count();
        let union_count = set_a.union(&set_b).count();

        if union_count == 0 {
            0.0
        } else {
            intersection_count as f32 / union_count as f32
        }
    }
}
