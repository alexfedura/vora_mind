use std::time::Instant;
use vora_lib::forge::ast_engine::AstEngine;
use vora_lib::forge::graph::DependencyGraph;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::model::{FileId, ResolutionStatus};
use vora_lib::forge::resolver::SemanticResolver;

fn calculate_percentile(latencies: &mut [f64], percentile: f64) -> f64 {
    if latencies.is_empty() {
        return 0.0;
    }
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((latencies.len() as f64) * (percentile / 100.0)).ceil() as usize;
    latencies[idx.saturating_sub(1).min(latencies.len() - 1)]
}

fn main() {
    println!("================================================================================");
    println!(" 🚀 RUNNING HIGH-SCALE THROUGHPUT & LATENCY BENCHMARK (5000+ SYMBOLS)");
    println!("================================================================================");

    let index = CodeIndex::new_in_memory().expect("Failed to create in-memory SQLite index");

    let total_files = 250;
    let symbols_per_file = 20; // مجموعاً ۵۰۰۰ نماد
    let mut generated_files = Vec::new();

    println!("\n🔨 [1/4] Synthesizing 5000 interconnected symbol declarations...");

    for f_idx in 0..total_files {
        let mut file_content = String::new();

        if f_idx + 1 < total_files {
            file_content.push_str(&format!(
                "use crate::module_{}::service_func_0;\n\n",
                f_idx + 1
            ));
        }

        for s_idx in 0..symbols_per_file {
            if s_idx == 0 && f_idx + 1 < total_files {
                file_content.push_str(&format!(
                    "pub fn service_func_{}(val: u64) -> u64 {{\n    service_func_0(val + 1)\n}}\n\n",
                    s_idx
                ));
            } else if s_idx > 0 {
                file_content.push_str(&format!(
                    "pub fn service_func_{}(val: u64) -> u64 {{\n    service_func_{}(val + {})\n}}\n\n",
                    s_idx,
                    s_idx - 1,
                    s_idx
                ));
            } else {
                file_content.push_str(&format!(
                    "pub fn service_func_{}(val: u64) -> u64 {{\n    val * 2\n}}\n\n",
                    s_idx
                ));
            }
        }

        let file_path = format!("src/module_{}.rs", f_idx);
        generated_files.push((file_path, file_content));
    }

    // ایندکسینگ دسته‌ای
    println!(
        "📦 [2/4] Ingesting {} files into SQLite & AST Engine...",
        generated_files.len()
    );
    let t_index_start = Instant::now();

    for (path, content) in &generated_files {
        let fid = FileId::generate();
        let parsed_semantics = AstEngine::parse_file_semantics(fid, path, content);
        index
            .index_file_semantics(&parsed_semantics)
            .expect("Indexing semantics failed");
        let parsed_syntax = AstEngine::parse_file_syntax(path, content);
        index
            .index_file(path, &parsed_syntax)
            .expect("Indexing syntax failed");
    }

    let t_index_total = t_index_start.elapsed();
    println!(
        "  ✅ Indexed {} symbols in: {:?}",
        total_files * symbols_per_file,
        t_index_total
    );

    // رزولوشن لبه‌های معنایی
    println!("\n🔗 [3/4] Resolving cross-file edges across 5000 symbols...");
    let t_resolve_start = Instant::now();

    let strict_edges = SemanticResolver::resolve_strict_semantic_edges(&index)
        .expect("Semantic resolution failed");

    let t_resolve_total = t_resolve_start.elapsed();
    let resolved_count = strict_edges
        .iter()
        .filter(|e| matches!(e.status, ResolutionStatus::Resolved(_)))
        .count();

    println!("  ✅ Strict Edges Resolved:   {}", resolved_count);
    println!("  ⏱️ Total Resolver Latency:  {:?}", t_resolve_total);

    // تحلیل تاخیر ارزیابی اثر آبشاری
    println!("\n💥 [4/4] Evaluating Impact Latency Percentiles (500 Iterations)...");
    let mut graph = DependencyGraph::new();
    graph.build_from_strict_semantic_edges(&strict_edges);

    let resolved_targets: Vec<String> = strict_edges
        .iter()
        .filter_map(|e| e.target_symbol.as_ref().map(|s| s.0.clone()))
        .collect();

    let mut query_latencies_us = Vec::new();

    for target_sym in resolved_targets.iter().take(500) {
        let t_q = Instant::now();
        let _report = graph.analyze_impact(target_sym);
        let elapsed_us = t_q.elapsed().as_secs_f64() * 1_000_000.0;
        query_latencies_us.push(elapsed_us);
    }

    let p50 = calculate_percentile(&mut query_latencies_us, 50.0);
    let p90 = calculate_percentile(&mut query_latencies_us, 90.0);
    let p95 = calculate_percentile(&mut query_latencies_us, 95.0);
    let p99 = calculate_percentile(&mut query_latencies_us, 99.0);

    println!("\n📈 LATENCY BENCHMARK RESULTS (Microseconds / µs):");
    println!("  ┌─────────────┬────────────────┐");
    println!("  │ Percentile  │ Latency (µs)   │");
    println!("  ├─────────────┼────────────────┤");
    println!("  │ P50 (Med)   │ {:>10.2} µs  │", p50);
    println!("  │ P90         │ {:>10.2} µs  │", p90);
    println!("  │ P95         │ {:>10.2} µs  │", p95);
    println!("  │ P99 (Worst) │ {:>10.2} µs  │", p99);
    println!("  └─────────────┴────────────────┘");

    println!("\n================================================================================");
    println!(" 🏆 HIGH-SCALE BENCHMARK COMPLETED SUCCESSFULLY");
    println!("================================================================================\n");
}
