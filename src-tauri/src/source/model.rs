use serde::{Deserialize, Serialize};

// ─── Enums ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Video,
    Audio,
    Doc,
    Url,
    Book,
    Image,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Doc => "doc",
            Self::Url => "url",
            Self::Book => "book",
            Self::Image => "image",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "video" => Self::Video,
            "audio" => Self::Audio,
            "doc" => Self::Doc,
            "url" => Self::Url,
            "book" => Self::Book,
            "image" => Self::Image,
            _ => return None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransformKind {
    Transcribed,
    Summarized,
    Translated,
    Quoted,
    Paraphrased,
    ExtractedIdea,
    Keyword,
    Question,
    Answer,
}

impl TransformKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Transcribed => "transcribed",
            Self::Summarized => "summarized",
            Self::Translated => "translated",
            Self::Quoted => "quoted",
            Self::Paraphrased => "paraphrased",
            Self::ExtractedIdea => "extracted_idea",
            Self::Keyword => "keyword",
            Self::Question => "question",
            Self::Answer => "answer",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "transcribed" => Self::Transcribed,
            "summarized" => Self::Summarized,
            "translated" => Self::Translated,
            "quoted" => Self::Quoted,
            "paraphrased" => Self::Paraphrased,
            "extracted_idea" => Self::ExtractedIdea,
            "keyword" => Self::Keyword,
            "question" => Self::Question,
            "answer" => Self::Answer,
            _ => return None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Certain,
    Probable,
    Uncertain,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Certain => "certain",
            Self::Probable => "probable",
            Self::Uncertain => "uncertain",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "certain" => Self::Certain,
            "probable" => Self::Probable,
            "uncertain" => Self::Uncertain,
            _ => return None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CreatedBy {
    User,
    Whisper,
    Llm,
    Import,
}

impl CreatedBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Whisper => "whisper",
            Self::Llm => "llm",
            Self::Import => "import",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "user" => Self::User,
            "whisper" => Self::Whisper,
            "llm" => Self::Llm,
            "import" => Self::Import,
            _ => return None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LinkTargetKind {
    MindNode,
    ChartAnnotation,
    Source,
    External,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    Evidence,
    Reference,
    Inspiration,
    Contradicts,
    Supports,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RevisionType {
    Create,
    Update,
    Correct,
    Retract,
    Review,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Source,
    Segment,
    Extract,
    Link,
}

// ─── Structs ────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Source {
    pub id: String,
    pub kind: SourceKind,
    pub path: Option<String>,
    pub title: String,
    pub author: Option<String>,
    pub language: Option<String>,
    pub duration_sec: Option<i64>,
    pub page_count: Option<i64>,
    pub metadata_json: String,
    pub notes: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Segment {
    pub id: String,
    pub source_id: String,
    pub label: String,
    pub start_sec: Option<i64>,
    pub end_sec: Option<i64>,
    pub page_start: Option<i64>,
    pub page_end: Option<i64>,
    pub line_start: Option<i64>,
    pub line_end: Option<i64>,
    pub notes: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub parent_segment_id: Option<String>,
    pub order_index: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Extract {
    pub id: String,
    pub segment_id: String,
    pub transform_kind: TransformKind,
    pub content: String,
    pub language: Option<String>,
    pub confidence: Confidence,
    pub created_by: CreatedBy,
    pub reviewed_by_user: bool,
    pub parent_extract_id: Option<String>,
    pub superseded_by: Option<String>,
    pub notes: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Link {
    pub id: String,
    pub extract_id: String,
    pub target_kind: LinkTargetKind,
    pub target_id: String,
    pub link_kind: LinkKind,
    pub note: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Revision {
    pub id: String,
    pub entity_kind: EntityKind,
    pub entity_id: String,
    pub revision_type: RevisionType,
    pub before_json: Option<String>,
    pub after_json: String,
    pub reason: String,
    pub created_by: String,
    pub created_at: i64,
}

// ─── Time helpers ───────────────────────────────────────────────

pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// "00:52:07" -> 3127, or "52:07" -> 3127
pub fn parse_hhmmss(s: &str) -> Option<i64> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        3 => {
            let h: i64 = parts[0].parse().ok()?;
            let m: i64 = parts[1].parse().ok()?;
            let sec: i64 = parts[2].parse().ok()?;
            Some(h * 3600 + m * 60 + sec)
        }
        2 => {
            let m: i64 = parts[0].parse().ok()?;
            let sec: i64 = parts[1].parse().ok()?;
            Some(m * 60 + sec)
        }
        1 => s.parse().ok(),
        _ => None,
    }
}

/// 3127 -> "00:52:07"
pub fn format_hhmmss(sec: i64) -> String {
    let h = sec / 3600;
    let m = (sec % 3600) / 60;
    let s = sec % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_time_variants() {
        assert_eq!(parse_hhmmss("00:52:07"), Some(3127));
        assert_eq!(parse_hhmmss("52:07"), Some(3127));
        assert_eq!(parse_hhmmss("3127"), Some(3127));
        assert_eq!(parse_hhmmss("bad"), None);
    }

    #[test]
    fn format_time() {
        assert_eq!(format_hhmmss(3127), "00:52:07");
        assert_eq!(format_hhmmss(0), "00:00:00");
        assert_eq!(format_hhmmss(3661), "01:01:01");
    }

    #[test]
    fn source_kind_roundtrip() {
        for k in [SourceKind::Video, SourceKind::Audio, SourceKind::Book] {
            assert_eq!(SourceKind::from_str(k.as_str()), Some(k));
        }
    }

    #[test]
    fn transform_kind_roundtrip() {
        for k in [TransformKind::Transcribed, TransformKind::Summarized] {
            assert_eq!(TransformKind::from_str(k.as_str()), Some(k));
        }
    }
}
