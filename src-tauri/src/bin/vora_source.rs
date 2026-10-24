//! Vora Source -- CLI for provenance / data lineage.
//!
//! Usage (SRC-1 commands):
//!   vora_source add-source --kind video --title "..." [--path ...]
//!   vora_source list-sources [--kind video]
//!   vora_source show-source <SRC-0001>
//!   vora_source add-segment --source SRC-0001 --label "..." [--start 00:52:07] [--end 01:01:08]
//!   vora_source list-segments --source SRC-0001
//!   vora_source add-extract --segment SEG-0001 --kind transcribed --content "..."
//!   vora_source list-extracts --segment SEG-0001
//!   vora_source show-extract <EXT-0001>
//!   vora_source mark-reviewed <EXT-0001>
//!   vora_source link --extract EXT-0001 --target-kind mind_node --target-id card-042
//!   vora_source list-links --extract EXT-0001
//!   vora_source revisions --entity extract --id EXT-0001
//!
//! Usage (SRC-2 tree commands):
//!   vora_source show-segment <SEG-0001>
//!   vora_source list-children --parent SEG-0001
//!   vora_source list-descendants --root SEG-0001
//!   vora_source ancestors --id SEG-0003
//!   vora_source tree --source SRC-0001
//!   vora_source split-segment --id SEG-0001 --parts '[{"label":"A","start_sec":0,"end_sec":1800}]'
//!   vora_source merge-segments --ids SEG-0002,SEG-0004 --label "Bookends"
//!   vora_source move-segment --id SEG-0003 --parent SEG-0001 [--order 2]

use clap::{Parser, Subcommand};
use rusqlite::Connection;

use vora_lib::source::db;
use vora_lib::source::model::*;
use vora_lib::source::ops;
use vora_lib::source::revision;
use vora_lib::source::tree;

#[derive(Parser, Debug)]
#[command(name = "vora_source", version, about = "Vora Source -- provenance CLI")]
struct Cli {
    /// Path to SQLite file. If omitted, uses the same default as mind.
    #[arg(long, env = "VORA_DB")]
    db: Option<String>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    // ── SRC-1 commands ──────────────────────────────────────────
    AddSource {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        duration_sec: Option<i64>,
        #[arg(long)]
        page_count: Option<i64>,
        #[arg(long)]
        notes: Option<String>,
    },
    ListSources {
        #[arg(long)]
        kind: Option<String>,
    },
    ShowSource {
        id: String,
    },

    AddSegment {
        #[arg(long)]
        source: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
        #[arg(long)]
        page_start: Option<i64>,
        #[arg(long)]
        page_end: Option<i64>,
        #[arg(long)]
        line_start: Option<i64>,
        #[arg(long)]
        line_end: Option<i64>,
        #[arg(long)]
        notes: Option<String>,
        /// Optional parent segment id for creating a child segment directly.
        #[arg(long)]
        parent: Option<String>,
        /// Order index among siblings (default 0).
        #[arg(long, default_value = "0")]
        order: i64,
    },
    ListSegments {
        #[arg(long)]
        source: String,
    },

    AddExtract {
        #[arg(long)]
        segment: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        content: String,
        #[arg(long)]
        language: Option<String>,
        #[arg(long, default_value = "certain")]
        confidence: String,
        #[arg(long, default_value = "user")]
        created_by: String,
        /// Pass --no-reviewed to mark as pending review (default: reviewed).
        #[arg(long = "no-reviewed", action = clap::ArgAction::SetTrue)]
        no_reviewed: bool,
        #[arg(long)]
        parent_extract: Option<String>,
        #[arg(long)]
        notes: Option<String>,
    },
    ListExtracts {
        #[arg(long)]
        segment: String,
    },
    ShowExtract {
        id: String,
    },
    MarkReviewed {
        id: String,
    },

    Link {
        #[arg(long)]
        extract: String,
        #[arg(long)]
        target_kind: String,
        #[arg(long)]
        target_id: String,
        #[arg(long, default_value = "reference")]
        link_kind: String,
        #[arg(long)]
        note: Option<String>,
    },
    ListLinks {
        #[arg(long)]
        extract: String,
    },

    Revisions {
        #[arg(long)]
        entity: String,
        #[arg(long)]
        id: String,
    },

    // ── SRC-2 tree commands ──────────────────────────────────────
    ShowSegment {
        id: String,
    },
    ListChildren {
        #[arg(long)]
        parent: String,
    },
    ListDescendants {
        #[arg(long)]
        root: String,
    },
    Ancestors {
        #[arg(long)]
        id: String,
    },
    Tree {
        #[arg(long)]
        source: String,
    },
    SplitSegment {
        #[arg(long)]
        id: String,
        /// JSON array of parts, e.g.:
        /// '[{"label":"A","start_sec":0,"end_sec":1800},{"label":"B","start_sec":1800,"end_sec":3600}]'
        #[arg(long)]
        parts: String,
    },
    MergeSegments {
        /// Comma-separated segment ids (at least 2).
        #[arg(long)]
        ids: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        notes: Option<String>,
    },
    MoveSegment {
        #[arg(long)]
        id: String,
        /// New parent segment id. Omit to make a root segment.
        #[arg(long)]
        parent: Option<String>,
        /// New order index among siblings.
        #[arg(long)]
        order: Option<i64>,
    },
}

fn open_db(cli_db: Option<&str>) -> Result<Connection, Box<dyn std::error::Error>> {
    let path = match cli_db {
        Some(p) => p.to_string(),
        None => vora_lib::default_db_path()?,
    };
    let conn = Connection::open(&path).map_err(|e| format!("opening DB at {}: {}", path, e))?;
    db::init_schema(&conn).map_err(|e| format!("applying source schema: {}", e))?;
    Ok(conn)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let conn = open_db(cli.db.as_deref())?;

    match cli.cmd {
        // ── SRC-1 ──────────────────────────────────────────────
        Cmd::AddSource {
            kind,
            title,
            path,
            author,
            language,
            duration_sec,
            page_count,
            notes,
        } => {
            let k = SourceKind::from_str(&kind)
                .ok_or("invalid --kind (video|audio|doc|url|book|image)")?;
            let id = db::create_source(
                &conn,
                Source {
                    id: String::new(),
                    kind: k,
                    path,
                    title,
                    author,
                    language,
                    duration_sec,
                    page_count,
                    metadata_json: "{}".into(),
                    notes: notes.unwrap_or_default(),
                    status: String::new(),
                    created_at: 0,
                    updated_at: 0,
                },
            )?;
            println!("{}", id);
        }

        Cmd::ListSources { kind } => {
            let list = db::list_sources(&conn, kind.as_deref())?;
            for s in list {
                println!("{:<10} {:<6} {}", s.id, s.kind.as_str(), s.title);
            }
        }

        Cmd::ShowSource { id } => {
            let s = db::get_source(&conn, &id)?;
            print_source(&s);
        }

        Cmd::AddSegment {
            source,
            label,
            start,
            end,
            page_start,
            page_end,
            line_start,
            line_end,
            notes,
            parent,
            order,
        } => {
            let start_sec = match start {
                Some(t) => Some(parse_hhmmss(&t).ok_or_else(|| format!("invalid --start: {}", t))?),
                None => None,
            };
            let end_sec = match end {
                Some(t) => Some(parse_hhmmss(&t).ok_or_else(|| format!("invalid --end: {}", t))?),
                None => None,
            };
            let id = db::create_segment(
                &conn,
                Segment {
                    id: String::new(),
                    source_id: source,
                    label,
                    start_sec,
                    end_sec,
                    page_start,
                    page_end,
                    line_start,
                    line_end,
                    notes: notes.unwrap_or_default(),
                    status: String::new(),
                    created_at: 0,
                    updated_at: 0,
                    parent_segment_id: parent,
                    order_index: order,
                },
            )?;
            println!("{}", id);
        }

        Cmd::ListSegments { source } => {
            let list = db::list_segments(&conn, &source)?;
            for s in list {
                let range = match (s.start_sec, s.end_sec) {
                    (Some(a), Some(b)) => {
                        format!("{} -> {}", format_hhmmss(a), format_hhmmss(b))
                    }
                    _ => String::new(),
                };
                println!("{:<10} {:<20} {}", s.id, range, s.label);
            }
        }

        Cmd::AddExtract {
            segment,
            kind,
            content,
            language,
            confidence,
            created_by,
            no_reviewed,
            parent_extract,
            notes,
        } => {
            let tk = TransformKind::from_str(&kind).ok_or(
                "invalid --kind (transcribed|summarized|translated|quoted|paraphrased|extracted_idea|keyword|question|answer)",
            )?;
            let cf = Confidence::from_str(&confidence)
                .ok_or("invalid --confidence (certain|probable|uncertain)")?;
            let cb = CreatedBy::from_str(&created_by)
                .ok_or("invalid --created-by (user|whisper|llm|import)")?;
            let id = db::create_extract(
                &conn,
                Extract {
                    id: String::new(),
                    segment_id: segment,
                    transform_kind: tk,
                    content,
                    language,
                    confidence: cf,
                    created_by: cb,
                    reviewed_by_user: !no_reviewed,
                    parent_extract_id: parent_extract,
                    superseded_by: None,
                    notes: notes.unwrap_or_default(),
                    status: String::new(),
                    created_at: 0,
                    updated_at: 0,
                },
            )?;
            println!("{}", id);
        }

        Cmd::ListExtracts { segment } => {
            let list = db::list_extracts(&conn, &segment)?;
            for e in &list {
                let flag = if e.reviewed_by_user { " " } else { "*" };
                println!(
                    "{}{:<10} {:<15} {:<10} {}",
                    flag,
                    e.id,
                    e.transform_kind.as_str(),
                    e.created_by.as_str(),
                    truncate(&e.content, 60)
                );
            }
            if list.iter().any(|e| !e.reviewed_by_user) {
                println!("\n(* = pending review)");
            }
        }

        Cmd::ShowExtract { id } => {
            let e = db::get_extract(&conn, &id)?;
            print_extract(&e);
        }

        Cmd::MarkReviewed { id } => {
            db::mark_extract_reviewed(&conn, &id)?;
            println!("[OK] {} marked reviewed", id);
        }

        Cmd::Link {
            extract,
            target_kind,
            target_id,
            link_kind,
            note,
        } => {
            let tk = match target_kind.as_str() {
                "mind_node" => LinkTargetKind::MindNode,
                "chart_annotation" => LinkTargetKind::ChartAnnotation,
                "source" => LinkTargetKind::Source,
                "external" => LinkTargetKind::External,
                other => {
                    return Err(format!(
                        "invalid --target-kind '{}' (mind_node|chart_annotation|source|external)",
                        other
                    )
                    .into());
                }
            };
            let lk = match link_kind.as_str() {
                "evidence" => LinkKind::Evidence,
                "reference" => LinkKind::Reference,
                "inspiration" => LinkKind::Inspiration,
                "contradicts" => LinkKind::Contradicts,
                "supports" => LinkKind::Supports,
                other => {
                    return Err(format!(
                        "invalid --link-kind '{}' (evidence|reference|inspiration|contradicts|supports)",
                        other
                    )
                    .into());
                }
            };
            let id = db::create_link(
                &conn,
                Link {
                    id: String::new(),
                    extract_id: extract,
                    target_kind: tk,
                    target_id,
                    link_kind: lk,
                    note: note.unwrap_or_default(),
                    status: String::new(),
                    created_at: 0,
                    updated_at: 0,
                },
            )?;
            println!("{}", id);
        }

        Cmd::ListLinks { extract } => {
            let list = db::list_links_for_extract(&conn, &extract)?;
            for l in list {
                println!(
                    "{:<10} {:<18} {:<25} {}",
                    l.id,
                    db::link_target_str(&l.target_kind),
                    l.target_id,
                    l.note
                );
            }
        }

        Cmd::Revisions { entity, id } => {
            let k = match entity.as_str() {
                "source" => EntityKind::Source,
                "segment" => EntityKind::Segment,
                "extract" => EntityKind::Extract,
                "link" => EntityKind::Link,
                other => {
                    return Err(format!(
                        "invalid --entity '{}' (source|segment|extract|link)",
                        other
                    )
                    .into());
                }
            };
            let list = revision::list_for_entity(&conn, k, &id)?;
            for r in list {
                let ts = format_ts(r.created_at);
                println!(
                    "{:<10} {:<10} {} {} ({})",
                    r.id,
                    rev_type_str(&r.revision_type),
                    ts,
                    r.reason,
                    r.created_by
                );
            }
        }

        // ── SRC-2 tree commands ─────────────────────────────────
        Cmd::ShowSegment { id } => {
            let s = db::get_segment(&conn, &id)?;
            print_segment_detailed(&conn, &s)?;
        }

        Cmd::ListChildren { parent } => {
            let kids = db::list_children(&conn, &parent)?;
            for k in kids {
                println!("{:<10} {:<4} {}", k.id, k.order_index, k.label);
            }
        }

        Cmd::ListDescendants { root } => {
            let list = tree::descendants(&conn, &root)?;
            for s in list {
                println!("{:<10} {}", s.id, s.label);
            }
        }

        Cmd::Ancestors { id } => {
            let chain = tree::ancestors(&conn, &id)?;
            for s in chain {
                println!("{:<10} {}", s.id, s.label);
            }
        }

        Cmd::Tree { source } => {
            let roots = tree::build_tree(&conn, &source)?;
            if roots.is_empty() {
                println!("(no segments)");
            }
            for r in &roots {
                print_tree_node(r, 0);
            }
        }

        Cmd::SplitSegment { id, parts } => {
            #[derive(serde::Deserialize)]
            struct PartIn {
                label: String,
                start_sec: Option<i64>,
                end_sec: Option<i64>,
                page_start: Option<i64>,
                page_end: Option<i64>,
                line_start: Option<i64>,
                line_end: Option<i64>,
                #[serde(default)]
                notes: String,
            }
            let parsed: Vec<PartIn> = serde_json::from_str(&parts)
                .map_err(|e| format!("--parts must be a JSON array: {}", e))?;
            let parts: Vec<ops::SegmentPart> = parsed
                .into_iter()
                .map(|p| ops::SegmentPart {
                    label: p.label,
                    start_sec: p.start_sec,
                    end_sec: p.end_sec,
                    page_start: p.page_start,
                    page_end: p.page_end,
                    line_start: p.line_start,
                    line_end: p.line_end,
                    notes: p.notes,
                })
                .collect();
            let ids = ops::split_segment(&conn, &id, parts)?;
            for nid in ids {
                println!("{}", nid);
            }
        }

        Cmd::MergeSegments { ids, label, notes } => {
            let list: Vec<String> = ids
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let pid = ops::merge_segments(&conn, &list, &label, notes)?;
            println!("{}", pid);
        }

        Cmd::MoveSegment { id, parent, order } => {
            ops::move_segment(&conn, &id, parent.as_deref(), order)?;
            println!("[OK] moved {}", id);
        }
    }
    Ok(())
}

// ── pretty printers ──────────────────────────────────────────────

fn print_source(s: &Source) {
    println!("Source: {}", s.id);
    println!("  kind:     {}", s.kind.as_str());
    println!("  title:    {}", s.title);
    if let Some(p) = &s.path {
        println!("  path:     {}", p);
    }
    if let Some(a) = &s.author {
        println!("  author:   {}", a);
    }
    if let Some(l) = &s.language {
        println!("  language: {}", l);
    }
    if let Some(d) = s.duration_sec {
        println!("  duration: {}", format_hhmmss(d));
    }
    if let Some(p) = s.page_count {
        println!("  pages:    {}", p);
    }
    if !s.notes.is_empty() {
        println!("  notes:    {}", s.notes);
    }
    println!("  status:   {}", s.status);
}

fn print_extract(e: &Extract) {
    println!("Extract: {}", e.id);
    println!("  segment:    {}", e.segment_id);
    println!("  kind:       {}", e.transform_kind.as_str());
    println!("  confidence: {}", e.confidence.as_str());
    println!("  created_by: {}", e.created_by.as_str());
    println!("  reviewed:   {}", e.reviewed_by_user);
    if let Some(p) = &e.parent_extract_id {
        println!("  parent:     {}", p);
    }
    println!("  ---");
    println!("{}", e.content);
}

fn print_segment_detailed(
    conn: &Connection,
    s: &Segment,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Segment: {}", s.id);
    println!("  source:   {}", s.source_id);
    println!("  label:    {}", s.label);
    if let (Some(a), Some(b)) = (s.start_sec, s.end_sec) {
        println!("  time:     {} -> {}", format_hhmmss(a), format_hhmmss(b));
    }
    if let Some(p) = &s.parent_segment_id {
        println!("  parent:   {}", p);
    }
    println!("  order:    {}", s.order_index);
    let kids = db::list_children(conn, &s.id)?;
    println!(
        "  children: {} ({})",
        kids.len(),
        kids.iter()
            .map(|k| k.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}

fn print_tree_node(n: &tree::SegmentNode, indent: usize) {
    let pad = "  ".repeat(indent);
    let range = match (n.segment.start_sec, n.segment.end_sec) {
        (Some(a), Some(b)) => format!("{} -> {}", format_hhmmss(a), format_hhmmss(b)),
        _ => String::new(),
    };
    println!(
        "{}{:<10} {:<30} {}",
        pad, n.segment.id, n.segment.label, range
    );
    for c in &n.children {
        print_tree_node(c, indent + 1);
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}...", &s[..n.saturating_sub(3)])
    }
}

fn format_ts(ts: i64) -> String {
    let secs_per_min = 60i64;
    let secs_per_hour = 3600i64;
    let secs_per_day = 86400i64;
    let days = ts / secs_per_day;
    let rem = ts % secs_per_day;
    let h = rem / secs_per_hour;
    let m = (rem % secs_per_hour) / secs_per_min;
    let s = rem % secs_per_min;
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, h, m, s
    )
}

fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn rev_type_str(r: &RevisionType) -> &'static str {
    match r {
        RevisionType::Create => "create",
        RevisionType::Update => "update",
        RevisionType::Correct => "correct",
        RevisionType::Retract => "retract",
        RevisionType::Review => "review",
    }
}
