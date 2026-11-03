/**
 * `@ruviform/ui-viz` — framework-neutral Reference Viz renderers
 * (Issue #260), on the `ui/core` (#258) + `ui/components` (#259)
 * contracts.
 *
 * Layers, top to bottom:
 * - **Elements**: `<rvf-waveform>`, `<rvf-meter>`, `<rvf-spectrum>` —
 *   Custom Elements with typed properties, the `rvf-viewport` /
 *   `rvf-select` / `rvf-backend` event contract, and explicit
 *   registration via {@link registerRvfVizElements} (importing this
 *   module has NO registration side effects).
 * - **Scene builders**: `waveformScene` / `meterScene` / `spectrumScene`
 *   produce backend-neutral {@link DrawOp} lists from typed models —
 *   testable without any canvas.
 * - **Backends**: `createVizBackend` picks Canvas2D or WebGL2 with a
 *   declared capability/fallback record (plot/overlay partition keeps
 *   text + state cues on the 2D overlay when GL paints the plot).
 * - **VizStyle**: `resolveVizStyle` + `collectVizTokens` — the paint
 *   contract over `--rvf-viz-*` tokens, deliberately independent of
 *   data/query/model (a theme swap repaints; it never re-queries).
 * - **Data ports**: `WaveformDataSource` / `MeterDataSource` /
 *   `SpectrumDataSource` — bounded, revision-aware, abort-safe ports
 *   mirroring `ruviform-viz` semantics; LOD/FFT stay behind the port,
 *   never in this package.
 *
 * Framework-neutral by construction: plain TypeScript + DOM/Web
 * platform APIs only.
 */

export * from "./axis.ts";
export * from "./cache.ts";
export * from "./color.ts";
export * from "./layout.ts";
export * from "./ops.ts";
export * from "./scene.ts";
export * from "./source.ts";
export * from "./style.ts";
export * from "./viewport.ts";

export { Canvas2DExecutor, bandXs, hatchLines, type VizExecutor } from "./canvas2d.ts";
export { WebGl2Executor } from "./webgl2.ts";
export {
  createVizBackend,
  type BackendCanvases,
  type BackendPreference,
  type BackendSize,
  type VizBackend,
  type VizBackendCapabilities,
} from "./backend.ts";

export { waveformScene, type WaveformModel } from "./waveform.ts";
export {
  meterScene,
  dbOf,
  DEFAULT_METER_SCALE,
  type MeterModel,
  type MeterScale,
} from "./meter.ts";
export { spectrumScene, type SpectrumModel } from "./spectrum.ts";

export {
  RvfVizElement,
  RVF_BACKEND_EVENT,
  RVF_VIEWPORT_EVENT,
  RVF_SELECT_EVENT,
  type VizStats,
} from "./elements/viz-element.ts";
export {
  SnapshotVizElement,
  type SnapshotFrame,
} from "./elements/snapshot-element.ts";
export { RvfWaveform, type WaveformRange } from "./elements/waveform.ts";
export { RvfMeter } from "./elements/meter.ts";
export { RvfSpectrum } from "./elements/spectrum.ts";

import { RvfWaveform } from "./elements/waveform.ts";
import { RvfMeter } from "./elements/meter.ts";
import { RvfSpectrum } from "./elements/spectrum.ts";

/**
 * Register the Reference Viz custom elements — the explicit mount
 * boundary (same contract as `registerRvfElements` in ui/components).
 * Idempotent: already-defined tags are skipped.
 */
export function registerRvfVizElements(): void {
  const defs: [string, CustomElementConstructor][] = [
    ["rvf-waveform", RvfWaveform],
    ["rvf-meter", RvfMeter],
    ["rvf-spectrum", RvfSpectrum],
  ];
  for (const [tag, ctor] of defs) {
    if (customElements.get(tag) === undefined) {
      customElements.define(tag, ctor);
    }
  }
}
