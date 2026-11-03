/**
 * Regression tests — the review-protocol findings for Issue #260.
 *
 * Each test names the violated invariant it guards, not just the
 * symptom: GL hatch clipping, context-exclusivity canvas swap,
 * superseded-query rejection, single-subscribe lifecycle,
 * silence-vs-missing meter semantics, finite scene geometry under
 * hostile inputs, bounded tick generation, and snapshot bookkeeping
 * reset on source swap.
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  registerRvfVizElements,
  type RvfMeter,
  type RvfWaveform,
} from "../src/index.ts";
import { linearTicks } from "../src/axis.ts";
import { axisTicks } from "../src/scene.ts";
import { computeLayout } from "../src/layout.ts";
import { meterScene, DEFAULT_METER_SCALE } from "../src/meter.ts";
import { spectrumScene } from "../src/spectrum.ts";
import {
  ManualSnapshotSource,
  MemoryWaveformSource,
  type MeterDataSource,
  type MeterFrame,
  type WaveformDataSource,
  type WaveformView,
} from "../src/source.ts";
import { REFERENCE_VIZ_STYLE } from "../src/style.ts";
import { WebGl2Executor } from "../src/webgl2.ts";
import { partitionOps, type DrawOp } from "../src/ops.ts";
import { dom, FakeGL, installFakeCanvas, mount, unmount } from "./env.ts";

registerRvfVizElements();

const flush = () => new Promise((r) => setTimeout(r, 0));
const LAYOUT = computeLayout({
  width: 400,
  height: 200,
  yAxis: true,
  xAxis: true,
  style: REFERENCE_VIZ_STYLE,
});

function finiteOps(ops: readonly DrawOp[]): void {
  for (const op of ops) {
    for (const v of Object.values(op)) {
      if (typeof v === "number") {
        assert.ok(
          Number.isFinite(v),
          `non-finite ${JSON.stringify(v)} in ${op.kind}`,
        );
      } else if (Array.isArray(v)) {
        for (const n of v) {
          if (typeof n === "number") {
            assert.ok(Number.isFinite(n), `non-finite entry in ${op.kind}`);
          }
        }
      }
    }
  }
}

describe("review: WebGL hatch clipping", () => {
  it("hatch ops scissor to their rect (Canvas2D parity)", () => {
    const gl = new FakeGL();
    const ex = new WebGl2Executor(
      gl as unknown as WebGL2RenderingContext,
      100,
      50,
      1,
    );
    const ops: DrawOp[] = [
      {
        kind: "hatch", layer: "plot", x: 5, y: 6, w: 20, h: 10,
        stroke: "#ffffff", width: 1, spacing: 4, angle: Math.PI / 4,
      },
    ];
    const { plot } = partitionOps(ops);
    ex.execute(plot);
    const names = gl.calls.map((c) => c.method);
    const sci = names.indexOf("scissor");
    const dis = names.lastIndexOf("disable");
    assert.ok(sci >= 0, "hatch must scissor — lines extend past the rect");
    assert.ok(dis > sci, "clip must be popped after the hatch draw");
    // The scissor rect mirrors the op rect (y-flipped GL space).
    const sc = gl.calls[sci];
    assert.equal(sc?.args[0], 5);
    assert.equal(sc?.args[2], 20);
    assert.equal(sc?.args[3], 10);
  });
});

describe("review: GL-pinned canvas fallback", () => {
  it("webglcontextlost swaps to a FRESH canvas → canvas2d still works", async () => {
    // "pin" models real context exclusivity: the GL-pinned node refuses
    // 2d; a replacement node does not.
    const fake = installFakeCanvas({ contexts: "pin" });
    try {
      const el = mount("rvf-waveform", { label: "wf" }) as RvfWaveform;
      el.setSize(400, 200);
      const src = new MemoryWaveformSource(
        Float64Array.from([0, 1, 0, -1, 0, 1, 0, -1]),
        1,
        { axis: "samples" },
      );
      el.source = src;
      el.drawNow();
      await flush();
      el.drawNow();
      assert.equal(el.backendKind, "webgl2");
      const oldCanvas = el.shadowRoot?.querySelector("canvas.plot");
      oldCanvas?.dispatchEvent(
        new dom.window.Event("webglcontextlost") as unknown as Event,
      );
      await flush();
      el.drawNow();
      // Without the node swap, getContext("2d") on the pinned canvas is
      // null forever → "canvas unavailable". The swap must succeed.
      assert.equal(el.backendKind, "canvas2d");
      assert.equal(el.hasAttribute("unsupported"), false);
      const newCanvas = el.shadowRoot?.querySelector("canvas.plot");
      assert.notEqual(newCanvas, oldCanvas);
      unmount(el);
    } finally {
      fake.restore();
    }
  });
});

describe("review: superseded waveform queries", () => {
  it("a cache-hit invalidate still supersedes an in-flight query", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = mount("rvf-waveform", { label: "wf" }) as RvfWaveform;
      el.setSize(400, 200);
      el.drawNow();
      let releaseB: ((v: WaveformView) => void) | undefined;
      const src: WaveformDataSource = {
        meta: () => ({
          id: "sup", revision: 0, axis: "samples", extent: 8, channels: 1,
        }),
        query: (q) => {
          const mk = (): WaveformView => ({
            kind: "samples", revision: 0, start: q.start,
            unitsPerSample: 1, channels: [Float64Array.from([0, 1])],
          });
          if (q.start !== 0) {
            // Span B stays pending until the test releases it.
            return new Promise<WaveformView>((res) => {
              releaseB = res;
            });
          }
          return Promise.resolve(mk());
        },
      };
      el.source = src;
      await flush(); // span A (0..8) resolved + cached
      el.viewport = { start: 4, end: 8 }; // span B → in-flight
      await flush();
      el.viewport = { start: 0, end: 8 }; // back to span A → cache hit
      await flush();
      // B's response arrives AFTER it was superseded — it must not
      // land in the view or the cache.
      releaseB?.({
        kind: "samples", revision: 0, start: 0,
        unitsPerSample: 1, channels: [Float64Array.from([9, 9])],
      });
      await flush();
      assert.equal(el.viewCache.size, 1, "superseded response cached");
      unmount(el);
    } finally {
      fake.restore();
    }
  });

  it("pre-connect source assignment subscribes exactly once", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      let subs = 0;
      let queries = 0;
      const listeners = new Set<() => void>();
      const inner = new MemoryWaveformSource(
        Float64Array.from([0, 1, 0, -1]),
        1,
        { axis: "samples" },
      );
      const src: WaveformDataSource = {
        meta: () => inner.meta(),
        query: (q, s) => {
          queries += 1;
          return inner.query(q, s);
        },
        subscribe: (fn) => {
          subs += 1;
          listeners.add(fn);
          return () => listeners.delete(fn);
        },
      };
      const el = document.createElement("rvf-waveform") as RvfWaveform;
      el.source = src; // detached — pre-connect subscribe
      assert.equal(subs, 1);
      document.body.append(el); // connect rebinds, not stacks
      assert.equal(listeners.size, 1, "stacked listeners");
      el.setSize(400, 200);
      el.drawNow();
      await flush();
      el.drawNow();
      const q0 = queries;
      // A revision bump with ONE live listener issues exactly one
      // query — a stacked second listener would double-invalidate.
      inner.replace(Float64Array.from([0, -1, 0, 1]));
      for (const fn of listeners) fn();
      await flush();
      assert.equal(queries - q0, 1, "double-subscribed source listener");
      unmount(el);
    } finally {
      fake.restore();
    }
  });
});

describe("review: pointer gesture identity", () => {
  function pointer(
    type: string,
    pointerId: number,
    clientX: number,
    shiftKey = false,
  ): Event {
    const e = new dom.window.MouseEvent(type, {
      bubbles: true,
      clientX,
      shiftKey,
    }) as unknown as Event;
    Object.defineProperty(e, "pointerId", { value: pointerId });
    return e;
  }

  it("a second pointer cannot move or end another pointer's drag", async () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = mount("rvf-waveform", { label: "wf" }) as RvfWaveform;
      el.setSize(400, 200);
      el.drawNow();
      el.source = new MemoryWaveformSource(
        Float64Array.from({ length: 16 }, (_, i) => Math.sin(i)),
        1,
        { axis: "samples" },
      );
      await flush();
      el.drawNow();
      const plot = el.shadowRoot?.querySelector("canvas.plot");
      assert.ok(plot !== null && plot !== undefined);
      let selects = 0;
      el.addEventListener("rvf-select", () => {
        selects += 1;
      });
      plot.dispatchEvent(pointer("pointerdown", 1, 100, true));
      plot.dispatchEvent(pointer("pointermove", 2, 300)); // wrong id
      plot.dispatchEvent(pointer("pointerup", 2, 300));   // wrong id
      assert.equal(selects, 0);
      plot.dispatchEvent(pointer("pointermove", 1, 300));
      plot.dispatchEvent(pointer("pointerup", 1, 300));
      assert.equal(selects, 1, "the owning pointer ends the selection");
      unmount(el);
    } finally {
      fake.restore();
    }
  });
});

describe("review: meter silence vs missing", () => {
  it("peak 0 is silence (empty bar), NaN/±Inf is unavailable (hatch)", () => {
    const style = REFERENCE_VIZ_STYLE;
    const mk = (peak: number, rms: number) =>
      meterScene(
        {
          channels: [{ peak, rms }],
          scale: DEFAULT_METER_SCALE,
          stale: false,
          dropped: 0,
          orientation: "vertical",
        },
        LAYOUT,
        style,
      );
    const silent = mk(0, 0);
    const missing = mk(Number.NaN, Number.NaN);
    const inf = mk(Number.POSITIVE_INFINITY, Number.POSITIVE_INFINITY);
    assert.equal(
      silent.filter((o) => o.kind === "hatch").length,
      0,
      "silence must not render the unavailable hatch",
    );
    assert.ok(missing.some((o) => o.kind === "hatch"));
    assert.ok(inf.some((o) => o.kind === "hatch"));
    finiteOps(silent);
    finiteOps(missing);
    finiteOps(inf);
  });
});

describe("review: hostile scene inputs stay finite", () => {
  const style = REFERENCE_VIZ_STYLE;

  it("spectrum with log scale + NaN/out-of-range bins", () => {
    const ops = spectrumScene(
      {
        magnitudes: Float64Array.from([
          1, Number.NaN, 0.5, Number.NaN, -1, 0.25,
        ]),
        axis: { kind: "centers", hz: [20, 40, 80, 160, 320, 640] },
        freq: { scale: "log", minHz: 20, maxHz: 640 },
        db: { minDb: -90, maxDb: 0, reference: 1 },
      },
      LAYOUT,
      style,
    );
    finiteOps(ops);
    assert.ok(ops.some((o) => o.kind === "line"));
  });

  it("spectrum with a degenerate freq window emits no NaN geometry", () => {
    const ops = spectrumScene(
      {
        magnitudes: Float64Array.from([1, 0.5, 0.25]),
        axis: { kind: "uniform", firstHz: 0, stepHz: 100 },
        freq: { scale: "log", minHz: 0, maxHz: 0 }, // degenerate
        db: { minDb: -90, maxDb: 0, reference: 1 },
      },
      LAYOUT,
      style,
    );
    finiteOps(ops);
  });

  it("tick generation is bounded under pathological inputs", () => {
    assert.equal(linearTicks(0, 1e12, 1e-300, 56).length <= 4096, true);
    assert.equal(linearTicks(0, -1, 1, 56).length, 0);
    assert.equal(linearTicks(0, 10, Number.NaN, 56).length, 0);
    assert.equal(
      axisTicks({ axisPx: 0, min: 0, max: 10, unit: "unit" }).length,
      0,
    );
    assert.equal(
      axisTicks({ axisPx: Number.NaN, min: 0, max: 10, unit: "unit" })
        .length,
      0,
    );
  });

  it("non-finite setSize clamps to zero, not NaN layout", () => {
    const fake = installFakeCanvas({ contexts: "2d" });
    try {
      const el = mount("rvf-waveform", { label: "wf" }) as RvfWaveform;
      el.setSize(Number.NaN, Number.POSITIVE_INFINITY);
      assert.equal(el.size.width, 0);
      assert.equal(el.size.height, 0);
      el.setSize(-50, 200);
      assert.equal(el.size.width, 0);
      assert.equal(el.size.height, 200);
      unmount(el);
    } finally {
      fake.restore();
    }
  });
});

describe("review: snapshot bookkeeping reset on swap", () => {
  class ManualMeter extends ManualSnapshotSource<MeterFrame> {
    constructor() {
      super({ id: "m", revision: 0 });
    }
    override meta(): ReturnType<MeterDataSource["meta"]> {
      return { id: "m", revision: 0, channels: 2 };
    }
  }

  it("a new source must not inherit the old one's loss bookkeeping", () => {
    const el = mount("rvf-meter", { label: "m" }) as RvfMeter;
    el.setSize(160, 200);
    el.autopump = false;
    const a = new ManualMeter();
    el.source = a;
    a.push({ sequence: 1, channels: [] });
    el.pumpNow();
    a.push({ sequence: 5, channels: [] }); // gap of 3
    el.pumpNow();
    assert.equal(el.dropped, 3);
    const b = new ManualMeter();
    el.source = b;
    assert.equal(el.dropped, 0, "loss carried across sources");
    b.push({ sequence: 42, channels: [] }); // first frame — no gap
    el.pumpNow();
    assert.equal(el.dropped, 0, "phantom gap from stale lastSeq");
    unmount(el);
  });
});
