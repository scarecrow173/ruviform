/**
 * `@ruviform/ui-browser-test` (Issue #310, RF-GT-01).
 *
 * Framework-neutral real-browser GUI test harness over the published
 * Ruviform Web Components + Showcase fixtures — Playwright's stock
 * test API plus the shared drivers/assertions below. Nothing here is
 * a production runtime dependency: `@playwright/test` is dev/peer
 * only, and no UI framework is imported anywhere.
 *
 * Layers:
 * - {@link openHarness} / {@link HarnessSession} — page lifecycle,
 *   deterministic clock, hermetic serve, `__rvf` protocol access.
 * - {@link control} — real pointer/keyboard/focus drivers on the
 *   element's published `part` surfaces.
 * - `events`/`receipts` — `rvf-*` CustomEvent + Command/Receipt
 *   semantic assertions (exactly-once termination, composed/bubbles,
 *   dedup).
 * - `states`/`theme`/`a11y` — attribute/ARIA/theme-token oracles.
 * - `canvas` — Canvas2D fill probes (NaN-gap semantics), backend
 *   identity + draw-op stats for the WebGL2 capability path.
 * - {@link test} — the Playwright fixture (`rvf.open(...)`).
 *
 * Browser PASS is browser-lane evidence only — never OS WebView,
 * Native GPU, or product E2E evidence (those lanes are #313/#314 and
 * each consumer's own gate).
 */

export * from "./protocol.ts";
export * from "./serve.ts";
export * from "./poll.ts";
export * from "./harness.ts";
export * from "./control.ts";
export * from "./events.ts";
export * from "./receipts.ts";
export * from "./states.ts";
export * from "./theme.ts";
export * from "./canvas.ts";
export * from "./a11y.ts";
export * from "./fixtures.ts";
