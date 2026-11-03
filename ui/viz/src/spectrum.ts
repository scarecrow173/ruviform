/**
 * Spectrum scene builder (Issue #260).
 *
 * Maps a magnitude snapshot (f64 bins from the `SpectrumDataSource`
 * port) onto a dB × Hz plot: curve stroke, under-curve fill, frequency
 * axis (linear or log — display mapping only), dB axis, and explicit
 * gap ticks for missing bins. `NaN` bins break the curve; they are
 * never drawn as level.
 */

import type { VizLayout } from "./layout.ts";
import type { DrawOp } from "./ops.ts";
import {
  frameOps,
  gapTickOps,
  lossBadgeOps,
  staleOps,
  xAxisOps,
  yAxisOps,
} from "./scene.ts";
import { binAxisHz, type BinAxis } from "./source.ts";
import type { ResolvedVizStyle } from "./style.ts";

const P = "plot" as const;

export interface SpectrumModel {
  /** f64 magnitudes, bin order; `NaN` = missing bin. */
  readonly magnitudes: ArrayLike<number>;
  readonly axis: BinAxis;
  /** Frequency window + scale (display mapping). */
  readonly freq: {
    readonly scale: "linear" | "log";
    readonly minHz: number;
    readonly maxHz: number;
  };
  /** dB window + full-scale reference (display mapping). */
  readonly db: {
    readonly minDb: number;
    readonly maxDb: number;
    readonly reference: number;
  };
  /** Whole-view stale flag (snapshot aged out — same contract as
   * `MeterModel.stale`). */
  readonly stale?: boolean;
  /** Frames lost since the previous snapshot (element bookkeeping). */
  readonly dropped?: number;
}

export function spectrumScene(
  model: SpectrumModel,
  layout: VizLayout,
  style: ResolvedVizStyle,
): DrawOp[] {
  const ops: DrawOp[] = [...frameOps(layout, style)];
  const { freq, db } = model;
  const log = freq.scale === "log";
  const ratio = freq.maxHz / freq.minHz;
  const unitHz = (hz: number): number =>
    log
      ? Math.log(hz / freq.minHz) / Math.log(ratio)
      : (hz - freq.minHz) / (freq.maxHz - freq.minHz);
  const xOf = (hz: number): number =>
    layout.plot.x + unitHz(hz) * layout.plot.w;
  const yOfDb = (dbv: number): number =>
    layout.plot.y +
    layout.plot.h -
    Math.min(1, Math.max(0, (dbv - db.minDb) / (db.maxDb - db.minDb))) *
      layout.plot.h;
  const yFloor = layout.plot.y + layout.plot.h;
  const dbOf = (mag: number): number => 20 * Math.log10(mag / db.reference);

  // Axes first so the curve draws over the grid.
  ops.push(
    ...xAxisOps(
      layout,
      style,
      {
        axisPx: layout.plot.w,
        min: freq.minHz,
        max: freq.maxHz,
        unit: "hz",
        scale: freq.scale,
        minPxPerTick: 64,
      },
      xOf,
    ),
    ...yAxisOps(
      layout,
      style,
      {
        axisPx: layout.plot.h,
        min: db.minDb,
        max: db.maxDb,
        unit: "db",
        minPxPerTick: 26,
      },
      yOfDb,
    ),
  );

  const n = model.magnitudes.length;
  const gapXs: number[] = [];
  let xs: number[] = [];
  let line: number[] = [];
  const flush = () => {
    if (line.length >= 4) {
      ops.push(
        {
          kind: "band",
          layer: P,
          span: { xs },
          top: line.filter((_, i) => i % 2 === 1),
          bottom: new Array(line.length / 2).fill(yFloor),
          fill: style.spectrum.fill,
        },
        { kind: "line", layer: P, pts: line, stroke: style.spectrum.stroke, width: 1.5 },
      );
    } else if (line.length === 2) {
      // Lone finite bin — a stem so it stays visible.
      const x = line[0] ?? 0;
      const y = line[1] ?? 0;
      ops.push({
        kind: "line",
        layer: P,
        pts: [x, yFloor, x, y],
        stroke: style.spectrum.stroke,
        width: 1,
      });
    }
    xs = [];
    line = [];
  };

  for (let b = 0; b < n; b++) {
    const mag = model.magnitudes[b] ?? Number.NaN;
    const hz = binAxisHz(model.axis, b);
    const inRange = Number.isFinite(hz) && hz >= freq.minHz && hz < freq.maxHz;
    const x = inRange ? xOf(hz) : Number.NaN;
    const dbv = dbOf(mag);
    if (!inRange || !Number.isFinite(x) || !Number.isFinite(dbv)) {
      flush();
      // Missing in-window bins get the point-wise gap cue on the axis.
      if (inRange && Number.isFinite(x) && !Number.isFinite(dbv)) {
        gapXs.push(x);
      }
      continue;
    }
    line.push(x, yOfDb(dbv));
    xs.push(x);
  }
  flush();
  if (gapXs.length > 0) ops.push(...gapTickOps(layout.plot, style, gapXs));
  if (model.stale === true) ops.push(...staleOps(layout, style));
  ops.push(...lossBadgeOps(layout, style, model.dropped ?? 0));
  return ops;
}
