# Vora Chart Spacebar Pan

## Behavior

- Holding Space and left-dragging over the chart pans the visible time and
  price ranges in the drag direction.
- Space-drag takes precedence over drawing selection and drawing manipulation;
  it does not select or modify drawings.
- Dragging over chart axes or floating controls does not start a canvas pan.
- When replay is open, a Space press without a pan drag continues to toggle
  replay playback.

## Verification

- Run `pnpm tsc --noEmit`, `pnpm build`, and
  `cargo check --manifest-path src-tauri/Cargo.toml`.
- Verify Space-drag pans left, right, up, and down without selecting drawings.
