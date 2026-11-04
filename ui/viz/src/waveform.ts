/**
 * Waveform scene builder (Issue #260).
 *
 * Consumes a bounded `WaveformView` (bins or raw samples — already
 * reduced by the source port; no LOD recomputation here) plus viewport
 * and style, and emits the DrawOp list: frame, per-channel lanes,
 * min/max envelope + RMS band, NaN gap wash/hatch, partial-coverage
 * ticks, zero lines, amplitude + time axes, and selection/playhead/
 * marker overlays.
 *
 * Amplitude mapping is display-side only (`range` → lane y); view data
 * is never modified. `count === 0` buckets and non-finite samples
 * become explicit gap marks — never zero-filled, never interpolated.
 */

import type { AxisUnit } from "./axis.ts";
import type { Rect, VizLayout } from "./layout.ts";
import { channelLanes } from "./layout.ts";
import type { DrawOp } from "./ops.ts";
import {
  frameOps,
  gapOps,
  gapTickOps,
  overlayOps,
  xAxisOps,
  xMapper,
  type OverlayState,
} from "./scene.ts";
import type { WaveformBucket, WaveformView } from "./source.ts";
import type { ResolvedVizStyle } from "./style.ts";
import { vpToPx, vpUnitsPerPx, type Viewport } from "./viewport.ts";

const P = "plot" as const;

export interface WaveformModel {
  readonly viewport: Viewport;
  readonly view: WaveformView;
  /** Amplitude range mapped to a full lane (display mapping). */
  readonly range: { readonly min: number; readonly max: number };
  readonly overlays?: OverlayState;
  /** Axis unit for the bottom time ruler. */
  readonly axisUnit: AxisUnit;
  /** Finite samples expected per axis unit — used to mark *partial*
   * coverage (a gap tick) vs full buckets. `undefined` disables the
   * partial cue (e.g. a `frames` axis without a known rate). */
  readonly samplesPerUnit?: number;
  /** Optional per-channel labels drawn at each lane's top-left. */
  readonly channelLabels?: readonly string[];
}

export function waveformScene(
  model: WaveformModel,
  layout: VizLayout,
  style: ResolvedVizStyle,
): DrawOp[] {
  const ops: DrawOp[] = [...frameOps(layout, style)];
  const { view, viewport, range } = model;
  const nCh = view.channels.length;
  const lanes = channelLanes(
    layout.plot,
    Math.max(1, nCh),
    style.waveform.channelGapPx,
  );
  const px0 = layout.plot.x;
  // Degenerate amplitude ranges (min >= max, non-finite) would emit
  // NaN/Inf lane coordinates — clamp to a finite display mapping.
  const span =
    Number.isFinite(range.min) &&
    Number.isFinite(range.max) &&
    range.max > range.min
      ? range.max - range.min
      : 1;
  const laneY = (lane: Rect, v: number): number =>
    lane.y + lane.h - ((v - range.min) / span) * lane.h;

  const gapRanges: [number, number][] = [];
  const partialXs: number[] = [];

  for (let c = 0; c < nCh; c++) {
    const lane = lanes[c];
    const chan = view.channels[c];
    if (lane === undefined || chan === undefined) continue;
    ops.push({
      kind: "clipPush",
      layer: P,
      x: lane.x,
      y: lane.y,
      w: lane.w,
      h: lane.h,
    });
    gapRanges.length = 0;
    partialXs.length = 0;

    if (view.kind === "bins") {
      const buckets = chan as readonly WaveformBucket[];
      const upb = view.unitsPerBucket;
      const pxPerBucket = upb / vpUnitsPerPx(viewport);
      const expected =
        model.samplesPerUnit !== undefined
          ? upb * model.samplesPerUnit
          : undefined;
      let runTop: number[] = [];
      let runBottom: number[] = [];
      let runStart = 0;
      // RMS runs mirror the envelope's split-at-missing semantics: one
      // `xs`-spanned band per contiguous run — never a phantom band
      // interpolated across a dropout.
      const rmsRuns: { xs: number[]; top: number[]; bottom: number[] }[] =
        [];
      let rmsTop: number[] = [];
      let rmsBottom: number[] = [];
      let rmsXs: number[] = [];
      const flushRms = () => {
        if (rmsXs.length > 0) {
          rmsRuns.push({ xs: rmsXs, top: rmsTop, bottom: rmsBottom });
          rmsXs = [];
          rmsTop = [];
          rmsBottom = [];
        }
      };
      const flush = () => {
        if (runTop.length > 0) {
          ops.push({
            kind: "band",
            layer: P,
            span: {
              x0: px0 + vpToPx(viewport, view.start + runStart * upb),
              dx: pxPerBucket,
            },
            top: runTop,
            bottom: runBottom,
            fill: style.waveform.fill,
          });
          runTop = [];
          runBottom = [];
        }
      };
      for (let i = 0; i < buckets.length; i++) {
        const b = buckets[i];
        if (b === undefined || b.count === 0) {
          flush();
          flushRms();
          const a = view.start + i * upb;
          mergeGap(gapRanges, a, a + upb);
          continue;
        }
        if (runTop.length === 0) runStart = i;
        runTop.push(laneY(lane, b.max));
        runBottom.push(laneY(lane, b.min));
        if (expected !== undefined && b.count < Math.floor(expected)) {
          partialXs.push(px0 + vpToPx(viewport, view.start + (i + 0.5) * upb));
        }
        if (style.waveform.showRms && Number.isFinite(b.rms)) {
          rmsXs.push(px0 + vpToPx(viewport, view.start + (i + 0.5) * upb));
          rmsTop.push(laneY(lane, b.rms));
          rmsBottom.push(laneY(lane, -b.rms));
        } else {
          flushRms();
        }
      }
      flush();
      flushRms();
      if (style.waveform.showRms) {
        for (const run of rmsRuns) {
          ops.push({
            kind: "band",
            layer: P,
            span: { xs: run.xs },
            top: run.top,
            bottom: run.bottom,
            fill: style.waveform.rms,
          });
        }
      }
    } else {
      // Raw-sample view: polyline runs split at non-finite values.
      const usp = view.unitsPerSample;
      const samples = chan as ArrayLike<number>;
      let pts: number[] = [];
      const emitRun = () => {
        if (pts.length >= 4) {
          ops.push({
            kind: "line",
            layer: P,
            pts,
            stroke: style.waveform.stroke,
            width: 1,
          });
        } else if (pts.length === 2) {
          // Single finite sample — a dot, not an invisible point.
          ops.push({
            kind: "rect",
            layer: P,
            x: (pts[0] ?? 0) - 0.5,
            y: (pts[1] ?? 0) - 1,
            w: 1,
            h: 2,
            fill: style.waveform.stroke,
          });
        }
        pts = [];
      };
      for (let i = 0; i < samples.length; i++) {
        const v = samples[i] ?? Number.NaN;
        const x = px0 + vpToPx(viewport, view.start + i * usp);
        if (!Number.isFinite(v)) {
          emitRun();
          mergeGap(
            gapRanges,
            view.start + i * usp,
            view.start + (i + 1) * usp,
          );
          continue;
        }
        pts.push(x, laneY(lane, v));
      }
      emitRun();
    }

    ops.push(...gapOps(lane, style, viewport, lane.x, gapRanges));
    if (partialXs.length > 0) ops.push(...gapTickOps(lane, style, partialXs));
    // Zero reference line (mid for symmetric ranges).
    if (range.min < 0 && range.max > 0) {
      const y0 = laneY(lane, 0);
      ops.push({
        kind: "line",
        layer: P,
        pts: [lane.x, y0, lane.x + lane.w, y0],
        stroke: style.grid,
        width: style.gridWidth,
      });
    }
    const label = model.channelLabels?.[c];
    if (label !== undefined && label !== "") {
      ops.push({
        kind: "text",
        layer: "overlay",
        x: lane.x + 4,
        y: lane.y + 2,
        text: label,
        fill: style.tick,
        font: style.font,
        align: "left",
        baseline: "top",
      });
    }
    ops.push({ kind: "clipPop", layer: P });
  }

  // Amplitude labels on the left edge of the top lane (min/mid/max).
  const top = lanes[0];
  if (top !== undefined && nCh > 0) {
    for (const v of [range.min, (range.min + range.max) / 2, range.max]) {
      ops.push({
        kind: "text",
        layer: "overlay",
        x: layout.plot.x - 6,
        y: laneY(top, v),
        text: trimNum(v),
        fill: style.tick,
        font: style.font,
        align: "right",
        baseline: "middle",
      });
    }
  }

  ops.push(
    ...xAxisOps(
      layout,
      style,
      {
        axisPx: viewport.px,
        min: viewport.start,
        max: viewport.end,
        unit: model.axisUnit,
      },
      xMapper(layout, viewport),
    ),
  );

  if (model.overlays !== undefined) {
    ops.push(...overlayOps(layout, style, viewport, model.overlays));
  }
  return ops;
}

function mergeGap(ranges: [number, number][], a: number, b: number): void {
  const last = ranges[ranges.length - 1];
  if (last !== undefined && a <= last[1] + 1e-12) {
    last[1] = Math.max(last[1], b);
  } else {
    ranges.push([a, b]);
  }
}

function trimNum(v: number): string {
  const t = v.toFixed(2);
  return t.includes(".") ? t.replace(/\.?0+$/, "") : t;
}
