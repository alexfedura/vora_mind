use super::index::CodeIndex;
use super::model::{
    EdgeRelationKind, ImportKind, ResolutionStatus, ResolvedSemanticEdge, SymbolId,
};
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub type SymbolRecord = (String, String, String, String, String, String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedEdge {
    pub source_symbol_id: String,
    pub target_symbol_id: String,
    pub source_file: String,
    pub target_file: String,
    pub relation_kind: String,
    pub call_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticProjectGraph {
    pub edges: Vec<ResolvedEdge>,
    pub total_symbols: usize,
    pub total_edges: usize,
}

pub struct SemanticResolver;

impl SemanticResolver {
    pub fn resolve_strict_semantic_edges(index: &CodeIndex) -> Result<Vec<ResolvedSemanticEdge>> {
        let conn = index.connection();

        let mut symbol_map: HashMap<String, Vec<SymbolRecord>> = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT id, file_id, module_namespace, qualified_scope, kind, name, file_path FROM code_symbols",
        )?;
        let symbol_rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;

        for item in symbol_rows.flatten() {
            symbol_map
                .entry(item.5.clone())
                .or_default()
                .push((item.0, item.1, item.2, item.3, item.4, item.6));
        }

        let mut scope_map: HashMap<String, (Option<String>, HashMap<String, ImportKind>)> =
            HashMap::new();
        let mut scope_stmt = conn
            .prepare("SELECT scope_id, parent_scope_id, visible_bindings_json FROM code_scopes")?;
        let scope_rows = scope_stmt.query_map([], |row| {
            let scope_id: String = row.get(0)?;
            let parent_id: Option<String> = row.get(1)?;
            let bindings_json: String = row.get(2)?;
            let bindings: HashMap<String, ImportKind> =
                serde_json::from_str(&bindings_json).unwrap_or_default();
            Ok((scope_id, parent_id, bindings))
        })?;

        for item in scope_rows.flatten() {
            scope_map.insert(item.0, (item.1, item.2));
        }

        let mut file_imports: HashMap<String, Vec<(String, String, Option<String>)>> =
            HashMap::new();
        if let Ok(mut import_stmt) = conn.prepare(
            "SELECT file_id, file_path, source_module, imported_name, alias FROM code_imports",
        ) {
            let import_rows = import_stmt.query_map([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?;

            for item in import_rows.flatten() {
                let (file_id_opt, file_path, source_module, imported_name, alias) = item;
                file_imports.entry(file_path.clone()).or_default().push((
                    source_module.clone(),
                    imported_name.clone(),
                    alias.clone(),
                ));

                if let Some(fid) = file_id_opt {
                    file_imports.entry(fid).or_default().push((
                        source_module,
                        imported_name,
                        alias,
                    ));
                }
            }
        }

        let mut ref_stmt = conn.prepare(
            "SELECT file_id, scope_id, caller_symbol_id, callee_raw_name, receiver_name, is_member_access, ref_kind, line FROM code_raw_references",
        )?;

        let ref_rows = ref_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)? == 1,
                row.get::<_, String>(6)?,
                row.get::<_, usize>(7)?,
            ))
        })?;

        let mut strict_edges = Vec::new();

        for r in ref_rows.flatten() {
            let (
                file_id,
                scope_id,
                caller_opt,
                callee_name,
                receiver_name,
                is_member_access,
                ref_kind_str,
                line,
            ) = r;
            let caller_id = caller_opt.unwrap_or_else(|| format!("{}:root", file_id));

            let relation = if ref_kind_str.contains("Type") {
                EdgeRelationKind::ReferencesType
            } else if ref_kind_str.contains("Trait") || ref_kind_str.contains("Impl") {
                EdgeRelationKind::Implements
            } else {
                EdgeRelationKind::Calls
            };

            let mut resolved_symbol_id = None;

            // ۱. بررسی اسکوپ محلی و پیمایش بالا به پایین سلسله‌مراتب لکسیکال
            let mut curr_scope = Some(scope_id.clone());
            while let Some(sid) = curr_scope {
                if let Some((parent_opt, bindings)) = scope_map.get(&sid) {
                    if let Some(import_entry) = bindings.get(&callee_name) {
                        match import_entry {
                            ImportKind::Alias { source_module, .. } => {
                                let orig_target_name =
                                    source_module.rsplit("::").next().unwrap_or(source_module);
                                if let Some(candidates) = symbol_map.get(orig_target_name) {
                                    if let Some(cand) = candidates.iter().find(|c| {
                                        Self::is_module_match(&file_id, source_module, &c.2, &c.5)
                                    }) {
                                        resolved_symbol_id = Some(cand.0.clone());
                                        break;
                                    }
                                }
                            }
                            ImportKind::Imported {
                                source_module,
                                original_name,
                            } => {
                                if let Some(candidates) = symbol_map.get(original_name) {
                                    if let Some(cand) = candidates.iter().find(|c| {
                                        Self::is_module_match(&file_id, source_module, &c.2, &c.5)
                                    }) {
                                        resolved_symbol_id = Some(cand.0.clone());
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    curr_scope = parent_opt.clone();
                } else {
                    break;
                }
            }

            // ۲. بررسی واردات سطح فایل
            if resolved_symbol_id.is_none() {
                if let Some(imports) = file_imports.get(&file_id) {
                    for (source_mod, imp_name, alias) in imports {
                        let matches_alias = alias.as_deref() == Some(&callee_name);
                        let matches_name = imp_name == &callee_name;
                        if matches_alias || matches_name {
                            if let Some(candidates) = symbol_map.get(imp_name) {
                                if let Some(cand) = candidates.iter().find(|c| {
                                    Self::is_module_match(&file_id, source_mod, &c.2, &c.5)
                                }) {
                                    resolved_symbol_id = Some(cand.0.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // ۳. بررسی نمادهای مستقیم محلی در همان فایل
            if resolved_symbol_id.is_none() {
                if let Some(candidates) = symbol_map.get(&callee_name) {
                    if let Some(local_cand) = candidates.iter().find(|c| c.1 == file_id) {
                        resolved_symbol_id = Some(local_cand.0.clone());
                    }
                }
            }

            if let Some(target_id) = resolved_symbol_id {
                if caller_id != target_id {
                    strict_edges.push(ResolvedSemanticEdge {
                        source_symbol: SymbolId(caller_id),
                        target_symbol: Some(SymbolId(target_id.clone())),
                        candidate_symbols: vec![SymbolId(target_id.clone())],
                        relation,
                        status: ResolutionStatus::Resolved(SymbolId(target_id)),
                        call_line: line,
                    });
                }
            } else if is_member_access && receiver_name.is_some() {
                strict_edges.push(ResolvedSemanticEdge {
                    source_symbol: SymbolId(caller_id),
                    target_symbol: None,
                    candidate_symbols: Vec::new(),
                    relation,
                    status: ResolutionStatus::DynamicFallback,
                    call_line: line,
                });
            } else if let Some(candidates) = symbol_map.get(&callee_name) {
                let amb_ids: Vec<SymbolId> =
                    candidates.iter().map(|c| SymbolId(c.0.clone())).collect();
                strict_edges.push(ResolvedSemanticEdge {
                    source_symbol: SymbolId(caller_id),
                    target_symbol: None,
                    candidate_symbols: amb_ids.clone(),
                    relation,
                    status: ResolutionStatus::Ambiguous(amb_ids),
                    call_line: line,
                });
            } else {
                strict_edges.push(ResolvedSemanticEdge {
                    source_symbol: SymbolId(caller_id),
                    target_symbol: None,
                    candidate_symbols: Vec::new(),
                    relation,
                    status: ResolutionStatus::Unresolved,
                    call_line: line,
                });
            }
        }

        Ok(strict_edges)
    }

    pub fn build_semantic_graph(index: &CodeIndex) -> Result<SemanticProjectGraph> {
        let conn = index.connection();

        let mut symbol_map: HashMap<String, Vec<(String, String, String, String)>> = HashMap::new();
        let mut stmt =
            conn.prepare("SELECT id, file_path, module_namespace, name, kind FROM code_symbols")?;
        let symbol_rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut total_symbols = 0;
        for item in symbol_rows.flatten() {
            total_symbols += 1;
            symbol_map
                .entry(item.3.clone())
                .or_default()
                .push((item.0, item.1, item.2, item.4));
        }

        let mut file_imports: HashMap<String, Vec<(String, String, Option<String>)>> =
            HashMap::new();
        let mut import_stmt = conn
            .prepare("SELECT file_path, source_module, imported_name, alias FROM code_imports")?;
        let import_rows = import_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;

        for item in import_rows.flatten() {
            file_imports
                .entry(item.0)
                .or_default()
                .push((item.1, item.2, item.3));
        }

        let mut ref_stmt =
            conn.prepare("SELECT file_path, caller_scope, callee_name, line FROM code_references")?;
        let ref_rows = ref_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, usize>(3)?,
            ))
        })?;

        let mut edges = Vec::new();

        for r in ref_rows.flatten() {
            let (file_path, caller_scope, callee_name, call_line) = r;
            let source_symbol_id = format!("{}:{}", file_path, caller_scope);

            let mut resolved_target: Option<(String, String, String)> = None;

            if let Some(candidates) = symbol_map.get(&callee_name) {
                if let Some(local_cand) = candidates.iter().find(|c| c.1 == file_path) {
                    resolved_target = Some((
                        local_cand.0.clone(),
                        local_cand.1.clone(),
                        local_cand.3.clone(),
                    ));
                } else if let Some(imports) = file_imports.get(&file_path) {
                    for (source_module, imported_name, alias) in imports {
                        let matches_import = if let Some(a) = alias {
                            a == &callee_name
                        } else {
                            imported_name == &callee_name || source_module.ends_with(&callee_name)
                        };

                        if matches_import {
                            for cand in candidates {
                                if Self::is_module_match(
                                    &file_path,
                                    source_module,
                                    &cand.2,
                                    &cand.1,
                                ) {
                                    resolved_target =
                                        Some((cand.0.clone(), cand.1.clone(), cand.3.clone()));
                                    break;
                                }
                            }
                        }
                        if resolved_target.is_some() {
                            break;
                        }
                    }
                }
            }

            if let Some((target_id, target_file, target_kind)) = resolved_target {
                if source_symbol_id != target_id {
                    let relation_kind = if target_kind.contains("struct")
                        || target_kind.contains("interface")
                        || target_kind.contains("type")
                    {
                        "references_type".to_string()
                    } else {
                        "calls_function".to_string()
                    };

                    edges.push(ResolvedEdge {
                        source_symbol_id,
                        target_symbol_id: target_id,
                        source_file: file_path,
                        target_file,
                        relation_kind,
                        call_line,
                    });
                }
            }
        }

        let total_edges = edges.len();

        Ok(SemanticProjectGraph {
            edges,
            total_symbols,
            total_edges,
        })
    }

    /// بررسی تطابق دقیق ماژولار و ساختاری مسیر ایمپورت با نام‌فضای کاندیدا
    fn is_module_match(
        _current_file: &str,
        import_path: &str,
        candidate_namespace: &str,
        candidate_file: &str,
    ) -> bool {
        let cleaned_import = import_path
            .trim_matches('"')
            .trim_matches('\'')
            .trim_matches(';')
            .trim();

        if cleaned_import == candidate_namespace || candidate_namespace.ends_with(cleaned_import) {
            return true;
        }

        let import_module_part = if cleaned_import.contains("::") {
            cleaned_import
                .rsplit_once("::")
                .map(|x| x.0)
                .unwrap_or(cleaned_import)
        } else {
            cleaned_import
        };

        if candidate_namespace == import_module_part
            || candidate_namespace.ends_with(import_module_part)
        {
            return true;
        }

        let cand_path = Path::new(candidate_file);
        if let Some(stem) = cand_path.file_stem().and_then(|s| s.to_str()) {
            if cleaned_import.ends_with(stem) || import_module_part.ends_with(stem) {
                return true;
            }
        }

        false
    }
}
