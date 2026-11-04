/**
 * Showcase matrix page (Issue #310): mounts {@link mountShowcase}
 * over the served dist modules and installs `window.__rvf`.
 *
 * Mirrors `ui/showcase/examples/showcase.html`'s deterministic setup
 * (synchronous `flushWith`, `autopump=false`, one demo frame pumped
 * per column) but leaves receipt handling to `__rvf.drain` — the
 * fixture, not a live listener, decides when commands are acked.
 */

import {
  mountShowcase,
  pumpDemoOnce,
} from "@rvf/showcase";
import {
  installRvfPage,
  type RvfPageOptions,
  type VizLike,
} from "@rvf/browser-test-page";
import type { WaveformSourceLike } from "../src/protocol.ts";

const host = document.getElementById("app");
if (host === null) throw new Error("showcase page: #app missing");

const h = mountShowcase(host, {
  flushWith: (fn) => fn(),
  autopump: false,
});

const viz: Record<string, VizLike> = {};
const waveforms: Record<string, WaveformSourceLike> = {};
for (const col of h.columns) {
  for (const [key, el] of col.viz) {
    const v = el as VizLike;
    viz[`${col.theme.id}.${key}`] = v;
    v.setSize(300, 120);
  }
  waveforms[col.theme.id] = col.sources.waveform;
  pumpDemoOnce(col.sources, 1, 0.2);
}

const opts: RvfPageOptions = {
  gateway: h.gateway,
  receipts: h,
  waveformSources: waveforms,
  viz,
  unmount: () => h.unmount(),
};
installRvfPage(opts);
