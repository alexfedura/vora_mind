use super::graph::{DependencyGraph, ImpactAnalysisReport, ImpactedTarget};
use super::model::CanonicalEntityId;
use super::skeleton::{SkeletonExtractor, SymbolDefinition};
use super::ssot::GitSSOT;
use std::collections::HashMap;
use std::path::Path;

pub struct ProjectContext {
    pub graph: DependencyGraph,
    pub all_definitions: Vec<SymbolDefinition>,
    pub canonical_registry: HashMap<CanonicalEntityId, SymbolDefinition>,
}

impl ProjectContext {
    /// Loads the entire Git-tracked project into an in-memory AST Dependency Graph with Cross-Language support
    pub fn load_from_repo<P: AsRef<Path>>(
        repo_path: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let ssot = GitSSOT::open(repo_path)?;
        let tracked_sources = ssot.load_all_tracked_sources()?;

        let mut all_definitions = Vec::new();
        let mut canonical_registry = HashMap::new();

        for (file_path, content) in &tracked_sources {
            let mut extractor = SkeletonExtractor::for_file(file_path);
            let defs = extractor.analyze_file(file_path, content);
            for def in &defs {
                if let Some(ref cid) = def.canonical_entity_id {
                    canonical_registry.insert(cid.clone(), def.clone());
                }
            }
            all_definitions.extend(defs);
        }

        let mut graph = DependencyGraph::new();
        graph.build_from_definitions(all_definitions.clone());

        Ok(Self {
            graph,
            all_definitions,
            canonical_registry,
        })
    }

    /// Simulates the blast radius if a given symbol is modified
    pub fn simulate_symbol_mutation(&self, symbol_name: &str) -> Vec<ImpactedTarget> {
        let report = self.graph.analyze_impact(symbol_name);
        let mut all_dependents = report.direct_dependents;
        all_dependents.extend(report.indirect_dependents);
        all_dependents
    }

    /// Analyzes the full impact report including direct, indirect, and test dependents
    pub fn analyze_symbol_impact(&self, symbol_name: &str) -> ImpactAnalysisReport {
        self.graph.analyze_impact(symbol_name)
    }

    /// Resolves symbol definition by its canonical entity identity
    pub fn find_by_canonical_id(
        &self,
        canonical_id: &CanonicalEntityId,
    ) -> Option<&SymbolDefinition> {
        self.canonical_registry.get(canonical_id)
    }

    /// Generates a lightweight, token-efficient skeleton of the entire codebase
    pub fn generate_global_skeleton(&self) -> String {
        let mut skeleton_output = String::new();
        let mut current_file = String::new();

        for def in &self.all_definitions {
            if def.file_path != current_file {
                current_file = def.file_path.clone();
                skeleton_output.push_str(&format!("\n// File: {}\n", current_file));
            }
            skeleton_output.push_str(&format!("  {}\n", def.signature));
        }

        skeleton_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_context_loading() {
        let context_result = ProjectContext::load_from_repo(".");
        assert!(context_result.is_ok(), "Failed to index current repository");

        let context = context_result.unwrap();
        println!("\n[PROJECT INDEX SUCCESS]");
        println!(
            "  Total Tracked Symbols Indexed: {}",
            context.all_definitions.len()
        );
        println!(
            "  Total Canonical Registry Entries: {}",
            context.canonical_registry.len()
        );

        let skeleton = context.generate_global_skeleton();
        println!("\n--- Sample Global Skeleton Output ---\n{}", skeleton);
    }
}
