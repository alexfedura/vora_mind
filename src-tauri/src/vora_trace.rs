use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

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

const LEGACY_TRACE_PROJECT_ROOT: &str = r"D:\workspace\vora_trace";

fn copy_file_if_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_file() || destination.exists() {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    std::fs::copy(source, destination).map_err(|error| {
        format!(
            "copy {} to {}: {error}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn copy_directory_if_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(destination)
        .map_err(|error| format!("create {}: {error}", destination.display()))?;
    let entries =
        std::fs::read_dir(source).map_err(|error| format!("read {}: {error}", source.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read {}: {error}", source.display()))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory_if_missing(&source_path, &destination_path)?;
        } else {
            copy_file_if_missing(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn migrate_legacy_data(source_root: &Path, app_data_root: &Path) -> Result<(), String> {
    if !source_root.is_dir() {
        return Ok(());
    }
    let destination_root = app_data_root.join("VoraTrace");
    let marker = destination_root.join(".legacy-migration-v1-complete");
    if marker.is_file() {
        return Ok(());
    }
    std::fs::create_dir_all(&destination_root)
        .map_err(|error| format!("create {}: {error}", destination_root.display()))?;
    copy_file_if_missing(
        &source_root.join("trace.yaml"),
        &destination_root.join("trace.yaml"),
    )?;
    copy_directory_if_missing(
        &source_root.join(".vora").join("trace"),
        &destination_root.join(".vora").join("trace"),
    )?;
    std::fs::write(&marker, "Vora Trace data copied to Vora Mind AppData.\n")
        .map_err(|error| format!("write {}: {error}", marker.display()))?;
    Ok(())
}

fn trace_root(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("resolve Vora Mind AppData: {error}"))?;
    migrate_legacy_data(Path::new(LEGACY_TRACE_PROJECT_ROOT), &app_data)?;
    Ok(app_data.join("VoraTrace").join(".vora").join("trace"))
}

fn trace_registry_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("resolve Vora Mind AppData: {error}"))?;
    migrate_legacy_data(Path::new(LEGACY_TRACE_PROJECT_ROOT), &app_data)?;
    Ok(app_data.join("VoraTrace").join("trace.yaml"))
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
            flush_section(
                &current_section,
                &buffer,
                &mut why_am_i_here,
                &mut objective,
                &mut reason,
                &mut evidence,
            );
            current_section = line.trim_start_matches("## ").trim().to_lowercase();
            buffer.clear();
        } else {
            buffer.push_str(line);
            buffer.push('\n');
        }
    }
    flush_section(
        &current_section,
        &buffer,
        &mut why_am_i_here,
        &mut objective,
        &mut reason,
        &mut evidence,
    );

    if id.is_empty() {
        return None;
    }

    Some(TraceNodeData {
        id,
        kind,
        title: if title.is_empty() {
            "Untitled".into()
        } else {
            title
        },
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
        "why am i here?" | "why am i here" => {
            *why = Some(trimmed.to_string());
        }
        "objective" => {
            *obj = Some(trimmed.to_string());
        }
        "reason" => {
            *reason = Some(trimmed.to_string());
        }
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
pub fn trace_load_graph(app: AppHandle) -> Result<TraceGraph, String> {
    let root = trace_root(&app)?;
    let path = root.join("graph.json");
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let graph: TraceGraph =
        serde_json::from_str(&content).map_err(|e| format!("parse graph.json: {}", e))?;
    Ok(graph)
}

#[tauri::command]
pub fn trace_save_graph(app: AppHandle, graph: TraceGraph) -> Result<(), String> {
    let root = trace_root(&app)?;
    std::fs::create_dir_all(&root).map_err(|e| format!("mkdir {}: {}", root.display(), e))?;
    let path = root.join("graph.json");
    let json = serde_json::to_string_pretty(&graph).map_err(|e| format!("serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(())
}

#[tauri::command]
pub fn trace_save_decision_md(app: AppHandle, node: TraceNodeData) -> Result<String, String> {
    if node.id.is_empty()
        || !node.id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
    {
        return Err("Decision ID contains unsupported characters".to_string());
    }
    let root = trace_root(&app)?;
    let dir = root.join("decisions");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {}", dir.display(), e))?;
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

    std::fs::write(&path, md).map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn trace_list_decisions(app: AppHandle) -> Result<Vec<String>, String> {
    let root = trace_root(&app)?;
    let dir = root.join("decisions");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("read_dir {}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read {}: {error}", dir.display()))?;
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
pub fn trace_scan_ecosystem(app: AppHandle) -> Result<Vec<ProjectInfo>, String> {
    let trace_yaml = trace_registry_path(&app)?;
    let content = std::fs::read_to_string(&trace_yaml)
        .map_err(|e| format!("read {}: {}", trace_yaml.display(), e))?;

    let reg: RegistryYaml =
        serde_yaml::from_str(&content).map_err(|e| format!("parse trace.yaml: {}", e))?;

    let mut out = Vec::new();
    for p in &reg.projects {
        let path = std::path::PathBuf::from(&p.path);
        let exists = path.exists();

        let (branch, head, dirty) = if exists && path.join(".git").exists() {
            let b = run_git(&path, &["rev-parse", "--abbrev-ref", "HEAD"]);
            let h = run_git(&path, &["rev-parse", "--short", "HEAD"]);
            let d = run_git(&path, &["status", "--porcelain"]).map(|s| !s.is_empty());
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
pub fn trace_git_log_for_project(
    project_path: String,
    limit: usize,
) -> Result<Vec<GitLogEntry>, String> {
    let path = std::path::PathBuf::from(&project_path);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let n = format!("-{}", limit.max(1).min(100));
    let out = std::process::Command::new("git")
        .args(["log", &n, "--pretty=format:%H|%h|%aI|%s"])
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
pub fn trace_read_all_decisions(app: AppHandle) -> Result<Vec<TraceNodeData>, String> {
    let root = trace_root(&app)?;
    let dir = root.join("decisions");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("read_dir {}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read {}: {error}", dir.display()))?;
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let content = std::fs::read_to_string(&p)
            .map_err(|error| format!("read {}: {error}", p.display()))?;
        if let Some(node) = parse_decision_md(&content) {
            out.push(node);
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

#[tauri::command]
pub fn trace_find_commits_for_file(
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
        .args([
            "log",
            &n,
            "--pretty=format:%H|%h|%aI|%s",
            "--",
            &file_pattern,
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
pub fn trace_export_graph_json(app: AppHandle, dest_path: String) -> Result<String, String> {
    let root = trace_root(&app)?;
    let src = root.join("graph.json");
    let content =
        std::fs::read_to_string(&src).map_err(|e| format!("read {}: {}", src.display(), e))?;
    std::fs::write(&dest_path, &content).map_err(|e| format!("write {}: {}", dest_path, e))?;
    Ok(dest_path)
}

#[tauri::command]
pub fn trace_import_graph_json(src_path: String) -> Result<TraceGraph, String> {
    let content =
        std::fs::read_to_string(&src_path).map_err(|e| format!("read {}: {}", src_path, e))?;
    let graph: TraceGraph = serde_json::from_str(&content).map_err(|e| format!("parse: {}", e))?;
    Ok(graph)
}

#[tauri::command]
pub fn trace_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn trace_recent_decisions(app: AppHandle, limit: usize) -> Result<Vec<TraceNodeData>, String> {
    let mut all = trace_read_all_decisions(app)?;
    all.sort_by(|a, b| {
        let a_t = a.created_at.as_deref().unwrap_or("");
        let b_t = b.created_at.as_deref().unwrap_or("");
        b_t.cmp(a_t)
    });
    all.truncate(limit.max(1).min(50));
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::migrate_legacy_data;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("vora-trace-{label}-{}", uuid::Uuid::now_v7()))
    }

    #[test]
    fn migration_copies_trace_files_once_without_modifying_the_source() {
        let source_root = temp_root("source");
        let app_data_root = temp_root("appdata");
        let source_graph = source_root.join(".vora").join("trace").join("graph.json");
        let source_decision = source_root
            .join(".vora")
            .join("trace")
            .join("decisions")
            .join("D-001.md");
        fs::create_dir_all(source_decision.parent().expect("decision parent"))
            .expect("create legacy decision directory");
        fs::create_dir_all(source_root.join(".vora").join("trace"))
            .expect("create legacy graph directory");
        fs::write(
            source_root.join("trace.yaml"),
            "schemaVersion: '1.0'\nprojects: []\n",
        )
        .expect("write legacy registry");
        fs::write(&source_graph, "{\"project\":\"legacy\"}").expect("write legacy graph");
        fs::write(&source_decision, "# Legacy decision\n").expect("write legacy decision");

        migrate_legacy_data(&source_root, &app_data_root).expect("migrate legacy data");
        let destination_root = app_data_root.join("VoraTrace");
        assert_eq!(
            fs::read(destination_root.join("trace.yaml")).expect("read migrated registry"),
            b"schemaVersion: '1.0'\nprojects: []\n"
        );
        assert_eq!(
            fs::read(
                destination_root
                    .join(".vora")
                    .join("trace")
                    .join("graph.json")
            )
            .expect("read migrated graph"),
            b"{\"project\":\"legacy\"}"
        );
        assert_eq!(
            fs::read(
                destination_root
                    .join(".vora")
                    .join("trace")
                    .join("decisions")
                    .join("D-001.md")
            )
            .expect("read migrated decision"),
            b"# Legacy decision\n"
        );

        fs::write(&source_graph, "{\"project\":\"updated-source\"}")
            .expect("update original graph");
        migrate_legacy_data(&source_root, &app_data_root)
            .expect("repeat migration after marker creation");
        assert_eq!(
            fs::read(&source_graph).expect("read unchanged source graph"),
            b"{\"project\":\"updated-source\"}"
        );
        assert_eq!(
            fs::read(
                destination_root
                    .join(".vora")
                    .join("trace")
                    .join("graph.json")
            )
            .expect("read unchanged migrated graph"),
            b"{\"project\":\"legacy\"}"
        );

        fs::remove_dir_all(source_root).expect("remove test source");
        fs::remove_dir_all(app_data_root).expect("remove test AppData");
    }

    #[test]
    fn migration_is_optional_when_the_legacy_project_directory_is_missing() {
        let source_root = temp_root("missing");
        let app_data_root = temp_root("empty-appdata");

        migrate_legacy_data(&source_root, &app_data_root)
            .expect("missing source must not block app startup");
        assert!(!app_data_root.exists());
    }
}
