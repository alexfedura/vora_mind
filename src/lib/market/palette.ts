/**
 * TradingView Dark (2024) — exact palette extracted from live DOM.
 */
export const TV_DARK = {
  // Chart canvas
  chartBg:        "#0f0f0f",
  gridMajor:      "#2a2a2a",
  gridMinor:      "#1e1e1e",

  // Candles (up/down)
  upColor:        "#089981",
  downColor:      "#f23645",
  wickUpColor:    "#089981",
  wickDownColor:  "#f23645",
  borderUpColor:  "#089981",
  borderDownColor:"#f23645",
  borderVisible:  false,

  // Axes
  axisText:       "#b2b5be",
  axisLine:       "#2a2a2a",
  axisBg:         "#0f0f0f",

  // Crosshair
  crosshairLine:  "#758696",
  crosshairLabelBg: "#363a45",
  crosshairLabelText: "#ffffff",

  // Watermark / misc
  watermarkText:  "rgba(54, 58, 69, 0.35)",

  // Text (overlays, tooltips)
  textPrimary:    "#d1d4dc",
  textSecondary:  "#787b86",

  // Panels / toolbars (chart chrome)
  paneBg:         "#0f0f0f",
  paneHover:      "#1a1a1a",
  paneBorder:     "#2a2a2a",
} as const;
