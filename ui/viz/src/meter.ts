/**
 * Level-meter scene builder (Issue #260).
 *
 * Renders a snapshot `MeterModel` — peak bar (three paint segments),
 * RMS line, held-peak tick, clip indicator, per-channel availability,
 * and the stale/loss overlays the data contract requires. All mapping
 * is display-side (`scale` → px); snapshot values are never modified.
 *
 * - `stale`/`dropped` are element bookkeeping (sequence gap + snapshot
 *   age) passed through the model — the scene only renders them.
 * - An unavailable channel (`NaN` peak) gets a hatched lane — distinct
 *   from stale and never drawn as a live level.
 * - Clip shows when the source's `clip` flag is set OR the peak reaches
 *   `scale.clipDb` — the flag is authoritative, the level check is the
 *   display fallback for flag-less sources.
 */

import type { VizLayout } from "./layout.ts";
import { channelLanes, type Rect } from "./layout.ts";
import type { DrawOp } from "./ops.ts";
import {
  frameOps,
  lossBadgeOps,
  staleOps,
  xAxisOps,
  yAxisOps,
} from "./scene.ts";
import type { MeterChannel } from "./source.ts";
import type { ResolvedVizStyle } from "./style.ts";

const P = "plot" as const;
const O = "overlay" as const;

export interface MeterScale {
  /** dB at the bar's bottom/top (display mapping, not data). */
  readonly minDb: number;
  readonly maxDb: number;
  /** dB boundaries between the low/mid/high paint segments. */
  readonly segments?: readonly number[];
  /** dB at-or-above which the clip indicator lights (default 0). */
  readonly clipDb?: number;
}

export interface MeterModel {
  readonly channels: readonly MeterChannel[];
  readonly scale: MeterScale;
  /** Whole-view stale flag (snapshot aged out / revisions raced). */
  readonly stale: boolean;
  /** Frames lost since the previous snapshot (element bookkeeping). */
  readonly dropped: number;
  /** Bars grow up (`vertical`, channels side-by-side) or right
   * (`horizontal`, channels stacked). */
  readonly orientation: "vertical" | "horizontal";
}

export const DEFAULT_METER_SCALE: MeterScale = {
  minDb: -60,
  maxDb: 3,
  segments: [-18, -6],
  clipDb: 0,
};

export function dbOf(amplitude: number): number {
  return 20 * Math.log10(amplitude);
}

export function meterScene(
  model: MeterModel,
  layout: VizLayout,
  style: ResolvedVizStyle,
): DrawOp[] {
  const ops: DrawOp[] = [...frameOps(layout, style)];
  const { scale } = model;
  const spanDb = scale.maxDb - scale.minDb;
  // NaN propagates as NaN (missing); ±Inf clamps to the rail —
  // amplitude 0 → −Inf dB is *silence* (empty bar), not missing data.
  const unit = (db: number): number =>
    Number.isNaN(db) || !Number.isFinite(spanDb)
      ? Number.NaN
      : Math.min(1, Math.max(0, (db - scale.minDb) / spanDb));
  const clipDb = scale.clipDb ?? 0;
  const segments = [...(scale.segments ?? [-18, -6])].sort((a, b) => a - b);
  const segColors = [style.meter.low, style.meter.mid, style.meter.high];

  const vertical = model.orientation === "vertical";
  // "vertical" bars grow up → channels sit side-by-side (columns);
  // "horizontal" bars grow right → channels stack (rows).
  const lanes = channelLanes(
    layout.plot,
    Math.max(1, model.channels.length),
    style.meter.channelGapPx,
    vertical ? "horizontal" : "vertical",
  );

  // dB axis ticks on the cross axis (left gutter / bottom strip).
  const levelOf = (u: number): number => scale.minDb + u * spanDb;
  if (vertical) {
    ops.push(
      ...yAxisOps(
        layout,
        style,
        {
          axisPx: layout.plot.h,
          min: scale.minDb,
          max: scale.maxDb,
          unit: "db",
          minPxPerTick: 26,
        },
        (db) => layout.plot.y + layout.plot.h - unit(db) * layout.plot.h,
      ),
    );
  } else {
    ops.push(
      ...xAxisOps(
        layout,
        style,
        {
          axisPx: layout.plot.w,
          min: scale.minDb,
          max: scale.maxDb,
          unit: "db",
          minPxPerTick: 40,
        },
        (db) => layout.plot.x + unit(db) * layout.plot.w,
      ),
    );
  }

  // Segment boundaries as faint guides across the plot.
  for (const s of segments) {
    if (s <= scale.minDb || s >= scale.maxDb) continue;
    const u = unit(s);
    if (vertical) {
      const y = layout.plot.y + layout.plot.h - u * layout.plot.h;
      ops.push({
        kind: "line",
        layer: P,
        pts: [layout.plot.x, y, layout.plot.x + layout.plot.w, y],
        stroke: style.axis,
        width: Math.max(0.5, style.gridWidth / 2),
      });
    } else {
      const x = layout.plot.x + u * layout.plot.w;
      ops.push({
        kind: "line",
        layer: P,
        pts: [x, layout.plot.y, x, layout.plot.y + layout.plot.h],
        stroke: style.axis,
        width: Math.max(0.5, style.gridWidth / 2),
      });
    }
  }

  for (let c = 0; c < model.channels.length; c++) {
    const lane = lanes[c];
    const ch = model.channels[c];
    if (lane === undefined || ch === undefined) continue;
    ops.push({
      kind: "rect",
      layer: P,
      x: lane.x,
      y: lane.y,
      w: lane.w,
      h: lane.h,
      fill: style.meter.track,
    });

    // Availability is a *raw-amplitude* property: NaN/±Inf means the
    // channel has no reading; finite values (including 0 → −Inf dB
    // silence) render normally.
    const peakDb = Number.isFinite(ch.peak) ? dbOf(ch.peak) : Number.NaN;
    const rmsDb = Number.isFinite(ch.rms) ? dbOf(ch.rms) : Number.NaN;
    if (Number.isNaN(peakDb) && Number.isNaN(rmsDb)) {
      // Unavailable channel — hatched lane, no fabricated level.
      ops.push({
        kind: "hatch",
        layer: P,
        x: lane.x,
        y: lane.y,
        w: lane.w,
        h: lane.h,
        stroke: style.state.gap,
        width: 1,
        spacing: 6,
        angle: Math.PI / 4,
      });
      continue;
    }

    const pu = unit(peakDb);
    if (!Number.isNaN(pu)) {
      // Fill painted in up to three level segments.
      const bounds = [
        scale.minDb,
        ...segments.filter((s) => s > scale.minDb && s < levelOf(pu)),
        levelOf(pu),
      ];
      for (let s = 0; s < bounds.length - 1; s++) {
        const u0 = unit(bounds[s] ?? scale.minDb);
        const u1 = unit(bounds[s + 1] ?? scale.minDb);
        const fill =
          segColors[Math.min(s, segColors.length - 1)] ?? style.meter.low;
        ops.push(barOp(lane, u0, u1, vertical, fill));
      }
    }

    // RMS line inside the bar (−Inf clamps to the bottom rail, NaN
    // draws nothing).
    if (!Number.isNaN(rmsDb)) {
      const ru = unit(rmsDb);
      if (!Number.isNaN(ru)) ops.push(rmsOp(lane, ru, vertical, style.meter.peak));
    }

    // Held-peak tick.
    if (ch.heldPeak !== undefined && Number.isFinite(dbOf(ch.heldPeak))) {
      ops.push(
        holdOp(lane, unit(dbOf(ch.heldPeak)), vertical, style.meter.peak),
      );
    }

    // Clip indicator.
    if (ch.clip === true || peakDb >= clipDb) {
      ops.push(clipOp(lane, vertical, style.meter.clip));
    }
  }

  if (model.stale) ops.push(...staleOps(layout, style));
  ops.push(...lossBadgeOps(layout, style, model.dropped));
  return ops;
}

/** Fill the `[u0,u1]` fraction of a lane (measured along the level axis). */
function barOp(
  lane: Rect,
  u0: number,
  u1: number,
  vertical: boolean,
  fill: string,
): DrawOp {
  if (vertical) {
    const y = lane.y + lane.h - u1 * lane.h;
    return {
      kind: "rect",
      layer: P,
      x: lane.x,
      y,
      w: lane.w,
      h: Math.max(0, (u1 - u0) * lane.h),
      fill,
    };
  }
  return {
    kind: "rect",
    layer: P,
    x: lane.x + u0 * lane.w,
    y: lane.y,
    w: Math.max(0, (u1 - u0) * lane.w),
    h: lane.h,
    fill,
  };
}

function rmsOp(lane: Rect, u: number, vertical: boolean, color: string): DrawOp {
  if (vertical) {
    const y = lane.y + lane.h - u * lane.h;
    return {
      kind: "rect",
      layer: P,
      x: lane.x + 1,
      y: y - 1,
      w: Math.max(0, lane.w - 2),
      h: 2,
      fill: color,
    };
  }
  const x = lane.x + u * lane.w;
  return {
    kind: "rect",
    layer: P,
    x: x - 1,
    y: lane.y + 1,
    w: 2,
    h: Math.max(0, lane.h - 2),
    fill: color,
  };
}

function holdOp(lane: Rect, u: number, vertical: boolean, color: string): DrawOp {
  if (vertical) {
    const y = lane.y + lane.h - u * lane.h;
    return {
      kind: "rect",
      layer: P,
      x: lane.x,
      y: y - 1,
      w: lane.w,
      h: 1.5,
      fill: color,
    };
  }
  const x = lane.x + u * lane.w;
  return {
    kind: "rect",
    layer: P,
    x: x - 1,
    y: lane.y,
    w: 1.5,
    h: lane.h,
    fill: color,
  };
}

function clipOp(lane: Rect, vertical: boolean, color: string): DrawOp {
  const s = Math.min(8, Math.max(4, (vertical ? lane.w : lane.h) * 0.4));
  if (vertical) {
    return {
      kind: "rect",
      layer: O,
      x: lane.x,
      y: lane.y,
      w: lane.w,
      h: s,
      fill: color,
    };
  }
  return {
    kind: "rect",
    layer: O,
    x: lane.x + lane.w - s,
    y: lane.y,
    w: s,
    h: lane.h,
    fill: color,
  };
}
