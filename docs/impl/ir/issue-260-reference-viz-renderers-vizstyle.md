---
id: "IMPL-GhxLqs97yNh1VgtsWdSDL3"
type: "impl"
status: "completed"
title: "Issue #260 Reference Viz Renderer・VizStyle (Waveform / Meter / Spectrum) 実装記録"
created: "2026-11-03"
updated: "2026-11-03"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/260"
  changes:
    added:
      - type: "directory"
        path: "ui/viz/"
    modified:
      - type: "file"
        path: "ui/components/src/tokens.ts"
      - type: "file"
        path: ".github/workflows/ci.yml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implementation Record — Issue #260: Reference Viz Renderer・VizStyle (Waveform / Meter / Spectrum)

Status: delivered (headless-verified)
Depends on: Issue #258 (`ui/core`, merge `9ff4de9`), Issue #259 (`ui/components`, commit `84e6972` / main `e3d1471`)
Design source: `docs/designs/issue-257-…` §12 (VizStyle/token paint contract) and §13 (framework-neutral Web UI/Native Hybrid)

## What was built

New package `ui/viz` (`@ruviform/ui-viz`, private) — plain TypeScript +
Web platform APIs. No UI framework, runtime, or renderer-library
dependency; `happy-dom@21.0.4` is a test-only devDependency providing
the DOM for `node --test`. No data-processing dependency: LOD
decimation, FFT, and measurement stay in `ruviform-viz` /
`ruviform-audio-analysis` (Issue #157) behind the `*DataSource` ports.

Dispatched scope: Waveform, Level Meter, Spectrum renderers + `VizStyle`
+ Canvas2D/WebGL2 backends + Custom Element surfaces. Spectrogram and
generic TimeSeries/FeaturePlot remain deferred to the broader #260
acceptance set / Issue #262 native-parity lane.

### Module map

| Path | Role |
| --- | --- |
| `src/source.ts` | `WaveformDataSource` (bounded query, revision, AbortSignal), `SnapshotSource<F>` / `MeterDataSource` / `SpectrumDataSource` ports; reference impls `MemoryWaveformSource` (interleaved f64, NaN-preserving, raw-or-bins bounded queries) + `ManualSnapshotSource<F>` |
| `src/cache.ts` | `LruViewCache<WaveformView>` — byte-budgeted LRU; keys = source id + revision + channel + span + buckets (style is never a key) |
| `src/axis.ts` | `niceStep` (1/2/5), linear/log tick generation, axis-unit formatting (seconds/samples/Hz/db), `-0` normalization |
| `src/viewport.ts` | span mapping/clamp/pan/zoom helpers |
| `src/color.ts` | CSS color → RGBA parse for the GL path (diagnostics on failure) |
| `src/ops.ts` | backend-neutral `DrawOp` IR: rect/rectStroke/band/line/poly/hatch/text/clipPush/clipPop, `"plot"`/`"overlay"` layers, CSS-px coordinates |
| `src/layout.ts` | `computeLayout` — plot rect + axis gutters |
| `src/scene.ts` | shared frame/gap/hatch/stale/loss/overlay helpers |
| `src/style.ts` | `VizStyleInput` → `resolveVizStyle` → immutable `ResolvedVizStyle`; `collectVizTokens` resolves the composed `<rvf-theme>`/instance chain incl. `{ref}` indirection |
| `src/waveform.ts` | `waveformScene` — per-channel lanes, min/max envelope + RMS band (bins) or polyline (samples), NaN gaps, partial-coverage ticks, zero line, axes, selection/playhead/marker overlays |
| `src/meter.ts` | `meterScene` — dB segments (low/mid/high), RMS line, held peak, clip cue, `NaN` channel hatch, stale/loss cues, vertical/horizontal |
| `src/spectrum.ts` | `spectrumScene` — linear/log freq mapping, dB window, fill+curve runs split on NaN, gap ticks, stale/loss cues |
| `src/canvas2d.ts` | `Canvas2DExecutor` — all ops |
| `src/webgl2.ts` | `WebGl2Executor` — plot layer in GL (triangles/lines); hatch + per-x bands + all overlay ops rasterize on the 2D overlay per the declared capability record |
| `src/backend.ts` | `createVizBackend` — preference `auto|canvas2d|webgl2`, capability record, explicit fallback |
| `src/elements/viz-element.ts` | `RvfVizElement`: stacked plot/overlay canvases, dpr backing, ResizeObserver sizing, MutationObserver theme-watch (repaint-only), `rvf-backend` capability event, status part + ARIA as the non-color state channel |
| `src/elements/snapshot-element.ts` | `SnapshotVizElement` — snapshot pull + autopump, `seq`-gap→`dropped` bookkeeping, `stale-after` timeout |
| `src/elements/{waveform,meter,spectrum}.ts` | the three elements + `registerRvfVizElements()` (explicit registration, no import side effects) |
| `test/*.test.ts` + `test/env.ts` | 89 headless tests: happy-dom + fake 2D/GL contexts incl. context-pinning canvas |
| `examples/showcase.html` | synthetic-source demo page (no analysis state in-page) |

### Decisions worth recording

- **Paint/data separation is structural**: `ResolvedVizStyle` contains
  only paint values; `appearance`/`--rvf-viz-*` changes set
  `styleDirty` + repaint. Cache keys, query state, and subscriptions
  never observe style.
- **LOD budget = plot px**: `buckets` in a `WaveformQuery` is the laid-out
  plot width, not the element width — so the deferred first query waits
  for a real layout (`lastLayout_`), otherwise the first view would be
  cached under a stale key (found in review-cycle testing).
- **Deferred-first-query coalescing**: `invalidateQueued_` + the
  in-flight check (`abort_` live and not aborted) bound the "no view
  yet" requery path to one outstanding request — without it, every rAF
  draw while a slow query was pending reissued the query (found as an
  unbounded requery loop in testing).
- **Stale responses**: a view is accepted only when its `revision`
  matches both the meta captured at issue time and `meta()` at resolve
  time; one bounded retry, then `stale` state (hatch + status text, not
  color alone).
- **Missing data**: `NaN`/non-finite → gaps (waveform lane hatch spans,
  spectrum run breaks + gap ticks, meter channel hatch). Nothing is
  zero-filled or interpolated across.
- **Backend split**: WebGL2 draws the plot layer only; text, state cues,
  hatches, and log-frequency fills stay on the 2D overlay. Ops are
  CSS-px — one scene feeds both executors, so fallback preserves
  geometry exactly. `webglcontextlost` swaps sticky to Canvas2D with a
  recorded `rvf-backend` event.
- **Meter/spectrum are pull-based**: the source owns the measurement
  cycle; the element pulls `snapshot()` per draw. `sequence` gaps →
  `dropped` (visualized), age → `stale` — loss is surfaced, never
  hidden.
- **Waveform pointer interaction**: drag = pan, wheel = zoom
  (pointer-anchored), shift-drag = selection → `rvf-viewport` /
  `rvf-select` events. Viewport state lives in the element, not the
  source.

## Review-cycle fixes (found during implementation testing)

- **Query-cache key mismatch**: first query used element width as the
  LOD budget pre-layout, so the post-layout query missed the cache —
  fixed by gating `invalidateView` on a real `plot.w`.
- **Unbounded requery**: draws during an in-flight first query
  re-issued requests; fixed via the in-flight guard above.
- **TDZ crash**: `REFERENCE_VIZ_STYLE` initialized before `PALETTE_KEYS`.
- **`-0` ticks** in `linearTicks`; **1 ms boundary** in the seconds
  formatter.
- **`erasableSyntaxOnly`** forbade constructor parameter properties in
  several classes.

### Review-protocol pass (blocking findings fixed)

- **GL hatch clipping** — `hatchLines` segments deliberately extend
  past the op rect; Canvas2D relied on `ctx.clip` but WebGL2 had no
  per-op scissor, so meter gap-hatches streaked across the canvas on
  the default GL path. Fixed with `pushClip`/`popClip` around each
  hatch draw (`webgl2.ts`).
- **GL-pinned canvas fallback** — once `getContext("webgl2")` succeeds
  on a canvas, `getContext("2d")` on that node returns null forever, so
  both the ensureBackend fallback and `webglcontextlost` recovery were
  dead ends on real browsers. Fixed by swapping in a fresh plot-canvas
  node (`replacePlotCanvas` in `viz-element.ts`) before retrying
  Canvas2D — regression-tested with a context-pinning fake canvas.
- **Superseded in-flight query** — `invalidateView` cache-hit and
  empty-span early returns ran before the seq-bump/abort, so an older
  response could still land wrong-span data. Fixed by superseding
  (`++querySeq` + abort) at the top of every invalidate.
- **Double source subscription** — pre-connect `source` assignment
  subscribed, then `onElementConnect` stacked a second listener the
  cleanup list never removed. Fixed by unsubscribing before rebinding.
- **Pointer gesture identity** — move/up/cancel/lost-capture now filter
  on the drag's `pointerId`; a second pointer can no longer hijack or
  end an in-progress pan/select.
- **Meter silence-vs-missing** — availability is now judged on the raw
  amplitude (finite `0` = silence → empty bar; `NaN`/`±Inf` =
  unavailable → hatch), not on the dB-mapped value where `0` became
  `-Infinity`.
- **Non-finite geometry containment** — `spectrumScene` split line runs
  and skipped gap ticks on non-finite mappings; `waveformScene` clamps
  degenerate amplitude ranges; `linearTicks`/`axisTicks` reject
  non-finite/degenerate inputs and cap iteration at 4096; `setSize`
  normalizes non-finite/negative dimensions; `LruViewCache.set` cannot
  be poisoned by a NaN byte estimate; `hatchLines` clamps non-positive
  spacing; `overlayOps` clamps selection/playhead to the plot rect.
- **Snapshot bookkeeping on swap** — `setSnapshotSource` resets
  `lastSeq`/`dropped`/`stale` (sequence numbers from different sources
  are incomparable), `pull()` exceptions surface as status instead of
  killing the pump, non-finite `sequence`/`dropped` values are guarded,
  and reconnect rebinds the notify subscription.
- **Status channel correctness** — `draw()` preserves subclass status
  text instead of overwriting with "canvas unavailable"; dropped-frame
  count now reaches the DOM status text (loss is non-color-visible);
  the status part is `role="status" aria-live="polite"` and the overlay
  canvas is `aria-hidden`.

## Verification

Ran and green:

- `npm --prefix ui/viz run typecheck` (tsc --noEmit; strict +
  `exactOptionalPropertyTypes` + `erasableSyntaxOnly` +
  `noUncheckedIndexedAccess`)
- `npm --prefix ui/viz test` — 89 tests (`node --test`, happy-dom +
  fake 2D/GL executors): tick/viewport/color math, VizStyle resolution
  incl. `{ref}` chains, scene builders (gaps/partial coverage/NaN runs/
  dB scale), backend op partition, element contracts (bounded stale
  retry, cache hit on pan-back, theme-swap-without-requery, abort on
  unmount, snapshot stale/loss), plus `regression.test.ts` covering the
  review findings above (GL scissor clip, context-pin swap, supersede,
  single-subscribe, pointer id, silence-vs-missing, finite geometry,
  bounded ticks, bookkeeping reset, non-finite `setSize`)
- `mise run check-ui-viz` / `test-ui-viz` / `build-ui-viz` — green
- `mise run check-ui-headless` / `test-ui-headless` and
  `check-ui-components` / `test-ui-components` — #258/#259 lanes green
  (tokens.ts additions non-regressing)
- `mise run fmt-check` / `check` / `lint` / `test` — workspace lanes
  (see report)

CI: `ui-verify` job gained `check-ui-viz`, `test-ui-viz`,
`build-ui-viz` steps (same pattern: `mise install node@26.12.1`,
`npm ci` from committed `package-lock.json`, explicit `node --test`
glob).

## Not verified here (follow-up / manual)

- Real browser/WebView rendering, real WebGL2 raster output, real
  ResizeObserver layout, dpr>1 output, pointer feel — no pixel or GPU
  validation was performed; happy-dom + recorded-ops fakes only.
- Screen-reader announcement behavior — ARIA/status asserted
  structurally only.
- Frame-rate/memory performance under real 60 fps streams — `stats`
  counters exist for consumer fixtures; no benchmark run.
- Spectrogram, TimeSeries/FeaturePlot, spectrogram-style color ramps —
  deferred (#260 full scope / #262 native lane / #263 showcase).

## Boundary notes

- No DAW/analysis/track/playback/project state anywhere in the package:
  sources are ports, elements hold only viewport/presentation state.
- `ui/components` gained the `viz.*` token roles (+ dark map) in
  `tokens.ts` — paint roles only; no control-surface change.
- `MemoryWaveformSource`/`ManualSnapshotSource` are reference
  fixtures (bounded in-memory datasets), not bridges to
  `ruviform-viz`; a native bridge is Issue #262 scope.
