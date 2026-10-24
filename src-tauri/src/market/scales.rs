//! ScaleRegistry: data-driven scale definitions per (symbol, step).
//! Replaces hardcoded S0..S9 in a future chunk. v5.1.8 stores the
//! data model only; detection still uses the legacy S-scale.

use serde::{Deserialize, Serialize};

/// Default tolerance preserved from pre-v6.4.0 data model.
pub fn default_tolerance() -> f64 {
    20.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scale {
    pub id: String,
    pub label: String,
    pub start_pct: f64,
    pub end_pct: f64,
    #[serde(default = "default_tolerance")]
    pub tolerance_pct: f64,
    pub color: String,
    pub enabled: bool,
    pub sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScaleRegistry {
    pub symbol: String,
    #[serde(alias = "mScale", alias = "m_scale", default)]
    pub step: Option<u8>,
    pub scales: Vec<Scale>,
}

impl ScaleRegistry {
    /// Default S0..S9 (matches box-standard.md section 5 lookup table).
    /// v6.4.9: global per-symbol registry (no step).
    pub fn default_for(symbol: &str) -> Self {
        let defs: [(f64, f64); 10] = [
            (0.100, 0.125),
            (0.125, 0.156),
            (0.156, 0.195),
            (0.195, 0.244),
            (0.244, 0.305),
            (0.305, 0.382),
            (0.382, 0.477),
            (0.477, 0.596),
            (0.596, 0.746),
            (0.746, 999.0),
        ];
        let palette = [
            "#4caf50", "#8bc34a", "#cddc39", "#ffeb3b", "#ffc107", "#ff9800", "#ff5722", "#f44336",
            "#e91e63", "#9c27b0",
        ];
        let mut scales = Vec::with_capacity(10);
        for (i, (s, e)) in defs.iter().enumerate() {
            scales.push(Scale {
                id: format!("s{}", i),
                label: format!("S{}", i),
                start_pct: *s,
                end_pct: *e,
                tolerance_pct: 20.0,
                color: palette[i].to_string(),
                enabled: true,
                sort_order: i as u32,
            });
        }
        ScaleRegistry {
            symbol: symbol.to_string(),
            step: None,
            scales,
        }
    }

    /// Validate:
    ///   - non-empty
    ///   - each: start >= 0, end > start
    ///   - non-overlapping (sorted by sort_order)
    /// Returns Vec<String> of English error messages (empty = OK).
    /// Warns (to stdout) if > 20 scales but does NOT error.
    pub fn validate(&self) -> Vec<String> {
        let mut errors: Vec<String> = Vec::new();

        if self.scales.is_empty() {
            errors.push("Registry has no scales".to_string());
            return errors;
        }

        let mut sorted: Vec<&Scale> = self.scales.iter().collect();
        sorted.sort_by_key(|s| s.sort_order);

        for s in &sorted {
            if s.start_pct < 0.0 {
                errors.push(format!(
                    "Scale '{}': start_pct ({}) must be >= 0",
                    s.label, s.start_pct
                ));
            }
            if s.end_pct <= s.start_pct {
                errors.push(format!(
                    "Scale '{}': end_pct ({}) must be > start_pct ({})",
                    s.label, s.end_pct, s.start_pct
                ));
            }
        }

        for i in 1..sorted.len() {
            let prev = sorted[i - 1];
            let curr = sorted[i];
            if curr.start_pct < prev.end_pct {
                let ov_start = curr.start_pct;
                let ov_end = if curr.end_pct < prev.end_pct {
                    curr.end_pct
                } else {
                    prev.end_pct
                };
                errors.push(format!(
                    "Scale '{}' [{}, {}) overlaps with '{}' [{}, {}) at range [{}, {})",
                    prev.label,
                    prev.start_pct,
                    prev.end_pct,
                    curr.label,
                    curr.start_pct,
                    curr.end_pct,
                    ov_start,
                    ov_end
                ));
            }
        }

        if self.scales.len() > 20 {
            println!(
                "[scales] Warning: {} scales exceeds recommended max (20)",
                self.scales.len()
            );
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_ten_scales() {
        let r = ScaleRegistry::default_for("XAUUSD");
        assert_eq!(r.scales.len(), 10);
        assert!(r.validate().is_empty());
    }

    #[test]
    fn validate_rejects_overlap() {
        let mut r = ScaleRegistry::default_for("XAUUSD");
        r.scales[1].start_pct = 0.100;
        assert!(!r.validate().is_empty());
    }

    #[test]
    fn validate_warns_not_errors_over_20() {
        let mut r = ScaleRegistry::default_for("XAUUSD");
        for i in 10..25u32 {
            r.scales.push(Scale {
                id: format!("s{}", i),
                label: format!("S{}", i),
                start_pct: 1000.0 + i as f64,
                end_pct: 1001.0 + i as f64,
                tolerance_pct: 20.0,
                color: "#ffffff".to_string(),
                enabled: true,
                sort_order: i,
            });
        }
        // >20 warns to stdout but must not produce validation errors
        assert!(r.validate().is_empty());
    }
}
