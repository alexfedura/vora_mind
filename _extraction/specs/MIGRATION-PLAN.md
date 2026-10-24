# Scale Lab to Chart Migration Plan

Status: planning only. This document does not authorize implementation; C1 requires separate user approval.

## 1. Confirmed Current State

- Current branch is `main`, clean at `72ad0a8` (`v0.83.23`).
- The router is `src/main.tsx`, not `src/App.tsx`.
- `/market` renders `MarketPage`; `/scale-lab` renders `ScaleLabPage`; `/chart` does not exist.
- `src/pages/MarketPage.tsx` is tracked on `main`. `src/pages/MarketWorkspace.tsx` and `src/pages/MarketPage.css` exist only on `wip/market-workspace-20261011-185431`; do not merge those files wholesale.
- The WIP workspace's working selector/watchlist can be adapted, but its drawing controls are visual-only. Its timeframe and warmup flows use standard intervals, whereas Scale Lab uses M0-M10.

## 2. Target Architecture

- Canonical route: `/chart`, rendered by `ChartPage` in `src/pages/ChartPage.tsx` (renamed from `ScaleLabPage.tsx` after the pre-migration backup tag exists).
- Preserve the existing Scale Lab chart engine, M0-M10 data contract, snapshots, overlays, drawings, zoom/pan, magnet behavior, and policy/registry logic.
- Keep `/scale-lab` as a compatibility redirect to `/chart`. Also redirect `/market` to `/chart` to preserve existing bookmarks without retaining the MarketPage implementation.
- Keep `/`, `/sources`, and `/snapshots` as separate existing destinations. `/` remains the canvas page when the configured default is Home.
- Add the WIP local-source selector and watchlist as focused components, not as a replacement chart shell. Keep their styles scoped to the new chart workspace additions; do not replace the Scale Lab chart theme.
- Default-route choices become Home and Chart. `src/lib/settings.ts` must interpret a stored legacy value `scale-lab` as Chart so existing user settings continue to work.
- A selected source changes the chart symbol and all symbol-dependent requests/state. If source listing is empty or fails, preserve the existing XAUUSD fallback and surface errors. Do not substitute candles.
- Do not wire WIP's standard-timeframe warmup into the M0-M10 chart. The WIP warmup endpoint builds standard intervals; its hard-coded `D:\workspace\...` file path is not portable. Any future warmup UI must be separately designed around the M-scale data contract and a supported source path.

## 3. File Operations

| Action | File | Notes |
|---|---|---|
| Rename | `src/pages/ScaleLabPage.tsx` to `src/pages/ChartPage.tsx` | C1, after backup tag; preserve component behavior and update imports. |
| Modify | `src/main.tsx` | Register `/chart`; make `/scale-lab` and `/market` redirects; point default-route behavior at `/chart`. This is the router; `App.tsx` is not. |
| Modify | `src/shared/layout/LeftSidebar.tsx` | Replace Market and Scale Lab entries with one Chart link to `/chart`; preserve Sources. |
| Modify | `src/lib/settings.ts` | Use `home | chart`; map stored `scale-lab` to `chart`; keep the current key `vora.default_route` for compatibility. |
| Modify | `src/components/market/PolicyPanel.tsx` | Change default-route control/type from Scale Lab to Chart. |
| Modify | `src/pages/SnapshotsPage.tsx` | Navigate/open snapshots at `/chart?snapshot=...`; update its return link and copy to Chart. |
| Modify | `src/pages/ScaleLabPage.tsx` before rename / `src/pages/ChartPage.tsx` after rename | Replace single-symbol constant and hard-coded API symbols; retain all Scale Lab behavior. |
| Create | `src/components/market/SymbolSelector.tsx` | Extract/adapt WIP local-source search and selection; consume typed market source records. |
| Create | `src/components/market/WatchlistPanel.tsx` | Extract/adapt watchlist/details UI; do not present standard timeframe cache counts as M0-M10 availability. |
| Create or adapt | `src/pages/ChartPage.css` | Only scoped styles needed for selector/watchlist. Keep chart palette/layout intact. |
| Delete | `src/pages/MarketPage.tsx` | C5, after Chart is proven and old path redirects. |
| Do not add/delete on `main` | `src/pages/MarketWorkspace.tsx`, `src/pages/MarketPage.css` | These are absent from `main`; retain the WIP branch as reference unless separately authorized. |
| No change | `src/App.tsx` | It implements the canvas UI; route selection is in `src/main.tsx`. |

## 4. WIP Integration Evidence

Sidebar links on `main`:

```tsx
<Link to="/market"> ... <span>Market</span> ... </Link>
<Link to="/scale-lab"> ... <span>Scale Lab</span> ... </Link>
```

Source: `src/shared/layout/LeftSidebar.tsx:107-118`.

Current default-route behavior:

```ts
export type DefaultRoute = "home" | "scale-lab";
return v === "home" ? "home" : "scale-lab";
```

Source: `src/lib/settings.ts:1-11`; persisted key is `vora.default_route` (`:4`). The router redirects `/` when `defaultRoute === "scale-lab"` in `src/main.tsx:18-22`. `src/App.tsx` contains no route redirect.

The WIP selector is driven by `MarketWorkspace` props `sources`, `activeSource`, `symbol`, `onSelectSource`, and `timeframe` (`MarketWorkspace.tsx:65-87`, `:131-148`). It filters the sources, then invokes `chooseSource(source)` from each option (`:164-170`, `:274-277`); `chooseSource` calls `onSelectSource(source)` (`:200-204`). WIP `MarketPage` initializes `sources`, `activeSource`, `symbol`, `timeframe`, candles, loading/error/warmup state (`MarketPage.tsx:12-21`), loads sources (`:27-32`), and changes active source and symbol (`:68-71`).

The WIP watchlist is an `aside.market-watchlist` with Watchlist/Details tabs, a filter, source rows, and source metadata (`MarketWorkspace.tsx:438-483`). Its add-symbol button is disabled (`:446`); it is a local-source list, not an editable external-symbol watchlist.

WIP token names are scoped below `.market-workspace` and therefore do not collide by CSS variable name with `src/App.css`'s `--color-*` variables:

```css
--market-bg: #0c0e12;
--market-panel: #12151a;
--market-panel-raised: #171b21;
--market-border: #242a32;
--market-border-soft: #1c222a;
--market-text: #d6dbe2;
--market-muted: #7f8997;
--market-faint: #596371;
--market-accent: #e8ad55;
--market-green: #30b58a;
--market-red: #e36b74;
```

Source: WIP `src/pages/MarketPage.css:1-12`; global tokens are in `src/App.css:4-10`. These tokens have no name collision. Preserve the existing chart's `TV_DARK` styling; apply WIP tokens only to the new controls/panel if adopted.

## 5. Decisions

### Q1 — Target route: A, `/chart`

This creates the requested explicit Chart URL without changing the canvas route. `/scale-lab` remains a redirect for bookmarks. The default-route control can choose Home or Chart while old persisted `scale-lab` values map to Chart. The root redirect belongs in `main.tsx`, not `App.tsx`.

### Q2 — MarketPage/MarketWorkspace fate: A, retire the Market page

Remove the tracked `MarketPage.tsx` implementation and route only after `/chart` works. Keep redirects from `/market` and `/scale-lab`. The `MarketWorkspace.tsx` and WIP CSS files are not on `main`; leave their WIP branch untouched as reference instead of importing them only to delete them. Adapt the useful selector/watchlist behavior into focused components.

### Q3 — Symbol/watchlist: A, integrate both with explicit M-scale boundaries

Adopt the local-source selector and watchlist because they are the WIP's useful additions. Replace the fixed `SYMBOL` with selected-symbol state and update drawings storage keys, snapshot metadata, candle fetch, pivots, boxes, registry, and every other symbol-scoped operation. Keep M0-M10 selection and fetch semantics. Do not switch the chart to 1m-1d or call standard warmup as if it populated M candles. Default to XAUUSD when source listing yields no usable entry, and show an empty/error state rather than fabricated data.

## 6. Sub-chunk Breakdown

Each chunk is independent, keeps compatibility routes until its acceptance checks pass, and stops for user review before the next chunk. No implementation is authorized by this plan.

| Chunk | Scope | Acceptance focus |
|---|---|---|
| C1 | Create `/chart` from Scale Lab; rename the component file; make `/scale-lab` redirect while preserving query string. | Existing chart mounts at `/chart`; `/scale-lab?snapshot=ID` still restores the same snapshot; no feature removed. |
| C2 | Update sidebar and default-route setting/control; update snapshot navigation; add `/market` redirect. | Home/Chart preference works; legacy `scale-lab` preference and links resolve to Chart; snapshot query survives. |
| C3 | Add SymbolSelector and selected-symbol state; migrate symbol-scoped API calls and local drawing/registry lifecycle. | XAUUSD fallback; source selection fetches that symbol's M data, pivots, boxes, registry, and drawings; missing data remains empty/error. |
| C4 | Add WatchlistPanel/source details with scoped styles. | Source rows, filter, metadata and selection work; standard interval counts are labeled distinctly from M-scale availability. |
| C5 | Remove MarketPage component/import; replace `/market` page with redirect; remove obsolete Market sidebar entry. | No `/market` page implementation remains; old URL redirects; `/chart` and `/sources` work. WIP branch remains untouched. |
| C6 | Audit references, localStorage compatibility, and feature parity; no branch deletion by default. | Search for stale Scale Lab route/default labels; verify every feature listed in the recon inventory. |
| C7 | Run verification and create release/backup tags only after explicit approval. | `pnpm tsc --noEmit`, `pnpm build`, manual chart/snapshot/drawing checks, and review sign-off. |

### Symbol-Scoped Work Required in C3

Scale Lab has `SYMBOL = "XAUUSD"` (`ScaleLabPage.tsx:262`) and many dependent references. At minimum audit/migrate candle fetch (`:842-843`), snapshot capture and restore (`:593`, `:991`), pivots (`:1079`), box detection (`:1183`), scale-registry load (`:1385`), drawing keys/hydration (`:260-327`), removal/history behavior (`:397-489`), and snapshot name (`:2886`). This is more than replacing the constant: ensure effects run on symbol change and cancel/ignore stale async results.

## 7. Rollback Plan

- The pre-migration tag created in this planning phase identifies the untouched starting commit.
- Keep each future chunk isolated in a reviewed commit. Roll back a completed chunk with `git revert <chunk-commit>` and push the revert normally; do not reset or force-push shared history.
- If a chunk is not committed, first preserve its diff with `git diff > migration-chunk.patch`, then restore only that chunk's files after review. Do not use broad checkout/reset commands that could discard unrelated work.
- Keep `/scale-lab` and `/market` redirects until parity and snapshot deep-link checks pass; do not delete the Scale Lab implementation before that gate.

## 8. Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `/scale-lab` bookmarks or snapshot deep links break | Low | High | Redirect while preserving query string; test both plain and `?snapshot=` URLs. |
| WIP standard timeframes are confused with M0-M10 | High | High | Keep Scale Lab M0-M10 fetches; label any standard cache metadata distinctly; do not reuse WIP warmup as M-scale warmup. |
| Symbol changes leave stale candles, overlays, or drawings | Medium | High | Audit every symbol-scoped API/effect/storage key; cancel or guard stale requests; test two symbols and fallback. |
| WIP CSS changes Scale Lab appearance | Medium | Medium | Scope new tokens/styles to selector/watchlist; preserve existing chart palette and layout. |
| Default route or PolicyPanel fails for existing `scale-lab` preference | Medium | Medium | Read legacy value as Chart, keep storage key, test Home/Chart choices and old stored value. |
| Market page removed before Chart parity | Low | High | Keep old routes until C1-C4 gates pass; C5 only after manual acceptance. |

## 9. Planning-Phase Verification

- `pnpm tsc --noEmit`: required before implementation; no source changes in this phase.
- `pnpm build`: required before implementation; no source changes in this phase.
- Backup tag: recorded in the phase report at the current HEAD. It must exist and be pushed before C1 changes any code or renames the Scale Lab file.

## 10. Approval Boundary

This plan covers C1 only as the next proposed implementation chunk. Stop after this planning phase and wait for explicit approval before C1. C2-C7 each require their own review/approval; this plan grants no implementation authorization.