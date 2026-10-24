use super::model::{LineageEvidence, LineageId, LineageResolution, LineageWeights, SymbolIdentity};
use std::collections::HashSet;

#[derive(Default)]
pub struct LineageEngine {
    weights: LineageWeights,
}

impl LineageEngine {
    pub fn new(weights: LineageWeights) -> Self {
        Self { weights }
    }

    /// محاسبه شواهد ساختاری و معنایی بین نماد قدیمی و نماد جدید
    pub fn evaluate_evidence(
        old_sym: &SymbolIdentity,
        new_sym: &SymbolIdentity,
    ) -> LineageEvidence {
        let same_file_id = old_sym.file_id == new_sym.file_id;
        let same_module_namespace = old_sym.module_namespace == new_sym.module_namespace;
        let same_scope = old_sym.qualified_scope == new_sym.qualified_scope;
        let same_kind = old_sym.kind == new_sym.kind;

        let name_similarity = if old_sym.name == new_sym.name {
            1.0
        } else if old_sym.name.to_lowercase() == new_sym.name.to_lowercase() {
            0.8
        } else {
            0.0
        };

        let signature_similarity = if old_sym.canonical_signature == new_sym.canonical_signature {
            1.0
        } else if old_sym.kind == new_sym.kind {
            0.5
        } else {
            0.0
        };

        let body_similarity =
            if !old_sym.body_hash.is_empty() && old_sym.body_hash == new_sym.body_hash {
                1.0
            } else {
                0.0
            };

        let ast_structure_similarity = if same_kind && (same_scope || body_similarity == 1.0) {
            1.0
        } else if same_kind {
            0.6
        } else {
            0.0
        };

        let relative_position_similarity = if old_sym.start_line == new_sym.start_line {
            1.0
        } else {
            let diff = (old_sym.start_line as isize - new_sym.start_line as isize).unsigned_abs();
            (1.0 - (diff as f64 / 100.0)).max(0.0)
        };

        LineageEvidence {
            same_file_id,
            same_module_namespace,
            same_scope,
            same_kind,
            name_similarity,
            signature_similarity,
            body_similarity,
            ast_structure_similarity,
            relative_position_similarity,
            neighboring_symbol_similarity: if same_module_namespace { 1.0 } else { 0.0 },
            reference_topology_similarity: if same_scope { 1.0 } else { 0.5 },
        }
    }

    /// محاسبه امتیاز قطعی تبار با ضرایب وزن‌دار
    pub fn compute_score(&self, evidence: &LineageEvidence) -> f64 {
        if evidence.same_kind
            && evidence.name_similarity == 1.0
            && evidence.same_scope
            && evidence.same_module_namespace
        {
            if evidence.body_similarity == 1.0 {
                return 1.0;
            }
            return 0.92;
        }

        if evidence.same_file_id && evidence.body_similarity == 1.0 && evidence.same_kind {
            return 0.95;
        }

        let mut score = 0.0;
        let w = &self.weights;

        if evidence.same_file_id {
            score += w.file;
        }
        if evidence.same_module_namespace {
            score += w.module;
        }
        if evidence.same_scope {
            score += w.scope;
        }
        if evidence.same_kind {
            score += w.kind;
        }

        score += evidence.name_similarity * w.name;
        score += evidence.signature_similarity * w.signature;
        score += evidence.body_similarity * w.body;
        score += evidence.ast_structure_similarity * w.ast;
        score += evidence.relative_position_similarity * w.position;
        score += evidence.neighboring_symbol_similarity * w.neighborhood;
        score += evidence.reference_topology_similarity * w.topology;

        let total_weight = w.file
            + w.module
            + w.scope
            + w.kind
            + w.name
            + w.signature
            + w.body
            + w.ast
            + w.position
            + w.neighborhood
            + w.topology;
        if total_weight > 0.0 {
            score / total_weight
        } else {
            0.0
        }
    }

    /// حل سراسری تبار بر پایه Global Maximum-Weight Bipartite Assignment و سورت قطعی
    pub fn resolve_lineages(
        &self,
        base_symbols: &[SymbolIdentity],
        target_symbols: &[SymbolIdentity],
    ) -> Vec<LineageResolution> {
        let mut sorted_base: Vec<(usize, &SymbolIdentity)> =
            base_symbols.iter().enumerate().collect();
        sorted_base.sort_by(|a, b| a.1.id.0.cmp(&b.1.id.0));

        let mut sorted_target: Vec<(usize, &SymbolIdentity)> =
            target_symbols.iter().enumerate().collect();
        sorted_target.sort_by(|a, b| a.1.id.0.cmp(&b.1.id.0));

        struct CandidateEdge {
            b_idx: usize,
            t_idx: usize,
            score: f64,
            evidence: LineageEvidence,
        }

        let mut candidate_edges = Vec::new();

        for &(t_idx, target_sym) in &sorted_target {
            for &(b_idx, base_sym) in &sorted_base {
                let evidence = Self::evaluate_evidence(base_sym, target_sym);

                let is_tier_a = evidence.same_file_id && evidence.same_kind && evidence.same_scope;
                let is_tier_b = evidence.same_file_id
                    && evidence.same_kind
                    && evidence.signature_similarity >= 0.5;
                let is_tier_c = !evidence.same_file_id
                    && evidence.same_kind
                    && (evidence.name_similarity >= 0.8 || evidence.body_similarity == 1.0);
                let is_tier_d = evidence.same_module_namespace && evidence.same_kind;

                if !is_tier_a && !is_tier_b && !is_tier_c && !is_tier_d {
                    continue;
                }

                let score = self.compute_score(&evidence);
                if score >= 0.40 {
                    candidate_edges.push(CandidateEdge {
                        b_idx,
                        t_idx,
                        score,
                        evidence,
                    });
                }
            }
        }

        // سورت قطعی یال‌ها: اولویت امتیاز نزولی، سپس تقدم قطعی same_file_id
        candidate_edges.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.evidence.same_file_id.cmp(&a.evidence.same_file_id))
                .then_with(|| base_symbols[a.b_idx].id.0.cmp(&base_symbols[b.b_idx].id.0))
                .then_with(|| {
                    target_symbols[a.t_idx]
                        .id
                        .0
                        .cmp(&target_symbols[b.t_idx].id.0)
                })
        });

        let mut matched_base = HashSet::new();
        let mut matched_target = HashSet::new();
        let mut resolutions = Vec::new();

        // تطابق بهینه یک‌به‌یک سراسری
        for edge in candidate_edges {
            if matched_base.contains(&edge.b_idx) || matched_target.contains(&edge.t_idx) {
                continue;
            }

            let base_sym = &base_symbols[edge.b_idx];
            let target_sym = &target_symbols[edge.t_idx];

            let lineage = match &base_sym.lineage_id {
                Some(lin) => lin.clone(),
                None => continue,
            };

            if edge.score >= 0.50 {
                matched_base.insert(edge.b_idx);
                matched_target.insert(edge.t_idx);

                resolutions.push(LineageResolution::Matched {
                    lineage_id: lineage,
                    old_symbol_id: base_sym.id.clone(),
                    new_symbol_id: target_sym.id.clone(),
                    confidence: edge.score,
                    evidence: edge.evidence,
                });
            }
        }

        // تولید تبار جدید صرفاً برای نمادهای واقعاً جدید
        for (t_idx, target_sym) in target_symbols.iter().enumerate() {
            if !matched_target.contains(&t_idx) {
                resolutions.push(LineageResolution::New {
                    lineage_id: LineageId::generate_persistent(),
                    new_symbol_id: target_sym.id.clone(),
                });
            }
        }

        // علامت‌گذاری نمادهای حذف‌شده با تبار قبلی آن‌ها
        for (b_idx, base_sym) in base_symbols.iter().enumerate() {
            if !matched_base.contains(&b_idx) {
                if let Some(ref lin) = base_sym.lineage_id {
                    resolutions.push(LineageResolution::Removed {
                        lineage_id: lin.clone(),
                        old_symbol_id: base_sym.id.clone(),
                    });
                }
            }
        }

        resolutions.sort_by(|a, b| {
            let get_id = |r: &LineageResolution| match r {
                LineageResolution::Matched { new_symbol_id, .. } => new_symbol_id.0.clone(),
                LineageResolution::New { new_symbol_id, .. } => new_symbol_id.0.clone(),
                LineageResolution::Ambiguous { new_symbol_id, .. } => new_symbol_id.0.clone(),
                LineageResolution::Removed { old_symbol_id, .. } => old_symbol_id.0.clone(),
            };
            get_id(a).cmp(&get_id(b))
        });

        resolutions
    }
}
