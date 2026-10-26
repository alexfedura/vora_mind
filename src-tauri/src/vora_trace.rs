use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TraceNodeData {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub status: String,
    #[serde(default, rename = "whyAmIHere")]
    pub why_am_i_here: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default, rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(default, rename = "parentId")]
    pub parent_id: Option<String>,
    #[serde(default, rename = "gitCommit")]
    pub git_commit: Option<String>,
    #[serde(default, rename = "linkedCommits")]
    pub linked_commits: Vec<String>,
    #[serde(default)]
    pub alternatives: Vec<Alternative>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub next: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Alternative {
    pub title: String,
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TraceEdgeData {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TraceGraph {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub project: String,
    pub nodes: Vec<TraceNodeData>,
    pub edges: Vec<TraceEdgeData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectInfo {
    pub id: String,
    pub path: String,
    pub role: String,
    pub status: String,
    #[serde(default)]
    pub description: Option<String>,
    pub exists: bool,
    pub git_branch: Option<String>,
    pub git_head: Option<String>,
    pub git_dirty: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryYaml {
    #[serde(rename = "schemaVersion", default)]
    pub schema_version: Option<String>,
    pub workspace_root: String,
    pub projects: Vec<RegistryProject>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryProject {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitLogEntry {
    pub hash: String,
    pub short: String,
    pub date: String,
    pub message: String,
}

fn trace_root() -> Result<PathBuf, String> {
    // Resolve D:\Workspace\vora_trace\.vora\trace
    // Strategy: try exe dir, then cwd, then fallback.
    let candidates = [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf())),
    ];
    for c in candidates.into_iter().flatten() {
        // Walk up looking for trace.yaml
        let mut dir = c.clone();
        for _ in 0..6 {
            if dir.join("trace.yaml").exists() {
                let root = dir.join(".vora").join("trace");
                return Ok(root);
            }
            if let Some(parent) = dir.parent() {
                dir = parent.to_path_buf();
            } else {
                break;
            }
        }
    }
    // Fallback for dev mode
    Ok(PathBuf::from("D:/Workspace/vora_trace/.vora/trace"))
}

fn run_git(cwd: &std::path::Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn parse_decision_md(content: &str) -> Option<TraceNodeData> {
    // Very small YAML-frontmatter + markdown parser.
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after = &trimmed[3..];
    let end = after.find("\n---")?;
    let frontmatter = &after[..end];
    let body = &after[end + 4..];

    let mut id = String::new();
    let mut kind = "decision".to_string();
    let mut status = "pending".to_string();
    let mut created_at: Option<String> = None;
    let mut parent_id: Option<String> = None;
    let mut title = String::new();
    let mut reason: Option<String> = None;
    let mut objective: Option<String> = None;
    let mut why_am_i_here: Option<String> = None;
    let mut evidence: Vec<String> = Vec::new();

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("id:") {
            id = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("kind:") {
            kind = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("status:") {
            status = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("created:") {
            created_at = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("parent:") {
            parent_id = Some(v.trim().to_string());
        }
    }

    // Parse section headers from body
    let mut current_section = String::new();
    let mut buffer = String::new();
    for line in body.lines() {
        if line.starts_with("# ") && title.is_empty() {
            title = line.trim_start_matches("# ").trim().to_string();
        } else if line.starts_with("## ") {
            // flush previous
            flush_section(&current_section, &buffer, &mut why_am_i_here,
                &mut objective, &mut reason, &mut evidence);
            current_section = line.trim_start_matches("## ")
                .trim().to_lowercase();
            buffer.clear();
        } else {
            buffer.push_str(line);
            buffer.push('\n');
        }
    }
    flush_section(&current_section, &buffer, &mut why_am_i_here,
        &mut objective, &mut reason, &mut evidence);

    if id.is_empty() {
        return None;
    }

    Some(TraceNodeData {
        id,
        kind,
        title: if title.is_empty() { "Untitled".into() } else { title },
        status,
        why_am_i_here,
        objective,
        reason,
        created_at,
        parent_id,
        git_commit: None,
        linked_commits: Vec::new(),
        alternatives: Vec::new(),
        evidence,
        next: Vec::new(),
    })
}

fn flush_section(
    section: &str,
    buffer: &str,
    why: &mut Option<String>,
    obj: &mut Option<String>,
    reason: &mut Option<String>,
    evidence: &mut Vec<String>,
) {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return;
    }
    match section {
        "why am i here?" | "why am i here" =>
            { *why = Some(trimmed.to_string()); }
        "objective" =>
            { *obj = Some(trimmed.to_string()); }
        "reason" =>
            { *reason = Some(trimmed.to_string()); }
        "evidence" => {
            for line in trimmed.lines() {
                let l = line.trim().trim_start_matches('-').trim();
                if !l.is_empty() {
                    evidence.push(l.to_string());
                }
            }
        }
        _ => {}
    }
}

#[tauri::command]
fn load_graph() -> Result<TraceGraph, String> {
    let root = trace_root()?;
    let path = root.join("graph.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("read {}: {}", path.display(), e))?;
    let graph: TraceGraph = serde_json::from_str(&content)
        .map_err(|e| format!("parse graph.json: {}", e))?;
    Ok(graph)
}

#[tauri::command]
fn save_graph(graph: TraceGraph) -> Result<(), String> {
    let root = trace_root()?;
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("mkdir {}: {}", root.display(), e))?;
    let path = root.join("graph.json");
    let json = serde_json::to_string_pretty(&graph)
        .map_err(|e| format!("serialize: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(())
}

#[tauri::command]
fn save_decision_md(node: TraceNodeData) -> Result<String, String> {
    let root = trace_root()?;
    let dir = root.join("decisions");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("mkdir {}: {}", dir.display(), e))?;
    let path = dir.join(format!("{}.md", node.id));

    let mut md = String::new();
    md.push_str("---\n");
    md.push_str(&format!("id: {}\n", node.id));
    md.push_str(&format!("kind: {}\n", node.kind));
    md.push_str(&format!("status: {}\n", node.status));
    if let Some(c) = &node.created_at {
        md.push_str(&format!("created: {}\n", c));
    }
    if let Some(p) = &node.parent_id {
        md.push_str(&format!("parent: {}\n", p));
    }
    md.push_str("---\n\n");
    md.push_str(&format!("# {}\n\n", node.title));
    if let Some(w) = &node.why_am_i_here {
        md.push_str("## Why am I here?\n\n");
        md.push_str(w);
        md.push_str("\n\n");
    }
    if let Some(o) = &node.objective {
        md.push_str("## Objective\n\n");
        md.push_str(o);
        md.push_str("\n\n");
    }
    if !node.alternatives.is_empty() {
        md.push_str("## Alternatives\n\n");
        for a in &node.alternatives {
            md.push_str(&format!("- {} — {}", a.title, a.status));
            if let Some(r) = &a.reason {
                md.push_str(&format!(": {}", r));
            }
            md.push_str("\n");
        }
        md.push('\n');
    }
    if let Some(r) = &node.reason {
        md.push_str("## Reason\n\n");
        md.push_str(r);
        md.push_str("\n\n");
    }
    if !node.evidence.is_empty() {
        md.push_str("## Evidence\n\n");
        for e in &node.evidence {
            md.push_str(&format!("- {}\n", e));
        }
    }

    std::fs::write(&path, md)
        .map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn list_decisions() -> Result<Vec<String>, String> {
    let root = trace_root()?;
    let dir = root.join("decisions");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("read_dir {}: {}", dir.display(), e))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                out.push(stem.to_string());
            }
        }
    }
    out.sort();
    Ok(out)
}

#[tauri::command]
fn scan_ecosystem() -> Result<Vec<ProjectInfo>, String> {
    let root = trace_root()?;
    // trace.yaml lives one level up from .vora/trace
    let trace_yaml = root
        .parent()                    // .vora
        .and_then(|p| p.parent())     // vora_trace
        .map(|p| p.join("trace.yaml"))
        .ok_or_else(|| "cannot resolve trace.yaml".to_string())?;

    let content = std::fs::read_to_string(&trace_yaml)
        .map_err(|e| format!("read {}: {}", trace_yaml.display(), e))?;

    let reg: RegistryYaml = serde_yaml::from_str(&content)
        .map_err(|e| format!("parse trace.yaml: {}", e))?;

    let mut out = Vec::new();
    for p in &reg.projects {
        let path = std::path::PathBuf::from(&p.path);
        let exists = path.exists();

        let (branch, head, dirty) = if exists && path.join(".git").exists() {
            let b = run_git(&path, &["rev-parse", "--abbrev-ref", "HEAD"]);
            let h = run_git(&path, &["rev-parse", "--short", "HEAD"]);
            let d = run_git(&path, &["status", "--porcelain"])
                .map(|s| !s.is_empty());
            (b, h, d)
        } else {
            (None, None, None)
        };

        out.push(ProjectInfo {
            id: p.id.clone(),
            path: p.path.clone(),
            role: p.role.clone().unwrap_or_else(|| "-".to_string()),
            status: p.status.clone().unwrap_or_else(|| "-".to_string()),
            description: p.description.clone(),
            exists,
            git_branch: branch,
            git_head: head,
            git_dirty: dirty,
        });
    }
    Ok(out)
}

#[tauri::command]
fn git_log_for_project(project_path: String, limit: usize)
    -> Result<Vec<GitLogEntry>, String>
{
    let path = std::path::PathBuf::from(&project_path);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let n = format!("-{}", limit.max(1).min(100));
    let out = std::process::Command::new("git")
        .args([
            "log", &n,
            "--pretty=format:%H|%h|%aI|%s",
        ])
        .current_dir(&path)
        .output()
        .map_err(|e| format!("git exec: {}", e))?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() == 4 {
            entries.push(GitLogEntry {
                hash: parts[0].to_string(),
                short: parts[1].to_string(),
                date: parts[2].to_string(),
                message: parts[3].to_string(),
            });
        }
    }
    Ok(entries)
}

#[tauri::command]
fn read_all_decisions() -> Result<Vec<TraceNodeData>, String> {
    let root = trace_root()?;
    let dir = root.join("decisions");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("read_dir {}: {}", dir.display(), e))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let content = std::fs::read_to_string(&p).unwrap_or_default();
        if let Some(node) = parse_decision_md(&content) {
            out.push(node);
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

#[tauri::command]
fn find_commits_for_file(
    project_path: String,
    file_pattern: String,
    limit: usize,
) -> Result<Vec<GitLogEntry>, String> {
    let path = std::path::PathBuf::from(&project_path);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let n = format!("-{}", limit.max(1).min(50));
    let out = std::process::Command::new("git")
        .args(["log", &n, "--pretty=format:%H|%h|%aI|%s", "--", &file_pattern])
        .current_dir(&path)
        .output()
        .map_err(|e| format!("git exec: {}", e))?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() == 4 {
            entries.push(GitLogEntry {
                hash: parts[0].to_string(),
                short: parts[1].to_string(),
                date: parts[2].to_string(),
                message: parts[3].to_string(),
            });
        }
    }
    Ok(entries)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[tauri::command]
fn export_graph_json(dest_path: String) -> Result<String, String> {
    let root = trace_root()?;
    let src = root.join("graph.json");
    let content = std::fs::read_to_string(&src)
        .map_err(|e| format!("read {}: {}", src.display(), e))?;
    std::fs::write(&dest_path, &content)
        .map_err(|e| format!("write {}: {}", dest_path, e))?;
    Ok(dest_path)
}

#[tauri::command]
fn import_graph_json(src_path: String) -> Result<TraceGraph, String> {
    let content = std::fs::read_to_string(&src_path)
        .map_err(|e| format!("read {}: {}", src_path, e))?;
    let graph: TraceGraph = serde_json::from_str(&content)
        .map_err(|e| format!("parse: {}", e))?;
    Ok(graph)
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn recent_decisions(limit: usize) -> Result<Vec<TraceNodeData>, String> {
    let mut all = read_all_decisions()?;
    all.sort_by(|a, b| {
        let a_t = a.created_at.as_deref().unwrap_or("");
        let b_t = b.created_at.as_deref().unwrap_or("");
        b_t.cmp(a_t)
    });
    all.truncate(limit.max(1).min(50));
    Ok(all)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            load_graph,
            save_graph,
            save_decision_md,
            list_decisions,
            scan_ecosystem,
            git_log_for_project,
            read_all_decisions,
            find_commits_for_file,
            export_graph_json,
            import_graph_json,
            app_version,
            recent_decisions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


