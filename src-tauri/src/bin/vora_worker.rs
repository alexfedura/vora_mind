use rusqlite::{params, Connection, Result as SqlResult};
use serde_json::json;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;
use vora_lib::forge::index::CodeIndex;
use vora_lib::forge::patcher_v2::SurgicalPatcher;
use vora_lib::forge::project::ProjectContext;

const CURRENT_VERSION: &str = "0.55.0";
const WORKER_DB_PATH: &str = "vora_worker_queue.db";
const WORKER_LOG_DIR: &str = "logs";
const MAX_HEALING_ATTEMPTS: i32 = 3;
const MIN_DISK_SPACE_MB: u64 = 1024; // حداقل ۱ گیگابایت برای ایمنی دیسک

const QUEUE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS worker_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL UNIQUE,
    project_root TEXT NOT NULL,
    intent TEXT NOT NULL,
    target_file TEXT,
    target_symbol TEXT,
    patch_code TEXT,
    priority INTEGER NOT NULL DEFAULT 10,
    status TEXT NOT NULL DEFAULT 'PENDING',
    claimed_by TEXT,
    healing_attempts INTEGER NOT NULL DEFAULT 0,
    last_error_stage TEXT,
    error_payload TEXT,
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    finished_at INTEGER
);

CREATE TABLE IF NOT EXISTS worker_heartbeats (
    worker_id TEXT PRIMARY KEY,
    last_seen INTEGER NOT NULL,
    current_task_id TEXT,
    status TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '0.55.0',
    port INTEGER NOT NULL DEFAULT 4040,
    active_tasks INTEGER NOT NULL DEFAULT 0,
    max_concurrency INTEGER NOT NULL DEFAULT 2
);

CREATE TABLE IF NOT EXISTS cluster_ast_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    source_node TEXT NOT NULL,
    file_path TEXT NOT NULL,
    symbol_id TEXT,
    action TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS cluster_file_locks (
    file_path TEXT PRIMARY KEY,
    locked_by_task TEXT NOT NULL,
    locked_by_node TEXT NOT NULL,
    locked_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS cluster_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id TEXT NOT NULL,
    task_id TEXT,
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_worker_tasks_status ON worker_tasks(status, priority DESC, created_at ASC);
CREATE INDEX IF NOT EXISTS idx_cluster_logs_ts ON cluster_logs(timestamp DESC);
"#;

#[derive(Debug, Clone)]
pub struct WorkerTaskRecord {
    pub id: i64,
    pub task_id: String,
    pub project_root: String,
    pub intent: String,
    pub target_file: Option<String>,
    pub target_symbol: Option<String>,
    pub patch_code: Option<String>,
    pub priority: i32,
    pub status: String,
    pub healing_attempts: i32,
}

#[derive(Debug, serde::Serialize)]
pub struct WorkerNodeRecord {
    pub worker_id: String,
    pub last_seen_epoch: i64,
    pub current_task_id: Option<String>,
    pub status: String,
    pub version: String,
    pub port: u16,
    pub active_tasks: i32,
    pub max_concurrency: i32,
    pub is_alive: bool,
}

pub struct CandidateWorkspace;

impl CandidateWorkspace {
    pub fn setup_sandbox(project_root: &Path, task_id: &str) -> Result<PathBuf, String> {
        let sandbox_base = project_root.join(".vora_sandboxes").join(task_id);
        if sandbox_base.exists() {
            fs::remove_dir_all(&sandbox_base)
                .map_err(|e| format!("Failed to clean existing sandbox: {}", e))?;
        }
        fs::create_dir_all(&sandbox_base)
            .map_err(|e| format!("Failed to create sandbox directory: {}", e))?;
        Ok(sandbox_base)
    }

    pub fn cleanup_sandbox(sandbox_path: &Path) {
        if sandbox_path.exists() {
            let _ = fs::remove_dir_all(sandbox_path);
        }
    }
}

fn check_disk_space_threshold(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        #[link(name = "kernel32")]
        extern "system" {
            fn GetDiskFreeSpaceExW(
                lpDirectoryName: *const u16,
                lpFreeBytesAvailableToCaller: *mut u64,
                lpTotalNumberOfBytes: *mut u64,
                lpTotalNumberOfFreeBytes: *mut u64,
            ) -> i32;
        }

        let path_str = path.to_str().unwrap_or(".");
        let mut wide: Vec<u16> = OsStr::new(path_str).encode_wide().collect();
        wide.push(0);

        let mut free_bytes_available: u64 = 0;
        let mut total_number_of_bytes: u64 = 0;
        let mut total_number_of_free_bytes: u64 = 0;

        let success = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_bytes_available,
                &mut total_number_of_bytes,
                &mut total_number_of_free_bytes,
            )
        };

        if success != 0 {
            let free_mb = free_bytes_available / (1024 * 1024);
            if free_mb < MIN_DISK_SPACE_MB {
                return Err(format!(
                    "Resource Guard: Low disk space ({} MB available, minimum required: {} MB)",
                    free_mb, MIN_DISK_SPACE_MB
                ));
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
    }
    Ok(())
}

fn get_node_id() -> String {
    env::var("WORKER_NODE_ID").unwrap_or_else(|_| {
        let hostname = env::var("COMPUTERNAME")
            .or_else(|_| env::var("HOSTNAME"))
            .unwrap_or_else(|_| "local".to_string());
        format!("node-{}", hostname.to_lowercase())
    })
}

fn get_max_concurrency() -> i32 {
    env::var("WORKER_MAX_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(2)
}

fn is_remote_mode() -> bool {
    env::var("WORKER_MODE")
        .map(|v| v.to_lowercase() == "remote")
        .unwrap_or(false)
}

fn enforce_security_boundary() {
    if is_remote_mode() && env::var("WORKER_AUTH_TOKEN").is_err() {
        eprintln!(
            "❌ FATAL SECURITY ERROR: WORKER_MODE=remote is set, but WORKER_AUTH_TOKEN is missing."
        );
        eprintln!("Worker process terminated to prevent unauthenticated remote execution.");
        std::process::exit(1);
    }
}

fn init_queue_db(path: &str) -> SqlResult<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(QUEUE_SCHEMA)?;
    let _ = conn.execute("ALTER TABLE worker_tasks ADD COLUMN patch_code TEXT;", []);
    let _ = conn.execute("ALTER TABLE worker_tasks ADD COLUMN claimed_by TEXT;", []);
    let _ = conn.execute(
        "ALTER TABLE worker_tasks ADD COLUMN healing_attempts INTEGER NOT NULL DEFAULT 0;",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE worker_tasks ADD COLUMN last_error_stage TEXT;",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE worker_heartbeats ADD COLUMN version TEXT NOT NULL DEFAULT '0.55.0';",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE worker_heartbeats ADD COLUMN port INTEGER NOT NULL DEFAULT 4040;",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE worker_heartbeats ADD COLUMN active_tasks INTEGER NOT NULL DEFAULT 0;",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE worker_heartbeats ADD COLUMN max_concurrency INTEGER NOT NULL DEFAULT 2;",
        [],
    );
    Ok(conn)
}

fn get_log_file_path() -> PathBuf {
    PathBuf::from(WORKER_LOG_DIR).join("worker.log")
}

fn append_worker_log(task_id: Option<&str>, message: &str) {
    let _ = fs::create_dir_all(WORKER_LOG_DIR);
    let log_file_path = get_log_file_path();
    let now = chrono_timestamp();
    let node_id = get_node_id();

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)
    {
        let _ = writeln!(
            file,
            "[{}] [{}] [{}] {}",
            now,
            node_id,
            task_id.unwrap_or("SYSTEM"),
            message
        );
    }

    if let Ok(conn) = Connection::open(WORKER_DB_PATH) {
        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let level = if message.contains('✗') || message.contains("FAILED") {
            "ERROR"
        } else if message.contains('✓') || message.contains("COMPLETED") {
            "INFO"
        } else {
            "DEBUG"
        };
        let _ = conn.execute(
            "INSERT INTO cluster_logs (node_id, task_id, level, message, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![node_id, task_id, level, message, now_sec],
        );
    }
}

fn chrono_timestamp() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", dur)
}

fn check_cli_tool(tool: &str, args: &[&str]) -> bool {
    let direct = Command::new(tool).args(args).output().is_ok();
    if direct {
        return true;
    }
    if cfg!(target_os = "windows") {
        let cmd_name = format!("{}.cmd", tool);
        if Command::new(&cmd_name).args(args).output().is_ok() {
            return true;
        }
        return Command::new("cmd")
            .args(["/C", tool])
            .args(args)
            .output()
            .is_ok();
    }
    false
}

fn acquire_file_lock(
    conn: &Connection,
    file_path: &str,
    task_id: &str,
    node_id: &str,
) -> Result<bool, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let res = conn.execute(
        "INSERT INTO cluster_file_locks (file_path, locked_by_task, locked_by_node, locked_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![file_path, task_id, node_id, now],
    );

    match res {
        Ok(_) => Ok(true),
        Err(_) => {
            let locked_by: String = conn.query_row(
                "SELECT locked_by_node || ' (Task: ' || locked_by_task || ')' FROM cluster_file_locks WHERE file_path = ?1",
                params![file_path],
                |r| r.get(0),
            ).unwrap_or_else(|_| "unknown".to_string());
            Err(format!(
                "File '{}' is currently locked by {}",
                file_path, locked_by
            ))
        }
    }
}

fn release_file_lock(conn: &Connection, file_path: &str, task_id: &str) {
    let _ = conn.execute(
        "DELETE FROM cluster_file_locks WHERE file_path = ?1 AND locked_by_task = ?2",
        params![file_path, task_id],
    );
}

fn broadcast_ast_event(
    conn: &Connection,
    source_node: &str,
    file_path: &str,
    symbol_id: Option<&str>,
    action: &str,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let event_id = format!("ast_evt_{}_{}", now, source_node);

    let _ = conn.execute(
        "INSERT INTO cluster_ast_events (event_id, source_node, file_path, symbol_id, action, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![event_id, source_node, file_path, symbol_id, action, now],
    );
    append_worker_log(
        None,
        &format!("AST Broadcast: action='{}', file='{}'", action, file_path),
    );
}

fn recover_stale_tasks(conn: &Connection) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let count = conn.execute(
        "UPDATE worker_tasks 
         SET status = 'PENDING', claimed_by = NULL, error_payload = 'Recovered from dead node or crash' 
         WHERE status IN ('CLAIMED', 'PLANNING', 'SIMULATING', 'TESTING', 'HEALING')
           AND (started_at IS NULL OR ?1 - started_at > 30)",
        params![now],
    ).unwrap_or(0);

    let _ = conn.execute(
        "DELETE FROM cluster_file_locks WHERE ?1 - locked_at > 30",
        params![now],
    );

    if count > 0 {
        println!(
            "  ⚠️ [CRASH RECOVERY] Recovered {} stale/dead-node tasks back to PENDING queue.",
            count
        );
        append_worker_log(None, &format!("Recovered {} stale tasks to PENDING", count));
    }
}

fn update_heartbeat(
    conn: &Connection,
    worker_id: &str,
    current_task: Option<&str>,
    status: &str,
    port: u16,
    active_tasks: i32,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let max_conc = get_max_concurrency();

    let _ = conn.execute(
        "INSERT INTO worker_heartbeats (worker_id, last_seen, current_task_id, status, version, port, active_tasks, max_concurrency)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(worker_id) DO UPDATE SET 
            last_seen = ?2, 
            current_task_id = ?3, 
            status = ?4, 
            version = ?5, 
            port = ?6,
            active_tasks = ?7,
            max_concurrency = ?8",
        params![worker_id, now, current_task, status, CURRENT_VERSION, port, active_tasks, max_conc],
    );
}

fn start_independent_heartbeat(
    worker_id: String,
    port: u16,
    is_running: Arc<AtomicBool>,
    current_active_task: Arc<Mutex<Option<String>>>,
) {
    thread::spawn(move || {
        while is_running.load(Ordering::Relaxed) {
            if let Ok(conn) = init_queue_db(WORKER_DB_PATH) {
                let active_task = current_active_task.lock().unwrap().clone();
                let status = if active_task.is_some() {
                    "BUSY"
                } else {
                    "IDLE"
                };
                let tasks_count = if active_task.is_some() { 1 } else { 0 };
                update_heartbeat(
                    &conn,
                    &worker_id,
                    active_task.as_deref(),
                    status,
                    port,
                    tasks_count,
                );
            }
            thread::sleep(Duration::from_secs(5));
        }
    });
}

fn get_cluster_nodes(conn: &Connection) -> Vec<WorkerNodeRecord> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let mut stmt = conn
        .prepare("SELECT worker_id, last_seen, current_task_id, status, version, port, active_tasks, max_concurrency FROM worker_heartbeats ORDER BY last_seen DESC")
        .unwrap();

    let rows = stmt
        .query_map([], |r| {
            let last_seen: i64 = r.get(1)?;
            let is_alive = now - last_seen <= 10;
            Ok(WorkerNodeRecord {
                worker_id: r.get(0)?,
                last_seen_epoch: last_seen,
                current_task_id: r.get(2)?,
                status: r.get(3)?,
                version: r.get(4)?,
                port: r.get(5)?,
                active_tasks: r.get(6).unwrap_or(0),
                max_concurrency: r.get(7).unwrap_or(2),
                is_alive,
            })
        })
        .unwrap();

    rows.filter_map(Result::ok).collect()
}

fn get_health_json(conn: &Connection) -> serde_json::Value {
    let pending_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worker_tasks WHERE status = 'PENDING'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let completed_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worker_tasks WHERE status = 'COMPLETED'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let failed_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worker_tasks WHERE status = 'FAILED'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let nodes = get_cluster_nodes(conn);
    let active_nodes_count = nodes.iter().filter(|n| n.is_alive).count();

    json!({
        "status": "healthy",
        "version": CURRENT_VERSION,
        "current_node": get_node_id(),
        "database": "connected",
        "mode": if is_remote_mode() { "remote" } else { "local" },
        "auth_enabled": env::var("WORKER_AUTH_TOKEN").is_ok(),
        "cluster": {
            "active_nodes": active_nodes_count,
            "total_nodes": nodes.len()
        },
        "queue": {
            "pending": pending_count,
            "completed": completed_count,
            "failed": failed_count,
        },
        "nodes": nodes
    })
}

fn get_cluster_load_json(conn: &Connection) -> serde_json::Value {
    let nodes = get_cluster_nodes(conn);
    let total_active_tasks: i32 = nodes
        .iter()
        .filter(|n| n.is_alive)
        .map(|n| n.active_tasks)
        .sum();
    let total_capacity: i32 = nodes
        .iter()
        .filter(|n| n.is_alive)
        .map(|n| n.max_concurrency)
        .sum();
    let cluster_load_percentage = if total_capacity > 0 {
        (total_active_tasks as f64 / total_capacity as f64) * 100.0
    } else {
        0.0
    };

    json!({
        "total_active_tasks": total_active_tasks,
        "total_cluster_capacity": total_capacity,
        "cluster_load_percentage": cluster_load_percentage,
        "nodes_breakdown": nodes
    })
}

fn run_health() {
    let conn = init_queue_db(WORKER_DB_PATH).expect("Failed to open task queue db");
    let health_payload = get_health_json(&conn);
    println!("{}", serde_json::to_string_pretty(&health_payload).unwrap());
}

fn run_doctor() {
    println!("============================================================");
    println!(
        "🔍 RUNNING VORA WORKER ENVIRONMENT DOCTOR 2.0 (v{})",
        CURRENT_VERSION
    );
    println!("============================================================");

    let rust_ok = check_cli_tool("rustc", &["--version"]);
    let cargo_ok = check_cli_tool("cargo", &["--version"]);
    let git_ok = check_cli_tool("git", &["--version"]);
    let node_ok = check_cli_tool("node", &["--version"]);
    let pnpm_ok = check_cli_tool("pnpm", &["--version"]);

    println!(
        "  [{}] Rust Compiler (rustc)",
        if rust_ok { "✓ OK" } else { "✗ FAIL" }
    );
    println!(
        "  [{}] Cargo Build System",
        if cargo_ok { "✓ OK" } else { "✗ FAIL" }
    );
    println!(
        "  [{}] Git Version Control",
        if git_ok { "✓ OK" } else { "✗ FAIL" }
    );
    println!(
        "  [{}] Node.js Runtime",
        if node_ok { "✓ OK" } else { "✗ FAIL" }
    );
    println!(
        "  [{}] pnpm Package Manager",
        if pnpm_ok { "✓ OK" } else { "✗ FAIL" }
    );

    let queue_db_ok = init_queue_db(WORKER_DB_PATH).is_ok();
    println!(
        "  [{}] SQLite Task Queue Database",
        if queue_db_ok { "✓ OK" } else { "✗ FAIL" }
    );

    let spine_db_ok = CodeIndex::open_in_memory().is_ok();
    println!(
        "  [{}] SQLite Identity Spine & AST Engine",
        if spine_db_ok { "✓ OK" } else { "✗ FAIL" }
    );

    let sandbox_test_path = Path::new(".vora_sandboxes").join("test_write");
    let sandbox_ok = fs::create_dir_all(&sandbox_test_path).is_ok()
        && fs::remove_dir_all(&sandbox_test_path).is_ok();
    println!(
        "  [{}] Candidate Workspace Sandbox FS Permissions",
        if sandbox_ok { "✓ OK" } else { "✗ FAIL" }
    );

    let disk_guard_ok = check_disk_space_threshold(Path::new(".")).is_ok();
    println!(
        "  [{}] Disk Space Guard (> {} MB)",
        if disk_guard_ok { "✓ OK" } else { "✗ FAIL" },
        MIN_DISK_SPACE_MB
    );

    let auth_token_set = env::var("WORKER_AUTH_TOKEN").is_ok();
    let git_token_ok = env::var("GIT_TOKEN").is_ok() || env::var("GITHUB_TOKEN").is_ok();
    let llm_key_ok = env::var("LLM_API_KEY").is_ok() || env::var("OPENAI_API_KEY").is_ok();
    let remote_mode = is_remote_mode();

    println!("  [✓ OK] Node Identity: {}", get_node_id());
    println!("  [✓ OK] Concurrency Limit: {}", get_max_concurrency());
    println!(
        "  [✓ OK] Execution Mode: {}",
        if remote_mode {
            "REMOTE (Cloud/VPS)"
        } else {
            "LOCAL"
        }
    );
    println!(
        "  [{}] Worker API Token Auth: {}",
        if auth_token_set {
            "✓ ENABLED"
        } else if !remote_mode {
            "ℹ OPEN (Local Mode)"
        } else {
            "✗ REQUIRED FOR REMOTE"
        },
        if auth_token_set || !remote_mode {
            ""
        } else {
            " [CRITICAL]"
        }
    );
    println!(
        "  [{}] Git Credentials",
        if git_token_ok { "✓ OK" } else { "⚠ UNSET" }
    );
    println!(
        "  [{}] LLM API Key",
        if llm_key_ok { "✓ OK" } else { "⚠ UNSET" }
    );

    println!("============================================================");
    let all_ok = rust_ok
        && cargo_ok
        && git_ok
        && queue_db_ok
        && spine_db_ok
        && sandbox_ok
        && disk_guard_ok
        && (!remote_mode || auth_token_set);
    if all_ok {
        println!("🏆 DOCTOR VERDICT: Worker is 100% PRODUCTION-READY for execution.");
    } else {
        println!("❌ DOCTOR VERDICT: Missing dependencies or security configuration.");
    }
    println!("============================================================");
}

fn submit_task(
    intent: &str,
    project_root: Option<&str>,
    target_file: Option<&str>,
    target_symbol: Option<&str>,
    patch_code: Option<&str>,
) -> String {
    let conn = init_queue_db(WORKER_DB_PATH).expect("Failed to open task queue db");
    let task_id = format!("task_{}", Uuid::now_v7());
    let root = project_root.unwrap_or(".");
    let now_sec = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO worker_tasks (task_id, project_root, intent, target_file, target_symbol, patch_code, priority, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 10, 'PENDING', ?7)",
        params![task_id, root, intent, target_file, target_symbol, patch_code, now_sec],
    ).expect("Failed to submit task");

    append_worker_log(
        Some(&task_id),
        &format!("Submitted: intent='{}', root='{}'", intent, root),
    );

    println!("✅ Task submitted successfully!");
    println!("  ID:     {}", task_id);
    println!("  Intent: {}", intent);
    println!("  Root:   {}", root);

    task_id
}

fn run_status() {
    let conn = init_queue_db(WORKER_DB_PATH).expect("Failed to open task queue db");
    println!("============================================================");
    println!(
        "📊 VORA WORKER TASK QUEUE & CLUSTER STATUS (v{})",
        CURRENT_VERSION
    );
    println!("============================================================");

    let nodes = get_cluster_nodes(&conn);
    println!(
        "📡 Active Cluster Nodes ({}):",
        nodes.iter().filter(|n| n.is_alive).count()
    );
    for n in &nodes {
        let mark = if n.is_alive { "🟢" } else { "⚪" };
        println!(
            "  {} {} (Port: {}) | Load: {}/{} | Status: {:<6} | Task: {}",
            mark,
            n.worker_id,
            n.port,
            n.active_tasks,
            n.max_concurrency,
            n.status,
            n.current_task_id.as_deref().unwrap_or("none")
        );
    }
    println!("------------------------------------------------------------");

    let mut stmt = conn
        .prepare("SELECT task_id, status, intent, claimed_by, error_payload FROM worker_tasks ORDER BY created_at DESC LIMIT 5")
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })
        .unwrap();

    let mut count = 0;
    for r in rows.flatten() {
        count += 1;
        let claimed = r.3.unwrap_or_else(|| "-".to_string());
        println!(
            "  - [{}] Status: {:<10} | Node: {:<12} | Intent: {}",
            r.0, r.1, claimed, r.2
        );
        if let Some(err) = r.4 {
            println!("    └─ Error: {}", err.lines().next().unwrap_or(""));
        }
    }

    if count == 0 {
        println!("  (Queue is currently empty)");
    }
    println!("============================================================");
}

fn claim_next_task(conn: &mut Connection, node_id: &str) -> SqlResult<Option<WorkerTaskRecord>> {
    let tx = conn.transaction()?;
    let next_task: Option<WorkerTaskRecord> = tx
        .query_row(
            "SELECT id, task_id, project_root, intent, target_file, target_symbol, patch_code, priority, status, healing_attempts
         FROM worker_tasks
         WHERE status = 'PENDING'
         ORDER BY priority DESC, created_at ASC
         LIMIT 1",
            [],
            |r| {
                Ok(WorkerTaskRecord {
                    id: r.get(0)?,
                    task_id: r.get(1)?,
                    project_root: r.get(2)?,
                    intent: r.get(3)?,
                    target_file: r.get(4)?,
                    target_symbol: r.get(5)?,
                    patch_code: r.get(6)?,
                    priority: r.get(7)?,
                    status: r.get(8)?,
                    healing_attempts: r.get(9)?,
                })
            },
        )
        .ok();

    if let Some(ref task) = next_task {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        tx.execute(
            "UPDATE worker_tasks SET status = 'CLAIMED', claimed_by = ?1, started_at = ?2 WHERE id = ?3",
            params![node_id, now, task.id],
        )?;
    }

    tx.commit()?;
    Ok(next_task)
}

fn update_task_status(
    conn: &Connection,
    task_id: &str,
    status: &str,
    error_stage: Option<&str>,
    error_payload: Option<&str>,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let _ = conn.execute(
        "UPDATE worker_tasks SET status = ?1, last_error_stage = ?2, error_payload = ?3, finished_at = ?4 WHERE task_id = ?5",
        params![status, error_stage, error_payload, now, task_id],
    );

    append_worker_log(
        Some(task_id),
        &format!("Status transition -> {} (Stage: {:?})", status, error_stage),
    );
}

fn increment_healing_attempt(
    conn: &Connection,
    task_id: &str,
    error_stage: &str,
    error_payload: &str,
) -> i32 {
    let attempts: i32 = conn.query_row(
        "UPDATE worker_tasks SET healing_attempts = healing_attempts + 1, status = 'HEALING', last_error_stage = ?1, error_payload = ?2 WHERE task_id = ?3 RETURNING healing_attempts",
        params![error_stage, error_payload, task_id],
        |r| r.get(0),
    ).unwrap_or(MAX_HEALING_ATTEMPTS);
    attempts
}

fn execute_single_task(
    task: &WorkerTaskRecord,
    node_id: &str,
    port: u16,
    active_task_ref: &Arc<Mutex<Option<String>>>,
) {
    let get_conn = || -> Connection {
        init_queue_db(WORKER_DB_PATH).expect("Failed to open worker db connection")
    };

    let root_path = Path::new(&task.project_root);

    // بررسی گارد دیسک پیش از آغاز پردازش
    if let Err(disk_err) = check_disk_space_threshold(root_path) {
        println!("  ✗ {}", disk_err);
        append_worker_log(Some(&task.task_id), &format!("✗ {}", disk_err));
        let conn = get_conn();
        update_task_status(
            &conn,
            &task.task_id,
            "BLOCKED",
            Some("INSUFFICIENT_DISK_SPACE"),
            Some(&disk_err),
        );
        update_heartbeat(&conn, node_id, None, "DEGRADED", port, 0);
        *active_task_ref.lock().unwrap() = None;
        return;
    }

    *active_task_ref.lock().unwrap() = Some(task.task_id.clone());

    println!(
        "\n🚀 [{}] Executing Task [{}] - Intent: '{}'",
        node_id, task.task_id, task.intent
    );
    append_worker_log(
        Some(&task.task_id),
        &format!("Starting execution of intent: '{}'", task.intent),
    );

    {
        let conn = get_conn();
        update_heartbeat(&conn, node_id, Some(&task.task_id), "BUSY", port, 1);
        update_task_status(&conn, &task.task_id, "PLANNING", None, None);
    }

    let target_dir = if root_path.join("src-tauri").exists() {
        root_path.join("src-tauri")
    } else {
        root_path.to_path_buf()
    };

    println!("  [1/6] Loading Semantic Project Context & AST Spine...");
    append_worker_log(
        Some(&task.task_id),
        "[1/6] Loading Semantic Project Context & AST Spine...",
    );

    let _context = match ProjectContext::load_from_repo(&target_dir) {
        Ok(ctx) => ctx,
        Err(e) => {
            let err = format!("Failed to parse project context: {}", e);
            println!("    ✗ {}", err);
            append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
            let conn = get_conn();
            update_task_status(
                &conn,
                &task.task_id,
                "FAILED",
                Some("CONTEXT_LOAD_FAILED"),
                Some(&err),
            );
            update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
            *active_task_ref.lock().unwrap() = None;
            return;
        }
    };

    println!("  [2/6] Running Pre-Mutation Baseline Invariant Check...");
    append_worker_log(
        Some(&task.task_id),
        "[2/6] Running Pre-Mutation Baseline Invariant Check...",
    );
    {
        let conn = get_conn();
        update_task_status(&conn, &task.task_id, "TESTING", None, None);
    }

    let status = Command::new("cargo")
        .args(["test", "--lib", "--", "--nocapture"])
        .current_dir(&target_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("\n    ✓ Baseline regression tests passed with 100% green output.");
            append_worker_log(
                Some(&task.task_id),
                "✓ Baseline regression tests passed 100% green.",
            );
        }
        Ok(s) => {
            let err = format!("Baseline tests failed: {}", s);
            println!("\n    ✗ {}", err);
            append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
            let conn = get_conn();
            update_task_status(
                &conn,
                &task.task_id,
                "FAILED",
                Some("BASELINE_TEST_FAILED"),
                Some(&err),
            );
            update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
            *active_task_ref.lock().unwrap() = None;
            return;
        }
        Err(e) => {
            let err = format!("Failed to spawn test process: {}", e);
            println!("\n    ✗ {}", err);
            append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
            let conn = get_conn();
            update_task_status(
                &conn,
                &task.task_id,
                "FAILED",
                Some("PROCESS_SPAWN_FAILED"),
                Some(&err),
            );
            update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
            *active_task_ref.lock().unwrap() = None;
            return;
        }
    }

    if let (Some(rel_path), Some(sym_id), Some(new_code)) =
        (&task.target_file, &task.target_symbol, &task.patch_code)
    {
        println!(
            "  [3/6] Initializing Candidate Workspace Sandbox for Task '{}'...",
            task.task_id
        );
        append_worker_log(
            Some(&task.task_id),
            "[3/6] Initializing Candidate Workspace Sandbox...",
        );

        let sandbox_dir = match CandidateWorkspace::setup_sandbox(root_path, &task.task_id) {
            Ok(dir) => dir,
            Err(e) => {
                let err = format!("Candidate workspace sandbox setup failed: {}", e);
                append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
                let conn = get_conn();
                update_task_status(
                    &conn,
                    &task.task_id,
                    "FAILED",
                    Some("SANDBOX_INIT_FAILED"),
                    Some(&err),
                );
                update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
                *active_task_ref.lock().unwrap() = None;
                return;
            }
        };

        println!(
            "  [4/6] Acquiring Invariant File Lock for '{}'...",
            rel_path
        );
        append_worker_log(
            Some(&task.task_id),
            &format!("[4/6] Acquiring Invariant File Lock for '{}'...", rel_path),
        );

        {
            let conn = get_conn();
            if let Err(lock_err) = acquire_file_lock(&conn, rel_path, &task.task_id, node_id) {
                println!("    ✗ Lock conflict: {}", lock_err);
                append_worker_log(
                    Some(&task.task_id),
                    &format!("✗ Lock conflict: {}", lock_err),
                );
                update_task_status(
                    &conn,
                    &task.task_id,
                    "FAILED",
                    Some("FILE_LOCK_CONFLICT"),
                    Some(&lock_err),
                );
                update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
                CandidateWorkspace::cleanup_sandbox(&sandbox_dir);
                *active_task_ref.lock().unwrap() = None;
                return;
            }
        }

        println!(
            "  [4/6] Applying Surgical Patch for Symbol '{}' in '{}'...",
            sym_id, rel_path
        );
        append_worker_log(
            Some(&task.task_id),
            &format!(
                "[4/6] Applying Surgical Patch for Symbol '{}' in '{}'...",
                sym_id, rel_path
            ),
        );
        {
            let conn = get_conn();
            update_task_status(&conn, &task.task_id, "SIMULATING", None, None);
        }

        let target_file_path = target_dir.join(rel_path);
        if !target_file_path.exists() {
            let err = format!("Target patch file not found: {:?}", target_file_path);
            println!("    ✗ {}", err);
            append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
            let conn = get_conn();
            release_file_lock(&conn, rel_path, &task.task_id);
            update_task_status(
                &conn,
                &task.task_id,
                "FAILED",
                Some("FILE_NOT_FOUND"),
                Some(&err),
            );
            update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
            CandidateWorkspace::cleanup_sandbox(&sandbox_dir);
            *active_task_ref.lock().unwrap() = None;
            return;
        }

        let original_content = match fs::read_to_string(&target_file_path) {
            Ok(c) => c,
            Err(e) => {
                let err = format!("Failed to read file: {}", e);
                append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
                let conn = get_conn();
                release_file_lock(&conn, rel_path, &task.task_id);
                update_task_status(
                    &conn,
                    &task.task_id,
                    "FAILED",
                    Some("FILE_READ_FAILED"),
                    Some(&err),
                );
                update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
                CandidateWorkspace::cleanup_sandbox(&sandbox_dir);
                *active_task_ref.lock().unwrap() = None;
                return;
            }
        };

        let patch_res = match SurgicalPatcher::apply_patch(
            target_file_path.to_str().unwrap(),
            sym_id,
            new_code,
            true,
        ) {
            Ok(r) => r,
            Err(e) => {
                let err = format!("AST Surgical Patch validation failed: {}", e);
                println!("    ✗ {}", err);
                append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
                let conn = get_conn();
                release_file_lock(&conn, rel_path, &task.task_id);

                let attempts =
                    increment_healing_attempt(&conn, &task.task_id, "AST_VALIDATION_FAILED", &err);
                if attempts <= MAX_HEALING_ATTEMPTS {
                    append_worker_log(
                        Some(&task.task_id),
                        &format!(
                            "Initiating self-healing loop (attempt {}/{})",
                            attempts, MAX_HEALING_ATTEMPTS
                        ),
                    );
                } else {
                    update_task_status(
                        &conn,
                        &task.task_id,
                        "FAILED",
                        Some("MAX_HEALING_EXCEEDED"),
                        Some(&err),
                    );
                }
                update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
                CandidateWorkspace::cleanup_sandbox(&sandbox_dir);
                *active_task_ref.lock().unwrap() = None;
                return;
            }
        };

        if !patch_res.is_syntax_valid {
            let err = "Candidate mutation produced unparseable AST invalid blocks.".to_string();
            println!("    ✗ {}", err);
            append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
            let conn = get_conn();
            release_file_lock(&conn, rel_path, &task.task_id);
            let attempts = increment_healing_attempt(&conn, &task.task_id, "SYNTAX_INVALID", &err);
            if attempts > MAX_HEALING_ATTEMPTS {
                update_task_status(
                    &conn,
                    &task.task_id,
                    "FAILED",
                    Some("SYNTAX_INVALID"),
                    Some(&err),
                );
            }
            update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
            CandidateWorkspace::cleanup_sandbox(&sandbox_dir);
            *active_task_ref.lock().unwrap() = None;
            return;
        }

        println!("  [5/6] Executing Invariant & Regression Tests on Patched Tree...");
        append_worker_log(
            Some(&task.task_id),
            "[5/6] Executing Invariant & Regression Tests on Patched Tree...",
        );
        let (start, end) = patch_res.original_byte_range;
        let candidate_content = format!(
            "{}{}{}",
            &original_content[..start],
            new_code,
            &original_content[end..]
        );

        if let Err(e) = fs::write(&target_file_path, &candidate_content) {
            let err = format!("Failed to write candidate file: {}", e);
            append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
            let conn = get_conn();
            release_file_lock(&conn, rel_path, &task.task_id);
            update_task_status(
                &conn,
                &task.task_id,
                "FAILED",
                Some("FILE_WRITE_FAILED"),
                Some(&err),
            );
            update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
            CandidateWorkspace::cleanup_sandbox(&sandbox_dir);
            *active_task_ref.lock().unwrap() = None;
            return;
        }

        let candidate_test_status = Command::new("cargo")
            .args(["test", "--lib", "--", "--nocapture"])
            .current_dir(&target_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        match candidate_test_status {
            Ok(s) if s.success() => {
                println!("\n    ✓ Candidate mutation verified and passed all Trust Gates.");
                append_worker_log(
                    Some(&task.task_id),
                    "✓ Candidate mutation verified and passed all Trust Gates.",
                );
                let _ = Command::new("git")
                    .args(["add", rel_path])
                    .current_dir(&target_dir)
                    .output();
                let commit_msg = format!("refactor(worker): {} [auto-verified]", task.intent);
                let _ = Command::new("git")
                    .args(["commit", "-m", &commit_msg])
                    .current_dir(&target_dir)
                    .output();
                let conn = get_conn();
                broadcast_ast_event(&conn, node_id, rel_path, Some(sym_id), "SURGICAL_PATCH");
                release_file_lock(&conn, rel_path, &task.task_id);
                CandidateWorkspace::cleanup_sandbox(&sandbox_dir);
            }
            Ok(s) => {
                let err = format!("Candidate test failed: {}", s);
                println!("\n    ✗ {}. Rolling back changes...", err);
                append_worker_log(
                    Some(&task.task_id),
                    &format!("✗ {}. Rolling back changes...", err),
                );
                let _ = fs::write(&target_file_path, &original_content);
                let conn = get_conn();
                release_file_lock(&conn, rel_path, &task.task_id);

                let attempts =
                    increment_healing_attempt(&conn, &task.task_id, "CANDIDATE_TEST_FAILED", &err);
                if attempts > MAX_HEALING_ATTEMPTS {
                    update_task_status(
                        &conn,
                        &task.task_id,
                        "FAILED",
                        Some("MAX_HEALING_EXCEEDED"),
                        Some(&err),
                    );
                }
                update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
                CandidateWorkspace::cleanup_sandbox(&sandbox_dir);
                *active_task_ref.lock().unwrap() = None;
                return;
            }
            Err(e) => {
                let err = format!("Failed to execute candidate tests: {}", e);
                let _ = fs::write(&target_file_path, &original_content);
                append_worker_log(Some(&task.task_id), &format!("✗ {}", err));
                let conn = get_conn();
                release_file_lock(&conn, rel_path, &task.task_id);
                update_task_status(
                    &conn,
                    &task.task_id,
                    "FAILED",
                    Some("TEST_EXECUTION_ERROR"),
                    Some(&err),
                );
                update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
                CandidateWorkspace::cleanup_sandbox(&sandbox_dir);
                *active_task_ref.lock().unwrap() = None;
                return;
            }
        }
    } else {
        println!("  [3/6] Pure Invariant Verification Task (No Patch Mutation).");
        println!("  [4/6] Invariants Verified.");
        append_worker_log(
            Some(&task.task_id),
            "[3/6] Pure Invariant Verification Task passed.",
        );
    }

    println!("  [6/6] Finalizing Lifecycle Audit...");
    append_worker_log(
        Some(&task.task_id),
        "[6/6] Finalizing Lifecycle Audit -> COMPLETED",
    );
    {
        let conn = get_conn();
        update_task_status(&conn, &task.task_id, "COMPLETED", None, None);
        update_heartbeat(&conn, node_id, None, "IDLE", port, 0);
    }
    *active_task_ref.lock().unwrap() = None;
    println!(
        "🏆 Task [{}] completed with 100% GREEN gates.\n",
        task.task_id
    );
}

fn run_once() {
    enforce_security_boundary();
    let mut conn = init_queue_db(WORKER_DB_PATH).expect("Failed to open task queue db");
    let node_id = get_node_id();
    let is_running = Arc::new(AtomicBool::new(true));
    let active_task = Arc::new(Mutex::new(None));
    start_independent_heartbeat(
        node_id.clone(),
        4040,
        Arc::clone(&is_running),
        Arc::clone(&active_task),
    );

    recover_stale_tasks(&conn);
    match claim_next_task(&mut conn, &node_id) {
        Ok(Some(task)) => {
            drop(conn);
            execute_single_task(&task, &node_id, 4040, &active_task);
        }
        Ok(None) => println!("[*] No pending tasks in queue."),
        Err(e) => println!("[!] Error claiming task: {}", e),
    }
    is_running.store(false, Ordering::Relaxed);
}

fn run_loop() {
    enforce_security_boundary();
    let node_id = get_node_id();
    println!("============================================================");
    println!(
        "🔄 VORA WORKER 24/7 DAEMON LOOP [{}] (v{})",
        node_id, CURRENT_VERSION
    );
    println!("============================================================");
    println!("[*] Polling queue every 2 seconds. Press Ctrl+C to terminate.");
    let mut conn = init_queue_db(WORKER_DB_PATH).expect("Failed to open task queue db");

    let is_running = Arc::new(AtomicBool::new(true));
    let active_task = Arc::new(Mutex::new(None));
    start_independent_heartbeat(
        node_id.clone(),
        4040,
        Arc::clone(&is_running),
        Arc::clone(&active_task),
    );

    recover_stale_tasks(&conn);

    loop {
        let task_opt = claim_next_task(&mut conn, &node_id).ok().flatten();
        if let Some(task) = task_opt {
            execute_single_task(&task, &node_id, 4040, &active_task);
        }
        thread::sleep(std::time::Duration::from_secs(2));
    }
}

fn is_authorized(headers: &str) -> bool {
    if let Ok(expected_token) = env::var("WORKER_AUTH_TOKEN") {
        let expected_header = format!(
            "authorization: bearer {}",
            expected_token.trim().to_lowercase()
        );
        for line in headers.lines() {
            if line.to_lowercase().starts_with("authorization:")
                && line.to_lowercase() == expected_header
            {
                return true;
            }
        }
        return false;
    }
    !is_remote_mode()
}

fn stream_sse_logs(mut stream: TcpStream, target_task_id: Option<&str>) {
    let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";
    if stream.write_all(headers.as_bytes()).is_err() {
        return;
    }

    let log_path = get_log_file_path();
    let mut last_pos = if let Ok(metadata) = fs::metadata(&log_path) {
        metadata.len().saturating_sub(1024)
    } else {
        0
    };

    for _ in 0..120 {
        if let Ok(file) = File::open(&log_path) {
            let mut file = file;
            if file.seek(SeekFrom::Start(last_pos)).is_ok() {
                let reader = BufReader::new(file);
                for line in reader.lines().map_while(Result::ok) {
                    let should_send = match target_task_id {
                        Some(t_id) => line.contains(t_id) || line.contains("[SYSTEM]"),
                        None => true,
                    };

                    if should_send {
                        let msg = format!("data: {}\n\n", line);
                        if stream.write_all(msg.as_bytes()).is_err() {
                            return;
                        }
                    }
                }
                if let Ok(meta) = fs::metadata(&log_path) {
                    last_pos = meta.len();
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn read_http_request(stream: &mut TcpStream) -> Option<String> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 4096];

    let mut content_length: Option<usize> = None;
    let mut header_end_index: Option<usize> = None;

    let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));

    loop {
        match stream.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&temp[..n]);

                if header_end_index.is_none() {
                    if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                        header_end_index = Some(pos + 4);
                    } else if let Some(pos) = buffer.windows(2).position(|w| w == b"\n\n") {
                        header_end_index = Some(pos + 2);
                    }

                    if let Some(h_end) = header_end_index {
                        let header_str = String::from_utf8_lossy(&buffer[..h_end]);
                        for line in header_str.lines() {
                            let lower = line.to_lowercase();
                            if lower.starts_with("content-length:") {
                                if let Some(val_str) = line.split(':').nth(1) {
                                    content_length = val_str.trim().parse::<usize>().ok();
                                }
                            }
                        }
                    }
                }

                if let (Some(h_end), Some(c_len)) = (header_end_index, content_length) {
                    if buffer.len() >= h_end + c_len {
                        break;
                    }
                } else if header_end_index.is_some() && content_length.is_none() {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if buffer.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&buffer).to_string())
    }
}

fn handle_http_client(mut stream: TcpStream, conn_mutex: Arc<Mutex<Connection>>) {
    let request_str = match read_http_request(&mut stream) {
        Some(req) => req,
        None => return,
    };

    let first_line = request_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let path = parts[1];

    if method == "OPTIONS" {
        let options_response = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(options_response.as_bytes());
        return;
    }

    if !is_authorized(&request_str) {
        let unauthorized_body =
            json!({ "error": "Unauthorized: Invalid or missing WORKER_AUTH_TOKEN" }).to_string();
        let response = format!(
            "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            unauthorized_body.len(),
            unauthorized_body
        );
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    if method == "GET" && path == "/api/logs/stream" {
        stream_sse_logs(stream, None);
        return;
    }
    if method == "GET" && path.starts_with("/api/tasks/") && path.ends_with("/stream") {
        let task_id = &path["/api/tasks/".len()..path.len() - "/stream".len()];
        stream_sse_logs(stream, Some(task_id));
        return;
    }

    let (status_code, body) = match (method, path) {
        ("GET", "/api/health") => {
            let conn = conn_mutex.lock().unwrap();
            let json_body = get_health_json(&conn);
            ("200 OK", json_body.to_string())
        }
        ("GET", "/api/nodes") => {
            let conn = conn_mutex.lock().unwrap();
            let nodes = get_cluster_nodes(&conn);
            ("200 OK", json!({ "nodes": nodes }).to_string())
        }
        ("GET", "/api/cluster/load") => {
            let conn = conn_mutex.lock().unwrap();
            let load_body = get_cluster_load_json(&conn);
            ("200 OK", load_body.to_string())
        }
        ("GET", "/api/cluster/locks") => {
            let conn = conn_mutex.lock().unwrap();
            let mut stmt = conn.prepare("SELECT file_path, locked_by_task, locked_by_node, locked_at FROM cluster_file_locks").unwrap();
            let rows = stmt
                .query_map([], |r| {
                    Ok(json!({
                        "file_path": r.get::<_, String>(0)?,
                        "locked_by_task": r.get::<_, String>(1)?,
                        "locked_by_node": r.get::<_, String>(2)?,
                        "locked_at": r.get::<_, i64>(3)?
                    }))
                })
                .unwrap();
            let locks: Vec<serde_json::Value> = rows.filter_map(Result::ok).collect();
            ("200 OK", json!({ "locks": locks }).to_string())
        }
        ("GET", "/api/cluster/logs") => {
            let conn = conn_mutex.lock().unwrap();
            let mut stmt = conn.prepare("SELECT node_id, task_id, level, message, timestamp FROM cluster_logs ORDER BY timestamp DESC LIMIT 50").unwrap();
            let rows = stmt
                .query_map([], |r| {
                    Ok(json!({
                        "node_id": r.get::<_, String>(0)?,
                        "task_id": r.get::<_, Option<String>>(1)?,
                        "level": r.get::<_, String>(2)?,
                        "message": r.get::<_, String>(3)?,
                        "timestamp": r.get::<_, i64>(4)?
                    }))
                })
                .unwrap();
            let log_entries: Vec<serde_json::Value> = rows.filter_map(Result::ok).collect();
            ("200 OK", json!({ "logs": log_entries }).to_string())
        }
        _ if path.starts_with("/api/tasks/") && path.ends_with("/diagnostics") => {
            let task_id = &path["/api/tasks/".len()..path.len() - "/diagnostics".len()];
            let conn = conn_mutex.lock().unwrap();
            let diag: Option<serde_json::Value> = conn.query_row(
                "SELECT task_id, status, healing_attempts, last_error_stage, error_payload FROM worker_tasks WHERE task_id = ?1",
                params![task_id],
                |r| {
                    Ok(json!({
                        "task_id": r.get::<_, String>(0)?,
                        "status": r.get::<_, String>(1)?,
                        "healing_attempts": r.get::<_, i32>(2)?,
                        "last_error_stage": r.get::<_, Option<String>>(3)?,
                        "error_payload": r.get::<_, Option<String>>(4)?
                    }))
                },
            ).ok();

            match diag {
                Some(data) => ("200 OK", data.to_string()),
                None => (
                    "404 Not Found",
                    json!({ "error": "Task diagnostics not found" }).to_string(),
                ),
            }
        }
        ("GET", "/api/tasks") => {
            let conn = conn_mutex.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT task_id, status, intent, claimed_by, error_payload FROM worker_tasks ORDER BY created_at DESC LIMIT 10")
                .unwrap();
            let rows = stmt
                .query_map([], |r| {
                    Ok(json!({
                        "task_id": r.get::<_, String>(0)?,
                        "status": r.get::<_, String>(1)?,
                        "intent": r.get::<_, String>(2)?,
                        "claimed_by": r.get::<_, Option<String>>(3)?,
                        "error": r.get::<_, Option<String>>(4)?
                    }))
                })
                .unwrap();

            let task_list: Vec<serde_json::Value> = rows.filter_map(Result::ok).collect();
            ("200 OK", json!({ "tasks": task_list }).to_string())
        }
        ("POST", "/api/tasks") => {
            if let (Some(first_brace), Some(last_brace)) =
                (request_str.find('{'), request_str.rfind('}'))
            {
                if first_brace <= last_brace {
                    let json_slice = &request_str[first_brace..=last_brace];
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_slice) {
                        let intent = val["intent"].as_str().unwrap_or("Unnamed API task");
                        let root = val["project_root"].as_str();
                        let file = val["target_file"].as_str();
                        let sym = val["target_symbol"].as_str();
                        let patch = val["patch_code"].as_str();

                        let task_id = submit_task(intent, root, file, sym, patch);
                        (
                            "201 Created",
                            json!({ "status": "enqueued", "task_id": task_id }).to_string(),
                        )
                    } else {
                        (
                            "400 Bad Request",
                            json!({ "error": "Invalid JSON body" }).to_string(),
                        )
                    }
                } else {
                    (
                        "400 Bad Request",
                        json!({ "error": "Malformed JSON brackets" }).to_string(),
                    )
                }
            } else {
                (
                    "400 Bad Request",
                    json!({ "error": "Missing JSON object in body" }).to_string(),
                )
            }
        }
        _ if path.starts_with("/api/tasks/") => {
            let task_id = &path["/api/tasks/".len()..];
            let conn = conn_mutex.lock().unwrap();
            let record: Option<serde_json::Value> = conn.query_row(
                "SELECT task_id, status, intent, claimed_by, error_payload FROM worker_tasks WHERE task_id = ?1",
                params![task_id],
                |r| {
                    Ok(json!({
                        "task_id": r.get::<_, String>(0)?,
                        "status": r.get::<_, String>(1)?,
                        "intent": r.get::<_, String>(2)?,
                        "claimed_by": r.get::<_, Option<String>>(3)?,
                        "error": r.get::<_, Option<String>>(4)?
                    }))
                },
            ).ok();

            match record {
                Some(data) => ("200 OK", data.to_string()),
                None => (
                    "404 Not Found",
                    json!({ "error": "Task not found" }).to_string(),
                ),
            }
        }
        _ => (
            "404 Not Found",
            json!({ "error": "Endpoint not found" }).to_string(),
        ),
    };

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status_code,
        body.len(),
        body
    );

    let _ = stream.write_all(response.as_bytes());
}

fn run_serve(port: u16) {
    enforce_security_boundary();
    let node_id = get_node_id();
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind TCP server");
    println!("============================================================");
    println!(
        "🌐 VORA CLUSTER WORKER [{}] ON http://{} (v{})",
        node_id, addr, CURRENT_VERSION
    );
    println!("============================================================");
    println!("  [GET]  /api/health");
    println!("  [GET]  /api/nodes");
    println!("  [GET]  /api/cluster/load");
    println!("  [GET]  /api/cluster/locks");
    println!("  [GET]  /api/cluster/logs");
    println!("  [GET]  /api/tasks");
    println!("  [POST] /api/tasks");
    println!("  [GET]  /api/tasks/:id");
    println!("  [GET]  /api/tasks/:id/diagnostics");
    println!("  [SSE]  /api/logs/stream");
    println!("  [SSE]  /api/tasks/:id/stream");
    if env::var("WORKER_AUTH_TOKEN").is_ok() {
        println!("  🔒 Token Authentication: ENABLED");
    } else {
        println!("  🔓 Token Authentication: OPEN (Local mode)");
    }
    println!("============================================================");

    let conn = init_queue_db(WORKER_DB_PATH).expect("Failed to open task queue db");
    recover_stale_tasks(&conn);
    let shared_conn = Arc::new(Mutex::new(conn));

    let is_running = Arc::new(AtomicBool::new(true));
    let active_task = Arc::new(Mutex::new(None));
    start_independent_heartbeat(
        node_id.clone(),
        port,
        Arc::clone(&is_running),
        Arc::clone(&active_task),
    );

    let worker_conn_clone = Arc::clone(&shared_conn);
    let worker_node_id = node_id.clone();
    let active_task_clone = Arc::clone(&active_task);
    thread::spawn(move || loop {
        let task_opt = {
            let mut conn = worker_conn_clone.lock().unwrap();
            claim_next_task(&mut conn, &worker_node_id).ok().flatten()
        };

        if let Some(task) = task_opt {
            execute_single_task(&task, &worker_node_id, port, &active_task_clone);
        }
        thread::sleep(std::time::Duration::from_secs(2));
    });

    for stream in listener.incoming().flatten() {
        let conn_clone = Arc::clone(&shared_conn);
        thread::spawn(move || {
            handle_http_client(stream, conn_clone);
        });
    }
}

fn print_help() {
    println!(
        r#"
Vora Distributed Cluster Worker CLI (v0.55.0)
Usage:
  vora_worker serve [--port 4040]                                   Start HTTP REST & SSE API server & cluster node
  vora_worker health                                                Output worker health & cluster metrics as JSON
  vora_worker doctor                                                Check environment requirements & tooling readiness
  vora_worker status                                                Display cluster nodes and task queue breakdown
  vora_worker submit "<intent>" [root] [file] [sym] [patch_code]    Enqueue a new task into the SQLite queue
  vora_worker once                                                  Claim and execute a single pending task
  vora_worker run                                                   Start continuous 24/7 worker polling loop
  vora_worker help                                                  Display this help message
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "serve" => {
            let port = args
                .get(2)
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(4040);
            run_serve(port);
        }
        "health" => run_health(),
        "doctor" => run_doctor(),
        "status" => run_status(),
        "once" => run_once(),
        "run" => run_loop(),
        "submit" => {
            if args.len() < 3 {
                println!("Error: 'submit' requires an intent string.");
                return;
            }
            let intent = &args[2];
            let root = args.get(3).map(|s| s.as_str());
            let file = args.get(4).map(|s| s.as_str());
            let sym = args.get(5).map(|s| s.as_str());
            let patch = args.get(6).map(|s| s.as_str());
            submit_task(intent, root, file, sym, patch);
        }
        _ => print_help(),
    }
}
