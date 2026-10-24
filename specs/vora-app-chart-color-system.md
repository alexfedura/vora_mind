# Vora App Chart Color System

## Goal

Make the complete Vora application UI use the same dark neutral color system as
Vora Chart. This applies to the Mind canvas shell, navigation, side panels,
Sources, Snapshots, settings, dialogs, and other application chrome.

## Visual contract

- App and chart canvas background: `#0f0f0f`.
- Vora Mind canvas uses a solid background without a dotted grid.
- Standard panels: `#0f0f0f`; raised controls and surfaces: `#181818`.
- Hover and selected neutral surfaces: `#242424` or the chart's existing
  component-specific neutral state.
- Standard and soft borders: `#2a2a2a` and `#242424`.
- Primary, muted, and faint text: `#d6dbe2`, `#7f8997`, and `#596371`.
- UI accent: chart amber `#e8ad55`; semantic positive and negative states use
  chart green `#30b58a` and chart red `#e36b74`.
- Existing chart-specific styles remain unchanged.

## Scope and preservation

- Unify application chrome, backgrounds, panel surfaces, borders, text, and
  interactive UI states across all routes.
- Preserve user-defined canvas node/card/shape colors, drawing colors, chart
  series colors, and other content colors. Only application UI chrome is
  restyled.
- Preserve layout, component behavior, accessibility states, and existing
  interaction semantics.
- Use shared global color tokens for new and updated UI styles; avoid
  introducing route-specific competing palettes.

## Verification

- Run `pnpm tsc --noEmit`.
- Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- Review changed surfaces to confirm that user-configurable content colors and
  chart rendering colors were not altered.
