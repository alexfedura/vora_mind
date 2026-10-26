use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeframe {
    M1,
    M5,
    M15,
    M30,
    H1,
    H4,
    D1,
}

impl Timeframe {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H4 => "4h",
            Self::D1 => "1d",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "1m" | "M1" => Some(Self::M1),
            "5m" | "M5" => Some(Self::M5),
            "15m" | "M15" => Some(Self::M15),
            "30m" | "M30" => Some(Self::M30),
            "1h" | "H1" => Some(Self::H1),
            "4h" | "H4" => Some(Self::H4),
            "1d" | "D1" => Some(Self::D1),
            _ => None,
        }
    }

    pub fn seconds(&self) -> i64 {
        match self {
            Self::M1 => 60,
            Self::M5 => 300,
            Self::M15 => 900,
            Self::M30 => 1800,
            Self::H1 => 3600,
            Self::H4 => 14400,
            Self::D1 => 86400,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtremeKind {
    HighFirst,
    LowFirst,
    Same,
}

impl Default for ExtremeKind {
    fn default() -> Self {
        ExtremeKind::Same
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Candle {
    pub symbol: String,
    pub timeframe: String,
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub tick_count: i64,
    #[serde(default)]
    pub first_extreme: ExtremeKind,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DrawingLineStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DrawingTextAlign {
    Left,
    Center,
    Right,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DrawingTextVerticalAlign {
    Top,
    Middle,
    Bottom,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DrawingKind {
    Rectangle,
    GradedRectangle,
    Vline,
    Hline,
    Hray,
    Trendline,
    Icon,
}

impl DrawingKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::GradedRectangle => "graded-rectangle",
            Self::Vline => "vline",
            Self::Hline => "hline",
            Self::Hray => "hray",
            Self::Trendline => "trendline",
            Self::Icon => "icon",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DrawingStyleTemplateSettings {
    pub color: String,
    pub color_opacity: f64,
    pub line_width: f64,
    pub style: DrawingLineStyle,
    pub fill_color: Option<String>,
    pub fill_opacity: f64,
    pub text: String,
    pub text_color: String,
    pub text_opacity: f64,
    pub text_size: f64,
    pub text_bold: bool,
    pub text_italic: bool,
    pub text_align_horizontal: DrawingTextAlign,
    pub text_align_vertical: DrawingTextVerticalAlign,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DrawingStyleTemplate {
    pub id: String,
    pub name: String,
    pub drawing_type: DrawingKind,
    pub settings: DrawingStyleTemplateSettings,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DrawingAlertCondition {
    EnteringDrawing,
    Crossing,
}

impl DrawingAlertCondition {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EnteringDrawing => "entering_drawing",
            Self::Crossing => "crossing",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChartDrawingAlert {
    pub id: String,
    pub symbol: String,
    pub timeframe: i32,
    pub drawing_id: String,
    pub condition: DrawingAlertCondition,
    pub message: String,
    pub trigger_once: bool,
    pub notify_app: bool,
    pub notify_toast: bool,
    pub notify_email: bool,
    pub notify_sound: bool,
    pub expiration_at: Option<i64>,
    pub enabled: bool,
    pub created_at: i64,
    pub last_evaluated_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TickStats {
    pub row_count: i64,
    pub first_ts: i64,
    pub last_ts: i64,
    pub min_price: f64,
    pub max_price: f64,
}

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeframe_roundtrip() {
        let tfs = [
            Timeframe::M1,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H4,
            Timeframe::D1,
        ];
        for tf in tfs {
            assert_eq!(Timeframe::from_str(tf.as_str()), Some(tf.clone()));
        }
    }

    #[test]
    fn timeframe_seconds() {
        assert_eq!(Timeframe::M1.seconds(), 60);
        assert_eq!(Timeframe::M5.seconds(), 300);
        assert_eq!(Timeframe::M15.seconds(), 900);
        assert_eq!(Timeframe::M30.seconds(), 1800);
        assert_eq!(Timeframe::H1.seconds(), 3600);
        assert_eq!(Timeframe::H4.seconds(), 14400);
        assert_eq!(Timeframe::D1.seconds(), 86400);
    }
}

// ─── Box DTOs (for IPC) ───

use crate::market::boxes::{Box as DetectedBox, BoxPattern, BoxZone};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxParamsDto {
    #[serde(alias = "userM", alias = "user_m")]
    pub user_step: Option<u8>,
    pub s_small: Option<u8>,
    pub s_medium: Option<u8>,
    pub s_large: Option<u8>,
    pub z0_scale: Option<u8>,
    #[serde(default)]
    pub extend_to_engulf: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BoxPatternDto {
    RBR,
    DBD,
    RBD,
    DBR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoxZoneDto {
    Supply,
    Demand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxDto {
    pub address: String,
    pub symbol: String,
    #[serde(alias = "mScale", alias = "m_scale")]
    pub step: u8,
    #[serde(alias = "pivotM", alias = "pivot_m")]
    pub pivot_step: u8,
    pub pattern: BoxPatternDto,
    pub zone: BoxZoneDto,
    pub scale: u8,
    pub role: String,
    pub x_idx: i64,
    pub ts_start: i64,
    pub ts_end: i64,
    pub engulf_ts: Option<i64>,
    pub duration_sec: i64,
    pub scale_sec: i64,
    pub high: f64,
    pub low: f64,
    pub origin_price: f64,
    pub leg1_magnitude: f64,
    pub leg2_magnitude: f64,
    pub parent_address: Option<String>,
    pub children_addresses: Vec<String>,
    pub x: i64,
    pub y_abs: i64,
    pub z: u8,
    pub y_band: i64,
    pub chain_id: Option<i64>,
    pub chain_pos: Option<u32>,
    pub y_lane: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainDto {
    pub id: i64,
    pub y_band: i64,
    pub z: u8,
    pub boxes: Vec<String>,
    pub kind_seq: Vec<String>,
    pub size_min: f64,
    pub size_max: f64,
}

impl From<&crate::market::chain::Chain> for ChainDto {
    fn from(c: &crate::market::chain::Chain) -> Self {
        Self {
            id: c.id,
            y_band: c.y_band,
            z: c.z,
            boxes: c.boxes.clone(),
            kind_seq: c.kind_seq.clone(),
            size_min: c.size_min,
            size_max: c.size_max,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotDto {
    pub idx: usize,
    pub ts: i64,
    pub price: f64,
    pub kind: String, // "H" | "L"
}

impl From<DetectedBox> for BoxDto {
    fn from(b: DetectedBox) -> Self {
        use crate::market::boxes::{role_for_scale, BoxRole};
        let role = match role_for_scale(b.step, b.scale) {
            Some(BoxRole::Small) => "small",
            Some(BoxRole::Medium) => "medium",
            Some(BoxRole::Large) => "large",
            None => "small",
        }
        .to_string();

        Self {
            address: b.address,
            symbol: b.symbol,
            step: b.step,
            pivot_step: b.pivot_step,
            pattern: match b.pattern {
                BoxPattern::RBR => BoxPatternDto::RBR,
                BoxPattern::DBD => BoxPatternDto::DBD,
                BoxPattern::RBD => BoxPatternDto::RBD,
                BoxPattern::DBR => BoxPatternDto::DBR,
            },
            zone: match b.zone {
                BoxZone::Supply => BoxZoneDto::Supply,
                BoxZone::Demand => BoxZoneDto::Demand,
            },
            scale: b.scale,
            role,
            x_idx: b.x_idx,
            ts_start: b.ts_start,
            ts_end: b.ts_end,
            engulf_ts: b.engulf_ts,
            duration_sec: b.duration_sec,
            scale_sec: b.scale_sec,
            high: b.high,
            low: b.low,
            origin_price: b.origin_price,
            leg1_magnitude: b.leg1_magnitude,
            leg2_magnitude: b.leg2_magnitude,
            parent_address: b.parent_address,
            children_addresses: b.children_addresses,
            x: b.x,
            y_abs: b.y_abs,
            z: b.z,
            y_band: b.y_band,
            chain_id: b.chain_id,
            chain_pos: b.chain_pos,
            y_lane: b.y_lane,
        }
    }
}
