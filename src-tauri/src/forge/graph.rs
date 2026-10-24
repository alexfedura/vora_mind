use super::model::{EdgeRelationKind, ResolvedSemanticEdge, SemanticChange};
use super::resolver::{ResolvedEdge, SemanticProjectGraph};
use super::skeleton::SymbolDefinition;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactedTarget {
    pub symbol_id: String,
    pub file_path: String,
    pub symbol_name: String,
    pub relation_kind: String,
    pub cascade_depth: usize,
    pub is_test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysisReport {
    pub target_symbol_id: String,
    pub total_impacted_count: usize,
    pub direct_dependents: Vec<ImpactedTarget>,
    pub indirect_dependents: Vec<ImpactedTarget>,
    pub affected_test_files: Vec<String>,
}

#[derive(Debug, Default)]
pub struct DependencyGraph {
    pub definitions: HashMap<String, Vec<SymbolDefinition>>,
    pub reverse_callers_map: HashMap<String, Vec<(String, String, String)>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build_from_definitions(&mut self, all_defs: Vec<SymbolDefinition>) {
        self.definitions.clear();
        self.reverse_callers_map.clear();

        for def in &all_defs {
            self.definitions
                .entry(def.name.clone())
                .or_default()
                .push(def.clone());

            for out_ref in &def.outbound_references {
                self.reverse_callers_map
                    .entry(out_ref.clone())
                    .or_default()
                    .push((
                        def.name.clone(),
                        def.file_path.clone(),
                        "calls_function".to_string(),
                    ));
            }
        }
    }

    pub fn build_from_semantic_graph(&mut self, graph: &SemanticProjectGraph) {
        self.reverse_callers_map.clear();

        for edge in &graph.edges {
            self.reverse_callers_map
                .entry(edge.target_symbol_id.clone())
                .or_default()
                .push((
                    edge.source_symbol_id.clone(),
                    edge.source_file.clone(),
                    edge.relation_kind.clone(),
                ));
        }
    }

    pub fn build_from_edges(&mut self, edges: &[ResolvedEdge]) {
        self.reverse_callers_map.clear();

        for edge in edges {
            self.reverse_callers_map
                .entry(edge.target_symbol_id.clone())
                .or_default()
                .push((
                    edge.source_symbol_id.clone(),
                    edge.source_file.clone(),
                    edge.relation_kind.clone(),
                ));
        }
    }

    pub fn build_from_strict_semantic_edges(&mut self, edges: &[ResolvedSemanticEdge]) {
        self.reverse_callers_map.clear();

        for edge in edges {
            if let Some(target_sym) = &edge.target_symbol {
                let rel_str = match edge.relation {
                    EdgeRelationKind::Calls => "calls_function",
                    EdgeRelationKind::ReferencesType => "references_type",
                    EdgeRelationKind::Implements => "implements_trait",
                    EdgeRelationKind::Extends => "extends_class",
                    EdgeRelationKind::UsesField => "uses_field",
                };

                let caller_file = edge
                    .source_symbol
                    .0
                    .split(':')
                    .next()
                    .unwrap_or("unknown")
                    .to_string();

                self.reverse_callers_map
                    .entry(target_sym.0.clone())
                    .or_default()
                    .push((
                        edge.source_symbol.0.clone(),
                        caller_file,
                        rel_str.to_string(),
                    ));
            }
        }
    }

    pub fn analyze_semantic_change_impact(&self, change: &SemanticChange) -> ImpactAnalysisReport {
        match change {
            // قانون ۱: تغییر در بدنه یا اضافه شدن نماد شعاع انفجار صفر دارد
            SemanticChange::BodyChanged { symbol_id, .. }
            | SemanticChange::SymbolAdded { symbol_id, .. } => ImpactAnalysisReport {
                target_symbol_id: symbol_id.0.clone(),
                total_impacted_count: 0,
                direct_dependents: Vec::new(),
                indirect_dependents: Vec::new(),
                affected_test_files: Vec::new(),
            },
            // قانون ۲: تغییر امضا، تغییر نام یا جهش تعاریف تایپ به تمام وابستگی‌ها سرایت می‌کند
            SemanticChange::SignatureChanged { old_symbol_id, .. }
            | SemanticChange::SymbolRenamed { old_symbol_id, .. }
            | SemanticChange::TypeDefinitionChanged { old_symbol_id, .. } => {
                self.analyze_impact(&old_symbol_id.0)
            }
            SemanticChange::VisibilityChanged { symbol_id, .. }
            | SemanticChange::SymbolRemoved { symbol_id, .. } => self.analyze_impact(&symbol_id.0),
        }
    }

    pub fn analyze_impact(&self, target_symbol_id: &str) -> ImpactAnalysisReport {
        let mut direct_dependents = Vec::new();
        let mut indirect_dependents = Vec::new();
        let mut affected_test_files_set = HashSet::new();

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        visited.insert(target_symbol_id.to_string());
        queue.push_back((target_symbol_id.to_string(), 0));

        while let Some((current_sym, depth)) = queue.pop_front() {
            if let Some(callers) = self.reverse_callers_map.get(&current_sym) {
                for (caller_id, caller_file, relation) in callers {
                    let is_test = caller_file.contains("test") || caller_file.starts_with("tests/");
                    if is_test {
                        affected_test_files_set.insert(caller_file.clone());
                    }

                    if !visited.contains(caller_id) {
                        visited.insert(caller_id.clone());

                        let target = ImpactedTarget {
                            symbol_id: caller_id.clone(),
                            file_path: caller_file.clone(),
                            symbol_name: caller_id
                                .split("::")
                                .last()
                                .unwrap_or(caller_id)
                                .to_string(),
                            relation_kind: relation.clone(),
                            cascade_depth: depth + 1,
                            is_test,
                        };

                        if depth == 0 {
                            direct_dependents.push(target);
                        } else {
                            indirect_dependents.push(target);
                        }

                        queue.push_back((caller_id.clone(), depth + 1));
                    }
                }
            }
        }

        // P11: مرتب‌سازی قطعی بر اساس عمق برای پردازش منظم ترکنش‌های چندفایلی
        direct_dependents.sort_by(|a, b| a.symbol_name.cmp(&b.symbol_name));
        indirect_dependents.sort_by(|a, b| {
            a.cascade_depth
                .cmp(&b.cascade_depth)
                .then_with(|| a.symbol_name.cmp(&b.symbol_name))
        });

        let total_impacted_count = direct_dependents.len() + indirect_dependents.len();
        let affected_test_files = affected_test_files_set.into_iter().collect();

        ImpactAnalysisReport {
            target_symbol_id: target_symbol_id.to_string(),
            total_impacted_count,
            direct_dependents,
            indirect_dependents,
            affected_test_files,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{LineageId, SymbolId};

    #[test]
    fn test_body_changed_zero_impact_rule() {
        let graph = DependencyGraph::new();
        let change = SemanticChange::BodyChanged {
            symbol_id: SymbolId::new("sym_test"),
            lineage_id: LineageId::new("lin_test"),
            old_body_hash: "hash1".to_string(),
            new_body_hash: "hash2".to_string(),
        };

        let report = graph.analyze_semantic_change_impact(&change);
        assert_eq!(report.total_impacted_count, 0);
    }
}
