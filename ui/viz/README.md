# @ruviform/ui-viz

Framework-neutral Reference Viz renderers for Ruviform (Issue #260):
Waveform, Level Meter, and Spectrum Custom Elements with a shared
`VizStyle` paint contract, backend-neutral `DrawOp` scenes, and
Canvas2D/WebGL2 executors — built on `@ruviform/ui-core` (Issue #258)
and `@ruviform/ui-components` (Issue #259).

**Plain TypeScript + DOM/Web platform APIs only.** No UI framework,
runtime, or renderer-library dependency is allowed in this package —
not for consumers, not transitively. `happy-dom` appears in
`devDependencies` *only* to provide a DOM for `node --test`; nothing
under `src/` imports it.

**No data processing.** LOD decimation, FFT, and measurement pipelines
stay in `ruviform-viz` / `ruviform-audio-analysis` (Issue #157) behind
the `*DataSource` ports below. This package renders bounded views and
snapshots; it never computes analysis.

## Architecture

```
ui/viz layers (top → bottom)

  elements/     <rvf-waveform> <rvf-meter> <rvf-spectrum>
                Custom Elements: typed properties, lifecycle,
                rvf-viewport / rvf-select / rvf-backend events.
  scene builders waveformScene / meterScene / spectrumScene
                model → DrawOp[] (pure, testable, no canvas).
  backends      createVizBackend → Canvas2DExecutor | WebGl2Executor
                declared capability record + fallback; overlay
                partition keeps text/state cues on 2D under GL.
  style         resolveVizStyle + collectVizTokens
                --rvf-viz-* tokens → ResolvedVizStyle → op colors.
  ports         WaveformDataSource / MeterDataSource /
                SpectrumDataSource — bounded, revision-aware,
                abort-safe. Data/query/model stays OUT of style.
```

The separation between **data/query/model** and **presentation** is the
load-bearing contract of this package: `VizStyle` contains paint-only
values (colors, widths, fonts); a theme swap or `appearance` override
repaints without touching queries, cache keys, or source state.

## Elements

| Tag | Role | Data port | Data shape |
| --- | --- | --- | --- |
| `<rvf-waveform>` | `img` + status | `WaveformDataSource` | bounded `query()` per visible span, f64, `revision`-checked |
| `<rvf-meter>` | `img` + status | `MeterDataSource` | snapshot frames (`peak`/`rms` dBFS) |
| `<rvf-spectrum>` | `img` + status | `SpectrumDataSource` | snapshot frames (`Float64Array` magnitudes) |

Register explicitly — no import-time side effects:

```ts
import { registerRvfVizElements } from "@ruviform/ui-viz";
registerRvfVizElements();
```

### `<rvf-waveform>` properties

- `source: WaveformDataSource` — the data port (see below).
- `viewport: { start, end } | undefined` — visible span in axis units
  (samples or seconds); unset → full extent. Pointer drag pans,
  wheel zooms, shift-drag selects. Emits `rvf-viewport` on change and
  `rvf-select` on selection.
- `overlays: { selection?, playhead?, markers? }` — paint-only overlay
  state; changes repaint without re-querying.
- `range: { min, max }` — amplitude domain (default ±1).
- `channelLabels: string[]` — lane labels.
- `viewCache: LruViewCache<WaveformView>` — bounded LRU (default 8 MiB);
  keyed by source id + revision + channel + span + buckets — style is
  never a key.

### `<rvf-meter>` / `<rvf-spectrum>` properties

- `source: MeterDataSource | SpectrumDataSource` — snapshot port.
- `scale: MeterScale` (`{ minDb, maxDb, segments?, clipDb? }`) — meter.
- `freq: { scale: "linear" | "log", minHz, maxHz }`,
  `db: { minDb, maxDb, reference }` — spectrum.
- `autopump: boolean` — when true (default) a new snapshot repaints
  immediately; when false the element pulls only on `drawNow()`.
- `stale`, `dropped` — read-only snapshot health.

### Shared attributes/properties

`label` (ARIA), `backend="auto|canvas2d|webgl2"` (fallback recorded in
the `rvf-backend` event + `backendKind` getter), `orientation`
(vertical|horizontal, meter), `appearance: VizStyleInput` (typed paint
override — property, never an attribute), `setSize(w,h)` (RO-driven in
real use; deterministic test hook), `stats`, `styleDiagnostics`.

## Data ports

```ts
interface WaveformDataSource {
  meta(): { id, revision, axis: "samples"|"seconds"|"frames", extent, channels, sampleRate? };
  query(request: { start, end, buckets, channels? }, signal?): Promise<WaveformView>;
  subscribe?(notify: () => void): () => void;
}
interface MeterDataSource    { meta(): { id, revision, channels }; snapshot(): MeterFrame | null; subscribe?(fn): () => void; }
interface SpectrumDataSource { meta(): { id, revision, bins, axis: BinAxis, sampleRate? }; snapshot(): SpectrumFrame | null; subscribe?(fn): () => void; }
```

- **Bounded**: waveform queries carry `buckets` = current plot px — the
  LOD budget. Sources may return raw samples or min/max/RMS bins.
- **Revision-safe**: the element rejects a response whose `revision` no
  longer matches `meta()` at resolve time, retrying once before marking
  the view stale.
- **Abort-safe**: source/viewport/unmount changes abort in-flight
  queries via `AbortSignal`.
- **Missing data**: `NaN`/non-finite values are gaps — rendered as
  hatch/gap ticks, never zero-filled or interpolated.
- **Snapshot loss**: meter/spectrum surface `seq` gaps as `dropped` and
  `stale-after` timeouts as a hatch + status text (non-color cues).

Reference ports included: `MemoryWaveformSource` (f64, NaN-preserving,
bounded) and `ManualSnapshotSource<F>` — test fixtures and
single-consumer bridges, not analysis replacements.

## VizStyle

`resolveVizStyle(tokens, appearance)` → immutable `ResolvedVizStyle`.
Resolution order: `--rvf-viz-*` computed values → `appearance`
overrides → resolved paint consumed by DrawOp emission. Token roles
live in `ui/components` `tokens.ts` (`viz.*` + `viz.series.0–7`) so CSS,
Canvas2D, and WebGL2 read one resolved value (§12.3/§12.5).

`collectVizTokens(el)` resolves the composed chain — `<rvf-theme>`
scopes and instance `tokens` (including `{ref}` indirection) — into a
`VizStyleInput` the resolver can consume.

## Backends

`createVizBackend(canvases, preference, size)` returns `{ kind,
fallback, capabilities, execute(ops) }`:

- `canvas2d` — full op support (rect/stroke/band/line/poly/hatch/text/clip).
- `webgl2` — plot layer in GL (triangles/lines; `hatch` and `band` with
  per-x spans are rasterized on the 2D overlay per the declared
  capability record), overlay layer always on 2D so text/state cues
  never depend on GL text support.
- `webglcontextlost` → sticky swap to Canvas2D, reported via
  `rvf-backend` (`fallback.reason: "webglcontextlost"`).

## Testing

```sh
npm ci
npm run typecheck   # tsc --noEmit, strict
npm test            # node --test --import=./test/env.ts test/*.test.ts
npm run build       # dist/ via tsconfig.build.json
```

Suite (78 tests): tick/viewport/color math, style resolution + token
refs, scene builders (gaps, partial coverage, NaN runs, dB scale),
backend op partition + executors against fake 2D/GL contexts, and
element contracts — stale-response rejection (bounded retry), cache
hit on pan-back, theme-swap-repaints-without-requery, abort on
unmount, snapshot stale/loss cues.

## Verification boundary

Headless CI covers ops, scenes, and element contracts — no pixels and
no real GPU. Real browser/WebView rendering, actual WebGL2 raster,
dpr>1 backing stores, and 60 fps interaction on real hardware are
manual-verification items (Issue #262 native-parity lane, Issue #263
showcase/visual-regression evidence).
