use std::time::Instant;
use vora_lib::forge::ast_engine::AstEngine;
use vora_lib::forge::graph::DependencyGraph;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::{FileId, LineageId, SemanticChange, SymbolId};
use vora_lib::forge::resolver::SemanticResolver;

fn main() {
    println!("============================================================");
    println!(" 🚀 RUNNING SEMANTIC CORE V1.0 GRAPH & RESOLVER BENCHMARK");
    println!("============================================================");

    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory index");

    let module_a = r#"
use crate::billing::tax::compute_tax;
use crate::auth::verify_token;
pub fn checkout_cart(user_id: u64, amount: u64) -> bool {
    if !verify_token(user_id) {
        return false;
    }
    let tax = compute_tax(amount);
    true
}
"#;
    let module_b = r#"
use crate::payment::gateway::process_payment;
pub fn compute_tax(amount: u64) -> u64 {
    let _ = process_payment(amount);
    amount * 10 / 100
}
"#;
    let module_c = r#"
pub fn process_payment(amount: u64) -> bool {
    amount > 0
}
"#;
    let module_d = r#"
pub fn verify_token(user_id: u64) -> bool {
    user_id > 1000
}
"#;

    let file_specs = vec![
        ("src/checkout/cart.rs", module_a),
        ("src/billing/tax.rs", module_b),
        ("src/payment/gateway.rs", module_c),
        ("src/auth/token.rs", module_d),
    ];

    println!(
        "\n📦 [1/4] Indexing {} Source Modules into AST Engine...",
        file_specs.len()
    );
    let t_index_start = Instant::now();
    for (path, content) in &file_specs {
        let fid = FileId::generate();
        let parsed_semantics = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed_semantics)
            .expect("Failed to index file semantics");
        let parsed_syntax = AstEngine::parse_file_syntax(path, content);
        index
            .index_file(path, &parsed_syntax)
            .expect("Failed to index file syntax");
    }
    let t_index_duration = t_index_start.elapsed();
    println!(
        "  ✅ AST Parsing & SQLite Indexing Completed in: {:?}",
        t_index_duration
    );

    println!("\n🔗 [2/4] Resolving Cross-File Semantic Edges...");
    let t_resolve_start = Instant::now();
    let strict_edges = SemanticResolver::resolve_strict_semantic_edges(&index)
        .expect("Failed to resolve strict semantic edges");
    let t_resolve_duration = t_resolve_start.elapsed();
    println!(
        "  ✅ Resolved Strict Semantic Edges: {}",
        strict_edges.len()
    );
    println!(
        "  ⏱️ Resolver Latency:                 {:?}",
        t_resolve_duration
    );

    assert!(
        !strict_edges.is_empty(),
        "FATAL: Semantic Resolver generated 0 edges on connected fixture!"
    );

    println!("\n💥 [3/4] Building Dependency Graph & Evaluating Blast Radius...");
    let mut dep_graph = DependencyGraph::new();
    dep_graph.build_from_strict_semantic_edges(&strict_edges);
    let t_impact_start = Instant::now();

    let resolved_targets: Vec<String> = strict_edges
        .iter()
        .filter_map(|e| e.target_symbol.as_ref().map(|s| s.0.clone()))
        .collect();
    let root_target = resolved_targets.first().cloned().unwrap_or_default();
    let impact_report = dep_graph.analyze_impact(&root_target);
    let t_impact_duration = t_impact_start.elapsed();
    println!("  🎯 Root Target Symbol:     {}", root_target);
    println!(
        "  📊 Total Impacted Symbols: {}",
        impact_report.total_impacted_count
    );
    println!(
        "  ⚡ Direct Dependents:       {}",
        impact_report.direct_dependents.len()
    );
    println!(
        "  🔁 Indirect Dependents:     {}",
        impact_report.indirect_dependents.len()
    );
    println!("  ⏱️ Impact Analysis Latency: {:?}", t_impact_duration);

    assert!(
        impact_report.total_impacted_count >= 1,
        "Blast radius should detect cascading dependents"
    );

    println!("\n🔬 [4/4] Verifying Semantic Policy Zero-Impact Rule...");
    let body_change = SemanticChange::BodyChanged {
        symbol_id: SymbolId(root_target),
        lineage_id: LineageId::new("lin_benchmark_target"),
        old_body_hash: "hash_v1".to_string(),
        new_body_hash: "hash_v2".to_string(),
    };
    let body_report = dep_graph.analyze_semantic_change_impact(&body_change);
    println!(
        "  🛡️ Body-Mutation Impact Count: {}",
        body_report.total_impacted_count
    );
    assert_eq!(body_report.total_impacted_count, 0);
    println!("\n============================================================");
    println!(" 🏆 ALL BENCHMARK CHECKS PASSED: NON-ZERO EDGES PROVEN & GREEN");
    println!("============================================================\n");
}
