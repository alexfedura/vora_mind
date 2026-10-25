# VORA Font Customization

## Intent

Allow users to tune fonts and text sizes independently for the application UI, canvas cards, freeform canvas text, and journal writing area. Users can import local TTF, OTF, WOFF, or WOFF2 fonts and assign an imported font to any of those areas.

## Persistence and behavior

- Store font names, sizes, and imported font data in the existing database-backed application settings.
- Apply preferences consistently across the app and after restart without adding a dependency or changing durable domain data.
- Keep canvas text-node size controls proportional to the configured freeform text size.
- Do not show routine journal saving/saved status; keep save errors and retry controls visible.

## Acceptance criteria

- Settings contains independent font-family and font-size controls for app UI, cards, canvas text, and journal.
- Imported local fonts can be selected per area and removed; invalid fonts produce a visible error.
- Font and size changes apply immediately and persist across restarts.
- Journal no longer displays routine save status, while save errors remain actionable.
- Frontend TypeScript and Rust checks pass.
