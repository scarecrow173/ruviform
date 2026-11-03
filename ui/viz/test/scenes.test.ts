/**
 * Scene-builder tests — `waveformScene` / `meterScene` /
 * `spectrumScene` emit DrawOp lists; assertions inspect structure
 * (op kinds, coordinates, gap cues) without any canvas.
 *
 * Contract under test (Issue #260):
 * - missing data is explicit geometry (wash+hatch / ticks), never
 *   zero-filled or interpolated,
 * - overlay vs plot layers are respected (overlay ops stay on top),
 * - state cues (stale/loss/unavailable) are non-color renderable.
 */
import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { computeLayout } from "../src/layout.ts";
import { partitionOps, type DrawOp } from "../src/ops.ts";
import { meterScene, dbOf, DEFAULT_METER_SCALE } from "../src/meter.ts";
import { spectrumScene } from "../src/spectrum.ts";
import { resolveVizStyle } from "../src/style.ts";
import { viewport } from "../src/viewport.ts";
import { waveformScene, type WaveformModel } from "../src/waveform.ts";
import type { WaveformView } from "../src/source.ts";

const style = resolveVizStyle({}).style;
const layout = computeLayout({
  width: 400,
  height: 200,
  yAxis: true,
  xAxis: true,
  style,
});

function byKind(ops: readonly DrawOp[], kind: DrawOp["kind"]): DrawOp[] {
  return ops.filter((o) => o.kind === kind);
}

describe("waveformScene", () => {
  const vp = viewport(0, 8, 1);
  assert.ok(vp !== undefined);
  const base: WaveformModel = {
    viewport: { ...vp, px: 300 },
    view: {
      kind: "bins",
      revision: 1,
      start: 0,
      unitsPerBucket: 1,
      channels: [
        [
          { min: -0.5, max: 0.5, rms: 0.3, count: 10 },
          { min: -0.6, max: 0.6, rms: 0.4, count: 10 },
        ],
      ],
    },
    range: { min: -1, max: 1 },
    axisUnit: "seconds",
    samplesPerUnit: 10,
  };

  it("emits envelope band + rms band + zero line + axes", () => {
    const ops = waveformScene(base, layout, style);
    const bands = byKind(ops, "band");
    assert.ok(bands.length >= 2); // envelope + rms
    assert.ok(byKind(ops, "text").length > 0); // axis + amp labels
    const { overlay } = partitionOps(ops);
    assert.ok(overlay.every((o) => o.layer === "overlay"));
  });

  it("count===0 buckets become gap wash+hatch; partial counts → ticks", () => {
    const view: WaveformView = {
      kind: "bins",
      revision: 1,
      start: 0,
      unitsPerBucket: 1,
      channels: [
        [
          { min: -0.5, max: 0.5, rms: 0.3, count: 10 },
          { min: Number.NaN, max: Number.NaN, rms: Number.NaN, count: 0 },
          { min: -0.4, max: 0.4, rms: 0.2, count: 3 }, // < expected 10 → partial
        ],
      ],
    };
    const ops = waveformScene({ ...base, view }, layout, style);
    const hatches = byKind(ops, "hatch");
    assert.ok(hatches.length >= 1); // gap region hatched
    const gapRects = byKind(ops, "rect").filter(
      (o) => o.kind === "rect" && o.fill === style.state.gapFill,
    );
    assert.ok(gapRects.length >= 1);
    // Partial-coverage tick: a short vertical line near the lane bottom.
    assert.ok(
      byKind(ops, "line").some(
        (o) => o.kind === "line" && o.stroke === style.state.gap,
      ),
    );
  });

  it("raw samples split polylines at NaN; single sample → dot", () => {
    const view: WaveformView = {
      kind: "samples",
      revision: 1,
      start: 0,
      unitsPerSample: 1,
      channels: [[0.1, 0.2, Number.NaN, 0.3, 0.4, Number.NaN, 0.5]],
    };
    const ops = waveformScene({ ...base, view }, layout, style);
    const lines = byKind(ops, "line").filter(
      (o) => o.kind === "line" && o.stroke === style.waveform.stroke,
    );
    // Runs [0.1,0.2] and [0.3,0.4] → two polylines; trailing 0.5 → dot.
    assert.equal(lines.length, 2);
    const dots = byKind(ops, "rect").filter(
      (o) => o.kind === "rect" && o.fill === style.waveform.stroke,
    );
    assert.ok(dots.length >= 1);
    // The NaN samples produced gap regions.
    assert.ok(byKind(ops, "hatch").length >= 1);
  });

  it("multi-channel data gets lane-separated bands + clip ops", () => {
    const view: WaveformView = {
      kind: "bins",
      revision: 1,
      start: 0,
      unitsPerBucket: 1,
      channels: [
        [{ min: -0.5, max: 0.5, rms: 0.3, count: 10 }],
        [{ min: -0.2, max: 0.2, rms: 0.1, count: 10 }],
      ],
    };
    const ops = waveformScene(
      { ...base, view, channelLabels: ["L", "R"] },
      layout,
      style,
    );
    const clips = byKind(ops, "clipPush");
    assert.equal(clips.length, 2); // one clip per lane
    const labelTexts = byKind(ops, "text").filter(
      (o) => o.kind === "text" && (o.text === "L" || o.text === "R"),
    );
    assert.equal(labelTexts.length, 2);
  });

  it("selection/playhead/markers are overlay ops in axis units", () => {
    const ops = waveformScene(
      {
        ...base,
        overlays: {
          selection: { start: 2, end: 4 },
          playhead: 5,
          markers: [{ at: 6, label: "m1" }],
        },
      },
      layout,
      style,
    );
    const sel = byKind(ops, "rect").find(
      (o) => o.kind === "rect" && o.fill === style.overlay.selectionFill,
    );
    assert.ok(sel !== undefined && sel.kind === "rect");
    assert.ok(sel.w > 0);
    const markerLabel = byKind(ops, "text").find(
      (o) => o.kind === "text" && o.text === "m1",
    );
    assert.ok(markerLabel !== undefined);
    // All overlay ops come after all plot ops (layer discipline).
    const { overlay } = partitionOps(ops);
    const selOp = overlay.find((o) => o === sel);
    assert.ok(selOp !== undefined && selOp.layer === "overlay");
  });

  it("f64 values pass through geometry unchanged (no quantization)", () => {
    const odd = 0.30000000000000004;
    const view: WaveformView = {
      kind: "samples",
      revision: 1,
      start: 0,
      unitsPerSample: 1,
      channels: [[odd, odd, odd]],
    };
    const ops = waveformScene({ ...base, view }, layout, style);
    const line = byKind(ops, "line").find(
      (o) => o.kind === "line" && o.stroke === style.waveform.stroke,
    );
    assert.ok(line !== undefined && line.kind === "line");
    // y for +odd maps inside the lane — exact value check.
    const expectedY =
      // laneY: lane.y + lane.h - (v - min)/span * lane.h
      // full lane = plot (1 channel), so:
      layout.plot.y +
      layout.plot.h -
      ((odd - -1) / 2) * layout.plot.h;
    assert.equal(line.pts[1], expectedY);
  });
});

describe("meterScene", () => {
  const base = {
    channels: [{ peak: 0.5, rms: 0.3 }],
    scale: DEFAULT_METER_SCALE,
    stale: false,
    dropped: 0,
    orientation: "vertical" as const,
  };

  it("dbOf maps amplitude to dB", () => {
    assert.equal(dbOf(1), 0);
    assert.equal(dbOf(0.5), 20 * Math.log10(0.5));
    assert.ok(!Number.isFinite(dbOf(0)));
  });

  it("peak bar segmented low/mid/high; rms line present", () => {
    const ops = meterScene(base, layout, style);
    const fills = byKind(ops, "rect").filter(
      (o) =>
        o.kind === "rect" &&
        (o.fill === style.meter.low ||
          o.fill === style.meter.mid ||
          o.fill === style.meter.high),
    );
    assert.ok(fills.length >= 1);
    const rmsBar = byKind(ops, "rect").filter(
      (o) => o.kind === "rect" && o.fill === style.meter.peak,
    );
    assert.ok(rmsBar.length >= 1);
    assert.ok(byKind(ops, "text").length > 0); // dB labels
  });

  it("clip flag lights indicator; heldPeak draws a tick", () => {
    const ops = meterScene(
      {
        ...base,
        channels: [{ peak: 1.1, rms: 0.9, clip: true, heldPeak: 0.95 }],
      },
      layout,
      style,
    );
    assert.ok(
      byKind(ops, "rect").some(
        (o) => o.kind === "rect" && o.fill === style.meter.clip,
      ),
    );
    // held-peak tick (peak-color rect in the bar)
    const holds = byKind(ops, "rect").filter(
      (o) => o.kind === "rect" && o.fill === style.meter.peak,
    );
    assert.ok(holds.length >= 2); // rms line + hold tick
  });

  it("NaN channel → hatched lane, no level fill", () => {
    const ops = meterScene(
      {
        ...base,
        channels: [{ peak: Number.NaN, rms: Number.NaN }],
      },
      layout,
      style,
    );
    assert.ok(byKind(ops, "hatch").length >= 1);
    assert.ok(
      !byKind(ops, "rect").some(
        (o) => o.kind === "rect" && o.fill === style.meter.low,
      ),
    );
  });

  it("stale → overlay hatch + text; dropped → loss badge", () => {
    const ops = meterScene(
      { ...base, stale: true, dropped: 3 },
      layout,
      style,
    );
    const { overlay } = partitionOps(ops);
    assert.ok(overlay.some((o) => o.kind === "hatch"));
    assert.ok(
      overlay.some((o) => o.kind === "text" && o.text === "stale"),
    );
    assert.ok(
      overlay.some((o) => o.kind === "text" && o.text === "−3"),
    );
  });

  it("horizontal orientation stacks channels as rows", () => {
    const ops = meterScene(
      {
        ...base,
        orientation: "horizontal",
        channels: [
          { peak: 0.5, rms: 0.3 },
          { peak: 0.4, rms: 0.2 },
        ],
      },
      layout,
      style,
    );
    const tracks = byKind(ops, "rect").filter(
      (o) => o.kind === "rect" && o.fill === style.meter.track,
    );
    assert.equal(tracks.length, 2);
    const [a, b] = tracks;
    assert.ok(a !== undefined && b !== undefined);
    if (a.kind === "rect" && b.kind === "rect") {
      assert.ok(a.y < b.y); // stacked vertically
      assert.equal(a.x, b.x); // same column
    }
  });
});

describe("spectrumScene", () => {
  const model = {
    magnitudes: [1, 0.5, 0.25, 0.1],
    axis: { kind: "uniform" as const, firstHz: 0, stepHz: 100 },
    freq: { scale: "linear" as const, minHz: 0, maxHz: 400 },
    db: { minDb: -60, maxDb: 0, reference: 1 },
  };

  it("curve stroke + under-fill band + hz/db axes", () => {
    const ops = spectrumScene(model, layout, style);
    const bands = byKind(ops, "band").filter(
      (o) => o.kind === "band" && o.fill === style.spectrum.fill,
    );
    assert.equal(bands.length, 1);
    const curve = byKind(ops, "line").filter(
      (o) => o.kind === "line" && o.stroke === style.spectrum.stroke,
    );
    assert.equal(curve.length, 1);
    const hzLabels = byKind(ops, "text").filter(
      (o) => o.kind === "text" && /k|\d/.test(o.text),
    );
    assert.ok(hzLabels.length > 0);
  });

  it("NaN bins break the curve and emit gap ticks", () => {
    const ops = spectrumScene(
      { ...model, magnitudes: [1, Number.NaN, 0.25, 0.1] },
      layout,
      style,
    );
    const curves = byKind(ops, "line").filter(
      (o) => o.kind === "line" && o.stroke === style.spectrum.stroke,
    );
    assert.equal(curves.length, 2); // two runs around the NaN
    const gapTicks = byKind(ops, "line").filter(
      (o) => o.kind === "line" && o.stroke === style.state.gap,
    );
    assert.ok(gapTicks.length >= 1);
  });

  it("log axis places bins non-uniformly (explicit xs)", () => {
    const ops = spectrumScene(
      {
        ...model,
        axis: { kind: "uniform" as const, firstHz: 100, stepHz: 100 },
        freq: { scale: "log" as const, minHz: 50, maxHz: 2000 },
      },
      layout,
      style,
    );
    const band = byKind(ops, "band").find(
      (o) => o.kind === "band" && o.fill === style.spectrum.fill,
    );
    assert.ok(band !== undefined && band.kind === "band");
    assert.ok("xs" in band.span); // non-uniform positions
  });

  it("stale + loss badges render like the meter", () => {
    const ops = spectrumScene(
      { ...model, stale: true, dropped: 2 },
      layout,
      style,
    );
    const { overlay } = partitionOps(ops);
    assert.ok(overlay.some((o) => o.kind === "hatch"));
    assert.ok(overlay.some((o) => o.kind === "text" && o.text === "−2"));
  });
});
