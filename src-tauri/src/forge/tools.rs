use super::ast_engine::AstEngine;
use super::commit::{ForgeTransactionManager, TransactionCommitResult, TransactionPatch};
use super::differ::{SemanticDiffReport, SemanticDiffer};
use super::graph::{DependencyGraph, ImpactAnalysisReport};
use super::index::CodeIndex;
use super::model::{FileId, LineageId, SymbolKind};
use super::overlay::CodeOverlayManager;
use super::patcher_v2::{PatchResult, SurgicalPatcher};
use super::project::ProjectContext;
use super::resolver::SemanticResolver;
use super::slicer::{CodeSlice, ScopeSlicer};
use super::ssot::GitSSOT;
use rusqlite::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSearchResult {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub signature: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub tool: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateTestResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// RAII Candidate Workspace که کپی ایزوله‌ای از پروژه می‌سازد
/// و در زمان Drop شدن به صورت قطعی پاکسازی می‌شود تا محیط واقعی آلوده نشود.
pub struct CandidateWorkspace {
    pub temp_dir: PathBuf,
}

impl CandidateWorkspace {
    pub fn create(source_repo: &Path) -> std::io::Result<Self> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let unique_name = format!("vora_sandbox_{}_{}", std::process::id(), timestamp);
        let temp_dir = std::env::temp_dir().join(unique_name);
        fs::create_dir_all(&temp_dir)?;

        Self::copy_workspace_filtered(source_repo, &temp_dir)?;

        Ok(Self { temp_dir })
    }

    pub fn path(&self) -> &Path {
        &self.temp_dir
    }

    fn copy_workspace_filtered(src: &Path, dst: &Path) -> std::io::Result<()> {
        if !src.exists() {
            return Ok(());
        }

        if src.is_dir() {
            let file_name = src.file_name().unwrap_or_default().to_string_lossy();
            if file_name == "target"
                || file_name == "node_modules"
                || file_name == ".git"
                || file_name == "dist"
                || file_name.starts_with(".vora_sandbox")
            {
                return Ok(());
            }

            fs::create_dir_all(dst)?;
            for entry in fs::read_dir(src)? {
                let entry = entry?;
                let path = entry.path();
                let child_name = path.file_name().unwrap_or_default();
                Self::copy_workspace_filtered(&path, &dst.join(child_name))?;
            }
        } else {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(src, dst)?;
        }
        Ok(())
    }

    pub fn apply_patch(&self, rel_path: &str, patched_content: &str) -> std::io::Result<()> {
        let target_path = self.temp_dir.join(rel_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, patched_content)
    }

    /// A9: اجرای تست با گارد محدودیت زمان (Timeout 45s) و سقف حجم خروجی لاگ (Max 512KB)
    pub fn run_tests(&self, test_command: Option<&str>) -> CandidateTestResult {
        let cmd_str = test_command.unwrap_or("cargo test");
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();

        let (cmd_name, cmd_args) = if parts.is_empty() {
            ("cargo", vec!["test"])
        } else {
            (parts[0], parts[1..].to_vec())
        };

        let mut child = match Command::new(cmd_name)
            .args(&cmd_args)
            .current_dir(&self.temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => {
                return CandidateTestResult {
                    success: false,
                    exit_code: Some(1),
                    stdout: String::new(),
                    stderr: format!(
                        "Failed to spawn test runner command '{}': {}",
                        cmd_name, err
                    ),
                }
            }
        };

        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();

        let (stdout_tx, stdout_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();

        // محدودیت سقف بافر لاگ برای جلوگیری از Memory Exhaustion (512 KB)
        const MAX_LOG_BYTES: usize = 512 * 1024;

        if let Some(mut out) = stdout_pipe.take() {
            thread::spawn(move || {
                let mut buffer = Vec::new();
                let mut chunk = [0u8; 8192];
                while let Ok(n) = out.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    if buffer.len() < MAX_LOG_BYTES {
                        let to_copy = n.min(MAX_LOG_BYTES - buffer.len());
                        buffer.extend_from_slice(&chunk[..to_copy]);
                    }
                }
                let _ = stdout_tx.send(buffer);
            });
        } else {
            let _ = stdout_tx.send(Vec::new());
        }

        if let Some(mut err_stream) = stderr_pipe.take() {
            thread::spawn(move || {
                let mut buffer = Vec::new();
                let mut chunk = [0u8; 8192];
                while let Ok(n) = err_stream.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    if buffer.len() < MAX_LOG_BYTES {
                        let to_copy = n.min(MAX_LOG_BYTES - buffer.len());
                        buffer.extend_from_slice(&chunk[..to_copy]);
                    }
                }
                let _ = stderr_tx.send(buffer);
            });
        } else {
            let _ = stderr_tx.send(Vec::new());
        }

        // سقف مهلت اجرای تست‌ها (45 ثانیه)
        let timeout = Duration::from_secs(45);
        let start_time = Instant::now();
        let status: std::process::ExitStatus;

        loop {
            match child.try_wait() {
                Ok(Some(s)) => {
                    status = s;
                    break;
                }
                Ok(None) => {
                    if start_time.elapsed() > timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        let stdout_bytes = stdout_rx
                            .recv_timeout(Duration::from_millis(500))
                            .unwrap_or_default();
                        let stderr_bytes = stderr_rx
                            .recv_timeout(Duration::from_millis(500))
                            .unwrap_or_default();

                        return CandidateTestResult {
                            success: false,
                            exit_code: Some(124),
                            stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
                            stderr: format!(
                                "{}\n[Vora Resource Guard] Sandbox Timeout: Execution exceeded {:?}. Process killed safely.",
                                String::from_utf8_lossy(&stderr_bytes),
                                timeout
                            ),
                        };
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    return CandidateTestResult {
                        success: false,
                        exit_code: Some(1),
                        stdout: String::new(),
                        stderr: format!("Error waiting on test child process: {}", e),
                    };
                }
            }
        }

        let stdout_bytes = stdout_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or_default();
        let stderr_bytes = stderr_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or_default();

        let stdout_str = String::from_utf8_lossy(&stdout_bytes).to_string();
        let stderr_str = String::from_utf8_lossy(&stderr_bytes).to_string();

        CandidateTestResult {
            success: status.success(),
            exit_code: status.code(),
            stdout: stdout_str,
            stderr: stderr_str,
        }
    }
}

impl Drop for CandidateWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

pub struct ForgeToolRegistry;

impl ForgeToolRegistry {
    pub fn get_tool_definitions() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "find_symbol".to_string(),
                description: "Search for symbols (functions, structs, interfaces) in the workspace index".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Symbol name or path pattern" }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "get_impact_radius".to_string(),
                description: "Calculate direct and indirect dependents and affected test files for a symbol".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "symbol_id": { "type": "string", "description": "Unique symbol ID" }
                    },
                    "required": ["symbol_id"]
                }),
            },
            ToolDefinition {
                name: "get_symbol_context".to_string(),
                description: "Extract the exact code slice and related definitions for a symbol".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "symbol_name": { "type": "string", "description": "Name of the symbol" },
                        "repo_root": { "type": "string", "description": "Root path of repository", "default": "." }
                    },
                    "required": ["symbol_name"]
                }),
            },
            ToolDefinition {
                name: "apply_surgical_patch".to_string(),
                description: "Apply a precise byte-level patch to a specific symbol in a file with syntax check".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string" },
                        "symbol_id": { "type": "string" },
                        "new_symbol_code": { "type": "string" },
                        "dry_run": { "type": "boolean", "default": false }
                    },
                    "required": ["file_path", "symbol_id", "new_symbol_code"]
                }),
            },
            ToolDefinition {
                name: "rewire_imports".to_string(),
                description: "Rewire and update import and namespace references for a renamed or mutated symbol across dependent files".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string" },
                        "old_symbol_name": { "type": "string" },
                        "new_symbol_name": { "type": "string" },
                        "dry_run": { "type": "boolean", "default": false }
                    },
                    "required": ["file_path", "old_symbol_name", "new_symbol_name"]
                }),
            },
            ToolDefinition {
                name: "index_workspace".to_string(),
                description: "Perform incremental AST indexing of the entire workspace".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "root_dir": { "type": "string", "default": "." }
                    },
                    "required": ["root_dir"]
                }),
            },
            ToolDefinition {
                name: "get_current_head".to_string(),
                description: "Get the current git HEAD commit hash of the repository".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "repo_root": { "type": "string", "default": "." }
                    }
                }),
            },
        ]
    }

    pub fn execute_tool(
        request: ToolCallRequest,
        index_opt: Option<&mut CodeIndex>,
    ) -> ToolCallResponse {
        match request.tool.as_str() {
            "get_current_head" => {
                let repo_root = request
                    .arguments
                    .get("repo_root")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                match GitSSOT::open(repo_root) {
                    Ok(ssot) => match ssot.repo().head().and_then(|h| h.peel_to_commit()) {
                        Ok(commit) => ToolCallResponse {
                            success: true,
                            result: serde_json::Value::String(commit.id().to_string()),
                            error: None,
                        },
                        Err(_) => ToolCallResponse {
                            success: true,
                            result: serde_json::Value::String("HEAD".to_string()),
                            error: None,
                        },
                    },
                    Err(_) => ToolCallResponse {
                        success: true,
                        result: serde_json::Value::String("HEAD".to_string()),
                        error: None,
                    },
                }
            }
            "find_symbol" => {
                let query = request
                    .arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if let Some(index) = index_opt {
                    match Self::find_symbol(query, index) {
                        Ok(res) => ToolCallResponse {
                            success: true,
                            result: serde_json::to_value(res).unwrap_or_default(),
                            error: None,
                        },
                        Err(e) => ToolCallResponse {
                            success: false,
                            result: serde_json::Value::Null,
                            error: Some(e.to_string()),
                        },
                    }
                } else {
                    ToolCallResponse {
                        success: false,
                        result: serde_json::Value::Null,
                        error: Some("Index connection unavailable".to_string()),
                    }
                }
            }
            "get_impact_radius" => {
                let symbol_id = request
                    .arguments
                    .get("symbol_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if let Some(index) = index_opt {
                    match Self::get_impact_radius(symbol_id, index) {
                        Ok(res) => ToolCallResponse {
                            success: true,
                            result: serde_json::to_value(res).unwrap_or_default(),
                            error: None,
                        },
                        Err(e) => ToolCallResponse {
                            success: false,
                            result: serde_json::Value::Null,
                            error: Some(e.to_string()),
                        },
                    }
                } else {
                    ToolCallResponse {
                        success: false,
                        result: serde_json::Value::Null,
                        error: Some("Index connection unavailable".to_string()),
                    }
                }
            }
            "get_symbol_context" => {
                let symbol_name = request
                    .arguments
                    .get("symbol_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let repo_root = request
                    .arguments
                    .get("repo_root")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                match Self::get_symbol_context(symbol_name, repo_root) {
                    Some(slice) => ToolCallResponse {
                        success: true,
                        result: serde_json::to_value(slice).unwrap_or_default(),
                        error: None,
                    },
                    None => ToolCallResponse {
                        success: false,
                        result: serde_json::Value::Null,
                        error: Some(format!(
                            "Could not extract slice for symbol '{}'",
                            symbol_name
                        )),
                    },
                }
            }
            "apply_surgical_patch" => {
                let file_path = request
                    .arguments
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let symbol_id = request
                    .arguments
                    .get("symbol_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let new_symbol_code = request
                    .arguments
                    .get("new_symbol_code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let dry_run = request
                    .arguments
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                match Self::apply_surgical_patch(file_path, symbol_id, new_symbol_code, dry_run) {
                    Ok(res) => ToolCallResponse {
                        success: true,
                        result: serde_json::to_value(res).unwrap_or_default(),
                        error: None,
                    },
                    Err(e) => ToolCallResponse {
                        success: false,
                        result: serde_json::Value::Null,
                        error: Some(e),
                    },
                }
            }
            "rewire_imports" => {
                let file_path = request
                    .arguments
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let old_symbol_name = request
                    .arguments
                    .get("old_symbol_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let new_symbol_name = request
                    .arguments
                    .get("new_symbol_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let dry_run = request
                    .arguments
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                match Self::rewire_imports(file_path, old_symbol_name, new_symbol_name, dry_run) {
                    Ok(rewired_code) => ToolCallResponse {
                        success: true,
                        result: serde_json::json!({ "file_path": file_path, "rewired_content": rewired_code }),
                        error: None,
                    },
                    Err(e) => ToolCallResponse {
                        success: false,
                        result: serde_json::Value::Null,
                        error: Some(e),
                    },
                }
            }
            "index_workspace" => {
                let root_dir = request
                    .arguments
                    .get("root_dir")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                if let Some(index) = index_opt {
                    match Self::index_workspace(root_dir, index) {
                        Ok(count) => ToolCallResponse {
                            success: true,
                            result: serde_json::json!({ "indexed_files_count": count }),
                            error: None,
                        },
                        Err(e) => ToolCallResponse {
                            success: false,
                            result: serde_json::Value::Null,
                            error: Some(e.to_string()),
                        },
                    }
                } else {
                    ToolCallResponse {
                        success: false,
                        result: serde_json::Value::Null,
                        error: Some("Index connection unavailable".to_string()),
                    }
                }
            }
            _ => ToolCallResponse {
                success: false,
                result: serde_json::Value::Null,
                error: Some(format!("Unknown tool: {}", request.tool)),
            },
        }
    }

    /// بازنگری و سیم‌کشی مجدد خودکار ایمپورت‌ها در فایل مقصد (P11 Import & Namespace Re-wiring)
    pub fn rewire_imports(
        file_path: &str,
        old_symbol_name: &str,
        new_symbol_name: &str,
        dry_run: bool,
    ) -> std::result::Result<String, String> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("Target file does not exist: {}", file_path));
        }

        let original_content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

        let rewired = SurgicalPatcher::apply_import_rewiring(
            file_path,
            &original_content,
            old_symbol_name,
            new_symbol_name,
        )?;

        if !dry_run {
            fs::write(path, &rewired).map_err(|e| {
                format!("Failed to write rewired imports to '{}': {}", file_path, e)
            })?;
        }

        Ok(rewired)
    }

    /// ایندکس کامل فضای کاری به همراه ثبت اسنپ‌شات اتمیک و ساخت انتیتی‌های کانونیکال
    pub fn index_workspace(root_dir: &str, index: &mut CodeIndex) -> Result<usize> {
        let mut files = Vec::new();
        Self::collect_files(Path::new(root_dir), &mut files);

        let ws_id = index.get_or_create_workspace(root_dir)?;
        let mut file_contents = Vec::new();
        let mut indexed_count = 0;

        for file_path in &files {
            if let Ok(bytes) = std::fs::read(file_path) {
                let path_str = file_path.to_string_lossy().to_string();
                file_contents.push((path_str, bytes));
            }
        }

        let slice_ref: Vec<(&str, &[u8])> = file_contents
            .iter()
            .map(|(p, b)| (p.as_str(), b.as_slice()))
            .collect();

        // ۱. ایجاد اسنپ‌شات کانونیکال پایدار
        let snap_id = index.commit_identity_snapshot(&ws_id, None, &slice_ref)?;

        // ۲. استخراج و ایندکس تفکیک معنایی برای هر فایل
        for (path_str, bytes) in &file_contents {
            if let Ok(content) = std::str::from_utf8(bytes) {
                if index.index_file_incremental(path_str, content)? {
                    indexed_count += 1;
                }

                let fid = index.get_or_create_file_id(path_str);
                let parsed_semantics = AstEngine::parse_file_semantics(fid, path_str, content);

                for sym in &parsed_semantics.symbols {
                    let lineage = sym
                        .lineage_id
                        .clone()
                        .unwrap_or_else(LineageId::generate_persistent);
                    index.record_lineage_version(&ws_id, &snap_id, &lineage, &sym.id, Some(1.0))?;

                    let _kind_str = match sym.kind {
                        SymbolKind::Struct => "struct",
                        SymbolKind::Trait => "trait",
                        SymbolKind::Interface => "interface",
                        SymbolKind::Enum => "enum",
                        SymbolKind::TypeAlias => "type_alias",
                        SymbolKind::Method => "method",
                        _ => "function",
                    };

                    index.register_canonical_entity(
                        &ws_id,
                        &snap_id,
                        &lineage,
                        &sym.id,
                        &sym.name,
                        &sym.module_namespace,
                        &sym.canonical_signature,
                    )?;
                }
            }
        }

        Ok(indexed_count)
    }

    pub fn find_symbol(query: &str, index: &CodeIndex) -> Result<Vec<SymbolSearchResult>> {
        let conn = index.connection();
        let query_pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, file_path, canonical_signature, start_line, end_line
             FROM code_symbols
             WHERE name LIKE ?1 OR qualified_scope LIKE ?1 OR file_path LIKE ?1
             LIMIT 20",
        )?;

        let rows = stmt.query_map([&query_pattern], |row| {
            Ok(SymbolSearchResult {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                file_path: row.get(3)?,
                signature: row.get(4)?,
                start_line: row.get::<_, i64>(5)? as usize,
                end_line: row.get::<_, i64>(6)? as usize,
            })
        })?;

        let mut results = Vec::new();
        for r in rows.flatten() {
            results.push(r);
        }
        Ok(results)
    }

    pub fn get_impact_radius(symbol_id: &str, index: &CodeIndex) -> Result<ImpactAnalysisReport> {
        let strict_edges = SemanticResolver::resolve_strict_semantic_edges(index)?;
        let mut dep_graph = DependencyGraph::new();

        if !strict_edges.is_empty() {
            dep_graph.build_from_strict_semantic_edges(&strict_edges);
        } else {
            let semantic_graph = SemanticResolver::build_semantic_graph(index)?;
            dep_graph.build_from_semantic_graph(&semantic_graph);
        }

        Ok(dep_graph.analyze_impact(symbol_id))
    }

    pub fn get_symbol_context(symbol_name: &str, repo_root: &str) -> Option<CodeSlice> {
        let context = ProjectContext::load_from_repo(repo_root).ok()?;
        let slicer = ScopeSlicer::new(&context, repo_root);
        slicer.slice_for_symbol(symbol_name)
    }

    pub fn apply_surgical_patch(
        file_path: &str,
        symbol_id: &str,
        new_symbol_code: &str,
        dry_run: bool,
    ) -> std::result::Result<PatchResult, String> {
        SurgicalPatcher::apply_patch(file_path, symbol_id, new_symbol_code, dry_run)
    }

    pub fn apply_patch_to_overlay_sandbox(
        overlay: &mut CodeOverlayManager,
        file_id: FileId,
        file_path: &str,
        base_content: &str,
        symbol_id: &str,
        new_symbol_code: &str,
    ) -> std::result::Result<PatchResult, String> {
        SurgicalPatcher::apply_patch_to_overlay(
            overlay,
            file_id,
            file_path,
            base_content,
            symbol_id,
            new_symbol_code,
        )
    }

    /// P10.5-H3: بازایندکس‌گذاری کامل Candidate Workspace و استخراج تفاوت معنایی حقیقی از روی AST سندباکس
    pub fn evaluate_candidate_workspace_semantics(
        repo_root: &str,
        patches: &[TransactionPatch],
    ) -> std::result::Result<SemanticDiffReport, String> {
        let sandbox = CandidateWorkspace::create(Path::new(repo_root))
            .map_err(|e| format!("Failed to materialize candidate sandbox: {}", e))?;

        let mut old_snapshots = Vec::new();
        let mut new_snapshots = Vec::new();

        for patch in patches {
            let original_file = Path::new(repo_root).join(&patch.file_path);
            let candidate_file = sandbox.path().join(&patch.file_path);
            let fid = FileId::new(format!("fid_{}", patch.file_path));

            let orig_code = std::fs::read_to_string(&original_file)
                .map_err(|e| format!("Failed to read original file {:?}: {}", original_file, e))?;

            // اعمال پچ روی فایل درون سندباکس
            let patch_res = SurgicalPatcher::apply_patch_to_source(
                &patch.file_path,
                &orig_code,
                &patch.symbol_id,
                &patch.new_symbol_code,
                true,
            )?;

            let start_byte = patch_res.original_byte_range.0;
            let end_byte = patch_res.original_byte_range.1;

            let patched_content = if start_byte <= orig_code.len() && end_byte <= orig_code.len() {
                let before = &orig_code[..start_byte];
                let after = &orig_code[end_byte..];
                format!("{}{}{}", before, patch.new_symbol_code, after)
            } else {
                return Err(format!(
                    "Byte range mismatch for symbol '{}'",
                    patch.symbol_id
                ));
            };

            std::fs::write(&candidate_file, &patched_content).map_err(|e| {
                format!("Failed to write candidate file {:?}: {}", candidate_file, e)
            })?;

            // ایجاد اسنپ‌شات‌ها با مسیر نسبی یکسان و شناسه فایل مشترک
            old_snapshots.push(SemanticDiffer::create_file_snapshot_with_id(
                fid.clone(),
                &patch.file_path,
                &orig_code,
            ));

            new_snapshots.push(SemanticDiffer::create_file_snapshot_with_id(
                fid,
                &patch.file_path,
                &patched_content,
            ));
        }

        let diff_report = SemanticDiffer::diff_project_snapshots(&old_snapshots, &new_snapshots);

        Ok(diff_report)
    }

    /// اجرای تست در Candidate Workspace کاملاً ایزوله بدون هیچ‌گونه جهش روی دیسک اصلی
    pub fn forge_test_candidate_patch(
        repo_root: &str,
        file_path: &str,
        patched_content: &str,
        test_command: Option<&str>,
    ) -> std::result::Result<CandidateTestResult, String> {
        let sandbox = CandidateWorkspace::create(Path::new(repo_root))
            .map_err(|e| format!("Failed to create isolated candidate sandbox: {}", e))?;

        sandbox
            .apply_patch(file_path, patched_content)
            .map_err(|e| format!("Failed to apply patch in candidate workspace: {}", e))?;

        Ok(sandbox.run_tests(test_command))
    }

    /// اجرای تست چندفایلی در یک Candidate Workspace واحد و مشترک
    pub fn forge_test_multi_candidate_patches(
        repo_root: &str,
        file_patches: &[(&str, &str)],
        test_command: Option<&str>,
    ) -> std::result::Result<CandidateTestResult, String> {
        let sandbox = CandidateWorkspace::create(Path::new(repo_root))
            .map_err(|e| format!("Failed to create isolated candidate sandbox: {}", e))?;

        for (rel_path, content) in file_patches {
            sandbox.apply_patch(rel_path, content).map_err(|e| {
                format!("Failed to apply patch for '{}' in sandbox: {}", rel_path, e)
            })?;
        }

        Ok(sandbox.run_tests(test_command))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_atomic_patch_transaction(
        base_commit: &str,
        task_id: &str,
        overlay: &mut CodeOverlayManager,
        index: &mut CodeIndex,
        file_path: &str,
        base_content: &str,
        symbol_id: &str,
        new_symbol_code: &str,
        intent: &str,
    ) -> std::result::Result<TransactionCommitResult, String> {
        let mut tx_manager = ForgeTransactionManager::new(base_commit, task_id, overlay);
        tx_manager.stage_base_file(file_path, base_content);
        tx_manager.stage_patch(file_path, symbol_id, new_symbol_code);
        tx_manager.commit_transaction(index, intent)
    }

    pub fn execute_multi_patch_transaction(
        base_commit: &str,
        task_id: &str,
        overlay: &mut CodeOverlayManager,
        index: &mut CodeIndex,
        base_files: &[(&str, &str)],
        patches: &[TransactionPatch],
        intent: &str,
    ) -> std::result::Result<TransactionCommitResult, String> {
        let mut tx_manager = ForgeTransactionManager::new(base_commit, task_id, overlay);
        for (path, content) in base_files {
            tx_manager.stage_base_file(path, content);
        }
        for patch in patches {
            tx_manager.stage_patch(&patch.file_path, &patch.symbol_id, &patch.new_symbol_code);
        }
        tx_manager.commit_transaction(index, intent)
    }

    fn collect_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                    if dir_name != "target"
                        && dir_name != "node_modules"
                        && dir_name != "dist"
                        && !dir_name.starts_with('.')
                    {
                        Self::collect_files(&path, files);
                    }
                } else if let Some(ext) = path.extension() {
                    if ext == "rs" || ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx" {
                        files.push(path);
                    }
                }
            }
        }
    }
}
