/**
 * Element-level tests — `<rvf-waveform>` / `<rvf-meter>` /
 * `<rvf-spectrum>` under happy-dom + fake canvas contexts.
 *
 * The contracts that must hold headlessly (Issue #260):
 * - bounded queries (LOD budget = plot px) + stale-response rejection +
 *   abort-on-disconnect + byte-budgeted LRU caching,
 * - style/theme changes repaint without EVER re-querying data,
 * - backend selection/fallback/status surface as events + DOM text
 *   (non-color channel), never silent,
 * - snapshot pump tracks sequence-gap loss + stale aging.
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  registerRvfVizElements,
  RVF_BACKEND_EVENT,
  type RvfMeter,
  type RvfSpectrum,
  type RvfWaveform,
} from "../src/index.ts";
import {
  ManualSnapshotSource,
  MemoryWaveformSource,
  type MeterDataSource,
  type MeterFrame,
  type SpectrumDataSource,
  type SpectrumFrame,
  type WaveformDataSource,
  type WaveformView,
} from "../src/source.ts";
import { applyTheme } from "../../components/src/tokens.ts";
import { Fake2DContext, installFakeCanvas, mount, unmount } from "./env.ts";

registerRvfVizElements();

const flush = () => new Promise((r) => setTimeout(r, 0));

class CountedWaveform extends MemoryWaveformSource {
  queries = 0;
  override query(
    q: Parameters<MemoryWaveformSource["query"]>[0],
    s?: AbortSignal,
  ): Promise<WaveformView> {
    this.queries += 1;
    return super.query(q, s);
  }
}

/** ManualSnapshotSource + the MeterDataSource meta shape. */
class ManualMeter
  extends ManualSnapshotSource<MeterFrame>
  implements MeterDataSource
{
  constructor() {
    super({ id: "m", revision: 0 });
  }
  override meta(): ReturnType<MeterDataSource["meta"]> {
    return { id: "m", revision: 0, channels: 2 };
  }
}

/** ManualSnapshotSource + the SpectrumDataSource meta shape. */
class ManualSpectrum
  extends ManualSnapshotSource<SpectrumFrame>
  implements SpectrumDataSource
{
  constructor() {
    super({ id: "sp", revision: 0 });
  }
  override meta(): ReturnType<SpectrumDataSource["meta"]> {
    return {
      id: "sp",
      revision: 0,
      bins: 4,
      axis: { kind: "uniform", firstHz: 0, stepHz: 100 },
    };
  }
}

function waveformEl(): RvfWaveform {
  return mount("rvf-waveform", { label: "wf" }) as RvfWaveform;
}

function stereoSource(): CountedWaveform {
  return new CountedWaveform(
    Float64Array.from({ length: 8 }, (_, i) =>
      i % 2 === 0 ? Math.sin(i) : Number.NaN,
    ),
    2,
    { axis: "samples" },
  );
}

describe("registration", () => {
  it("defines the three tags; idempotent", () => {
    registerRvfVizElements();
    assert.ok(customElements.get("rvf-waveform") !== undefined);
    assert.ok(customElements.get("rvf-meter") !== undefined);
    assert.ok(customElements.get("rvf-spectrum") !== undefined);
  });
});

describe("<rvf-waveform>", () => {
  it("queries with a bounded budget and draws a view", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = waveformEl();
      el.setSize(400, 200);
      el.drawNow(); // layout exists → the query issues immediately
      const src = stereoSource();
      el.source = src;
      await flush();
      el.drawNow();
      assert.equal(src.queries, 1);
      assert.ok(el.stats.framesDrawn >= 1);
      assert.ok(el.stats.lastOpCount > 2);
      const ctx = fake.contexts.get("2d") as Fake2DContext;
      assert.ok(ctx.count("fillRect") >= 1);
      unmount(el);
    } finally {
      fake.restore();
    }
  });

  it("rejects a stale response and re-queries once (bounded)", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = waveformEl();
      el.setSize(400, 200);
      let rev = "r0";
      let queries = 0;
      // A source that resolves one macrotask later so a silent revision
      // bump can land between issue and resolve — the stale check must
      // discard the answer.
      const racy: WaveformDataSource = {
        meta: () => ({
          id: "r",
          revision: rev,
          axis: "samples",
          extent: 4,
          channels: 1,
        }),
        query: () => {
          queries += 1;
          const captured = rev;
          return new Promise<WaveformView>((res) =>
            setTimeout(
              () =>
                res({
                  kind: "samples",
                  revision: captured,
                  start: 0,
                  unitsPerSample: 1,
                  channels: [Float64Array.from([1, 1, 1, 1])],
                }),
              0,
            ),
          );
        },
      };
      el.source = racy;
      el.drawNow(); // layout now exists → deferred query issued
      await flush();
      assert.equal(queries, 1);
      rev = "r1"; // silent publish — no notify, revision moved on
      await flush();
      await flush();
      assert.equal(queries, 2); // rejected once, retried once
      assert.equal(el.viewCache.size, 1); // only the valid view cached
      el.drawNow();
      assert.ok(el.stats.framesDrawn >= 1);
      unmount(el);
    } finally {
      fake.restore();
    }
  });

  it("a source that never matches revision is marked stale", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const stale: WaveformDataSource = {
        meta: () => ({
          id: "s",
          revision: "new",
          axis: "samples",
          extent: 4,
          channels: 1,
        }),
        query: () =>
          Promise.resolve({
            kind: "samples",
            revision: "old",
            start: 0,
            unitsPerSample: 1,
            channels: [Float64Array.from([1, 1, 1, 1])],
          }),
      };
      const el = waveformEl();
      el.setSize(400, 200);
      el.drawNow(); // layout first so the query issues immediately
      el.source = stale;
      await flush();
      el.drawNow();
      const status = el.shadowRoot?.querySelector(".status");
      assert.equal(status?.textContent, "stale");
      unmount(el);
    } finally {
      fake.restore();
    }
  });

  it("pan back to a cached span hits the LRU cache (no re-query)", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = waveformEl();
      el.setSize(400, 200);
      el.drawNow();
      const src = stereoSource();
      el.source = src;
      await flush();
      assert.equal(src.queries, 1);
      el.viewport = { start: 0, end: 2 };
      await flush();
      assert.equal(src.queries, 2);
      el.viewport = { start: 0, end: 4 }; // back to the original span
      await flush();
      assert.equal(src.queries, 2); // cache served it
      assert.ok(el.viewCache.hits >= 1);
      unmount(el);
    } finally {
      fake.restore();
    }
  });

  it("style/appearance/theme changes never re-query data", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const scope = document.createElement("div");
      applyTheme(scope, { base: "light" });
      document.body.append(scope);
      const el = waveformEl();
      scope.append(el);
      el.setSize(400, 200);
      el.drawNow();
      const src = stereoSource();
      el.source = src;
      await flush();
      el.drawNow();
      const q0 = src.queries;
      const ctx = fake.contexts.get("2d") as Fake2DContext;
      ctx.calls.length = 0;

      el.appearance = { waveform: { stroke: "#123456" } };
      el.tokens = { "viz.waveform.stroke": "#aabbcc" };
      applyTheme(scope, { base: "dark" });
      await flush();
      el.drawNow();

      assert.equal(src.queries, q0); // ZERO new queries
      assert.ok(el.viewCache.misses <= q0);
      // And paint DID change (the repaint actually happened).
      assert.ok(ctx.calls.length > 0);
      scope.remove();
      unmount(el);
    } finally {
      fake.restore();
    }
  });

  it("disconnect aborts an in-flight query (no post-unmount draw)", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      let release: (v: WaveformView) => void = () => {};
      const pending: WaveformDataSource = {
        meta: () => ({
          id: "p",
          revision: 0,
          axis: "samples",
          extent: 4,
          channels: 1,
        }),
        query: () =>
          new Promise<WaveformView>((res) => {
            release = res;
          }),
      };
      const el = waveformEl();
      el.setSize(400, 200);
      el.drawNow(); // layout first so the query issues immediately
      el.source = pending;
      const drawnAtUnmount = el.stats.framesDrawn;
      unmount(el); // cleanup aborts the query controller
      release({
        kind: "samples",
        revision: 0,
        start: 0,
        unitsPerSample: 1,
        channels: [Float64Array.from([1])],
      });
      await flush();
      // Aborted resolution never lands: no view, no cache entry, no draw.
      assert.equal(el.stats.framesDrawn, drawnAtUnmount);
      assert.equal(el.viewCache.size, 0);
    } finally {
      fake.restore();
    }
  });

  it("no canvas context → DOM unsupported status, no crash", async () => {
    const fake = installFakeCanvas({ contexts: "none" });
    try {
      const el = waveformEl();
      el.setSize(400, 200);
      el.source = stereoSource();
      await flush();
      el.drawNow();
      assert.ok(el.hasAttribute("unsupported"));
      const status = el.shadowRoot?.querySelector(".status");
      assert.ok((status?.textContent ?? "").includes("unavailable"));
      unmount(el);
    } finally {
      fake.restore();
    }
  });

  it("emits rvf-backend with the selected kind + fallback record", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = waveformEl();
      const seen: { kind?: string; requested?: string | undefined } = {};
      el.addEventListener(RVF_BACKEND_EVENT, (e) => {
        const d = (e as CustomEvent).detail as {
          kind: string;
          fallback: { requested: string } | null;
        };
        seen.kind = d.kind;
        seen.requested = d.fallback?.requested;
      });
      el.setSize(400, 200);
      el.source = stereoSource();
      await flush();
      el.drawNow();
      assert.equal(seen.kind, "canvas2d");
      assert.equal(seen.requested, "auto"); // auto → gl missing → fell back
      unmount(el);
    } finally {
      fake.restore();
    }
  });

  it("label → canvas aria-label; status appends to it", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = waveformEl();
      el.setSize(400, 200);
      const plot = el.shadowRoot?.querySelector("canvas.plot");
      assert.equal(plot?.getAttribute("role"), "img");
      assert.equal(plot?.getAttribute("aria-label"), "wf");
      el.drawNow(); // no source → status suffix appears
      assert.equal(plot?.getAttribute("aria-label"), "wf — no source");
      unmount(el);
    } finally {
      fake.restore();
    }
  });
});

describe("<rvf-meter>", () => {
  function meterEl(): RvfMeter {
    return mount("rvf-meter", { label: "m" }) as RvfMeter;
  }
  const src = () => new ManualMeter();

  it("pumps the latest snapshot and paints channels", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = meterEl();
      el.setSize(160, 200);
      el.autopump = false;
      const s = src();
      el.source = s;
      s.push({ sequence: 1, channels: [{ peak: 0.5, rms: 0.3 }] });
      el.drawNow();
      assert.ok(el.stats.lastOpCount > 0);
      const ctx = fake.contexts.get("2d") as Fake2DContext;
      assert.ok(ctx.count("fillRect") >= 1);
      unmount(el);
    } finally {
      fake.restore();
    }
  });

  it("sequence gaps accumulate into dropped (loss stays visible)", () => {
    const el = meterEl();
    el.setSize(160, 200);
    el.autopump = false;
    const s = src();
    el.source = s;
    s.push({ sequence: 1, channels: [] });
    el.pumpNow();
    s.push({ sequence: 4, channels: [], dropped: 1 });
    el.pumpNow();
    assert.equal(el.dropped, 3); // gap(4-1-1=2) + reported 1
    unmount(el);
  });

  it("stale aging flips on the injected clock → status + stale cue", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = meterEl();
      el.setSize(160, 200);
      el.autopump = false;
      el.staleAfterMs = 1000;
      let t = 0;
      el.clock = () => t;
      const s = src();
      el.source = s;
      s.push({ sequence: 1, channels: [{ peak: 0.5, rms: 0.3 }] });
      el.pumpNow();
      t = 2000;
      el.pumpNow(); // no new frame, aged out
      assert.equal(el.stale, true);
      el.drawNow();
      const status = el.shadowRoot?.querySelector(".status");
      assert.equal(status?.textContent, "stale");
      // Recovery: a new frame clears stale.
      s.push({ sequence: 2, channels: [] });
      el.pumpNow();
      assert.equal(el.stale, false);
      unmount(el);
    } finally {
      fake.restore();
    }
  });
});

describe("<rvf-spectrum>", () => {
  it("paints a snapshot; stale + status mirror the meter", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = mount("rvf-spectrum", { label: "sp" }) as RvfSpectrum;
      el.setSize(400, 200);
      el.autopump = false;
      el.staleAfterMs = 500;
      let t = 0;
      el.clock = () => t;
      const s = new ManualSpectrum();
      el.source = s;
      s.push({
        sequence: 1,
        magnitudes: Float64Array.from([1, 0.5, Number.NaN, 0.1]),
      });
      el.pumpNow();
      el.drawNow();
      assert.ok(el.stats.lastOpCount > 0);
      t = 1000;
      el.pumpNow();
      assert.equal(el.stale, true);
      el.drawNow();
      assert.equal(
        el.shadowRoot?.querySelector(".status")?.textContent,
        "stale",
      );
      unmount(el);
    } finally {
      fake.restore();
    }
  });
});
