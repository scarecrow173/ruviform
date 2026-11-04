/**
 * Analysis-shaped consumer fixture page (Issue #310 — "Consumer B").
 *
 * Mounts {@link mountAnalysisFixture}: waveform over a bounded-query
 * source + meter/spectrum over the snapshot port + a QueryBinding for
 * bounded metadata/feature pulls, on a DemoAnalysisBackend whose parked
 * queries the test resolves/rejects through `__rvf`.
 */

import { mountAnalysisFixture, pumpDemoOnce } from "@rvf/showcase";
import { installRvfPage, type VizLike } from "@rvf/browser-test-page";

const host = document.getElementById("app");
if (host === null) throw new Error("analysis page: #app missing");

const h = mountAnalysisFixture(host, {
  flushWith: (fn) => fn(),
  autopump: false,
});

// The waveform runs the pixel-probe lane (gap semantics need
// getImageData) — pin Canvas2D so the probe is never on a GL-bound
// canvas. Meter/spectrum stay on `auto`: whatever backend the browser
// picks is the capability record the viz spec asserts honestly.
h.waveform.setAttribute("backend", "canvas2d");

const vizEls: Record<string, VizLike> = {
  waveform: h.waveform as VizLike,
  meter: h.meter as VizLike,
  spectrum: h.spectrum as VizLike,
};
for (const el of Object.values(vizEls)) el.setSize(300, 120);
pumpDemoOnce(h.sources.snapshots, 1, 0.2);

installRvfPage({
  backend: h.backend,
  query: h.query,
  waveformSources: { waveform: h.sources.waveform },
  viz: vizEls,
  unmount: () => h.unmount(),
});
