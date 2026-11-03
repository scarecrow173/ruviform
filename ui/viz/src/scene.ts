/**
 * Shared scene-op builders — the reusable "non-data" visual elements
 * of the Reference Viz skin (frame, grid, axes, gaps, selection,
 * playhead, markers, stale badge, legend, unsupported cue).
 *
 * Every builder emits DrawOps only; which values mark a gap or a stale
 * state is the caller's model decision, never the scene's.
 */

import {
  formatTick,
  linearTicks,
  logTicks,
  type AxisUnit,
  type Tick,
} from "./axis.ts";
import type { Rect, VizLayout } from "./layout.ts";
import type { DrawOp } from "./ops.ts";
import type { Viewport } from "./viewport.ts";
import { vpToPx, vpUnitsPerPx } from "./viewport.ts";
import type { ResolvedVizStyle } from "./style.ts";

const P = "plot" as const;
const O = "overlay" as const;

/** Surface + plot background + outer border. */
export function frameOps(
  layout: VizLayout,
  style: ResolvedVizStyle,
): DrawOp[] {
  return [
    {
      kind: "rect",
      layer: P,
      x: layout.plot.x,
      y: layout.plot.y,
      w: layout.plot.w,
      h: layout.plot.h,
      fill: style.background,
    },
    {
      kind: "rectStroke",
      layer: P,
      x: layout.plot.x,
      y: layout.plot.y,
      w: layout.plot.w,
      h: layout.plot.h,
      stroke: style.axis,
      width: 1,
    },
  ];
}

export interface AxisOpsInput {
  /** Pixel span the axis covers on screen. */
  readonly axisPx: number;
  /** Value at pixel 0 and at `axisPx`. */
  readonly min: number;
  readonly max: number;
  readonly unit: AxisUnit;
  /** `"log"` switches tick selection to decade/sub-decade (values > 0). */
  readonly scale?: "linear" | "log";
  /** Minimum px between labeled ticks. */
  readonly minPxPerTick?: number;
}

export function axisTicks(input: AxisOpsInput): readonly Tick[] {
  // Degenerate pixel spans produce no axis — without this gate a 0 px
  // plot emits (max−min)/step ticks per frame.
  if (!(input.axisPx > 0) || !Number.isFinite(input.axisPx)) return [];
  const upp = (input.max - input.min) / input.axisPx;
  if (input.scale === "log") return logTicks(input.min, input.max);
  return linearTicks(input.min, input.max, upp, input.minPxPerTick ?? 56);
}

function tickStep(ticks: readonly Tick[]): number | undefined {
  const majors = ticks.filter((t) => t.major);
  if (majors.length < 2) return undefined;
  const a = majors[0]?.value;
  const b = majors[1]?.value;
  return a !== undefined && b !== undefined ? b - a : undefined;
}

/**
 * X axis: vertical grid lines + bottom-edge labels. `xOf` maps an axis
 * value to a canvas x (pixel space inside `layout`, already including
 * the gutter offset).
 */
export function xAxisOps(
  layout: VizLayout,
  style: ResolvedVizStyle,
  input: AxisOpsInput,
  xOf: (v: number) => number,
): DrawOp[] {
  const ops: DrawOp[] = [];
  const ticks = axisTicks(input);
  const step = tickStep(ticks);
  const labelY = layout.plot.y + layout.plot.h + 4;
  for (const t of ticks) {
    const x = xOf(t.value);
    if (x < layout.plot.x - 0.5 || x > layout.plot.x + layout.plot.w + 0.5) {
      continue;
    }
    ops.push({
      kind: "line",
      layer: P,
      pts: [x, layout.plot.y, x, layout.plot.y + layout.plot.h],
      stroke: t.major ? style.grid : style.grid,
      width: t.major ? style.gridWidth : Math.max(0.5, style.gridWidth / 2),
    });
    if (t.major) {
      ops.push({
        kind: "text",
        layer: O,
        x,
        y: labelY,
        text: formatTick(t.value, input.unit, step),
        fill: style.tick,
        font: style.font,
        align: "center",
        baseline: "top",
      });
    }
  }
  return ops;
}

/** Y axis: horizontal grid lines + left-gutter labels. `yOf` maps a
 * value to a canvas y. */
export function yAxisOps(
  layout: VizLayout,
  style: ResolvedVizStyle,
  input: AxisOpsInput,
  yOf: (v: number) => number,
): DrawOp[] {
  const ops: DrawOp[] = [];
  const ticks = axisTicks(input);
  const step = tickStep(ticks);
  const labelX = layout.plot.x - 6;
  for (const t of ticks) {
    const y = yOf(t.value);
    if (y < layout.plot.y - 0.5 || y > layout.plot.y + layout.plot.h + 0.5) {
      continue;
    }
    ops.push({
      kind: "line",
      layer: P,
      pts: [layout.plot.x, y, layout.plot.x + layout.plot.w, y],
      stroke: style.grid,
      width: t.major ? style.gridWidth : Math.max(0.5, style.gridWidth / 2),
    });
    if (t.major) {
      ops.push({
        kind: "text",
        layer: O,
        x: labelX,
        y,
        text: formatTick(t.value, input.unit, step),
        fill: style.tick,
        font: style.font,
        align: "right",
        baseline: "middle",
      });
    }
  }
  return ops;
}

/**
 * Missing-data regions on the X axis: a subtle wash + diagonal hatch —
 * the explicit non-color gap cue. `ranges` are axis-unit spans already
 * intersected with the viewport.
 */
export function gapOps(
  area: Rect,
  style: ResolvedVizStyle,
  viewport: Viewport,
  x0: number,
  ranges: readonly (readonly [number, number])[],
): DrawOp[] {
  const ops: DrawOp[] = [];
  for (const [a, b] of ranges) {
    const rx0 = x0 + vpToPx(viewport, a);
    const rx1 = x0 + vpToPx(viewport, b);
    if (rx1 <= rx0) continue;
    ops.push(
      {
        kind: "rect",
        layer: P,
        x: rx0,
        y: area.y,
        w: rx1 - rx0,
        h: area.h,
        fill: style.state.gapFill,
      },
      {
        kind: "hatch",
        layer: P,
        x: rx0,
        y: area.y,
        w: rx1 - rx0,
        h: area.h,
        stroke: style.state.gap,
        width: 1,
        spacing: 6,
        angle: Math.PI / 4,
      },
    );
  }
  return ops;
}

/** Small tick marks on the rect's bottom edge — the point-wise gap cue
 * for series without a region semantic (spectrum bins). */
export function gapTickOps(
  area: Rect,
  style: ResolvedVizStyle,
  xs: readonly number[],
): DrawOp[] {
  const y0 = area.y + area.h;
  return xs.map((x) => ({
    kind: "line" as const,
    layer: P,
    pts: [x, y0 - 5, x, y0],
    stroke: style.state.gap,
    width: 1.5,
  }));
}

export interface OverlayState {
  readonly selection?: { readonly start: number; readonly end: number };
  readonly playhead?: number;
  readonly markers?: readonly {
    readonly at: number;
    readonly label?: string;
    readonly color?: string;
  }[];
}

/** Selection band, playhead line, and marker lines+labels — positions
 * only; marker meaning stays with the consumer. */
export function overlayOps(
  layout: VizLayout,
  style: ResolvedVizStyle,
  viewport: Viewport,
  overlays: OverlayState,
): DrawOp[] {
  const ops: DrawOp[] = [];
  const px0 = layout.plot.x;
  const { selection, playhead, markers } = overlays;
  if (selection !== undefined) {
    // Clamp to the plot rect — a selection extending past the viewport
    // must not paint over gutters, labels, or the status corner.
    const px1 = layout.plot.x + layout.plot.w;
    const x0 = Math.min(
      Math.max(px0 + vpToPx(viewport, Math.min(selection.start, selection.end)), px0),
      px1,
    );
    const x1 = Math.min(
      Math.max(px0 + vpToPx(viewport, Math.max(selection.start, selection.end)), px0),
      px1,
    );
    if (x1 > x0) {
      ops.push(
        {
          kind: "rect",
          layer: O,
          x: x0,
          y: layout.plot.y,
          w: x1 - x0,
          h: layout.plot.h,
          fill: style.overlay.selectionFill,
        },
        {
          kind: "line",
          layer: O,
          pts: [x0, layout.plot.y, x0, layout.plot.y + layout.plot.h],
          stroke: style.overlay.selectionStroke,
          width: 1,
        },
        {
          kind: "line",
          layer: O,
          pts: [x1, layout.plot.y, x1, layout.plot.y + layout.plot.h],
          stroke: style.overlay.selectionStroke,
          width: 1,
        },
      );
    }
  }
  if (playhead !== undefined && Number.isFinite(playhead)) {
    const x = px0 + vpToPx(viewport, playhead);
    if (x >= layout.plot.x - 0.5 && x <= layout.plot.x + layout.plot.w + 0.5) {
      ops.push({
        kind: "line",
        layer: O,
        pts: [x, layout.plot.y, x, layout.plot.y + layout.plot.h],
        stroke: style.overlay.playhead,
        width: 1.5,
      });
    }
  }
  for (const m of markers ?? []) {
    if (!Number.isFinite(m.at)) continue;
    const x = px0 + vpToPx(viewport, m.at);
    if (x < layout.plot.x - 0.5 || x > layout.plot.x + layout.plot.w + 0.5) {
      continue;
    }
    const c = m.color ?? style.overlay.marker;
    ops.push({
      kind: "line",
      layer: O,
      pts: [x, layout.plot.y, x, layout.plot.y + layout.plot.h],
      stroke: c,
      width: 1,
    });
    if (m.label !== undefined && m.label !== "") {
      ops.push({
        kind: "text",
        layer: O,
        x: x + 3,
        y: layout.plot.y + 2,
        text: m.label,
        fill: c,
        font: style.font,
        align: "left",
        baseline: "top",
      });
    }
  }
  return ops;
}

/** Stale-data cue: hatch across the plot + a text badge. Never faked
 * as live data (§12.6 non-color channel). */
export function staleOps(
  layout: VizLayout,
  style: ResolvedVizStyle,
  label = "stale",
): DrawOp[] {
  return [
    {
      kind: "hatch",
      layer: O,
      x: layout.plot.x,
      y: layout.plot.y,
      w: layout.plot.w,
      h: layout.plot.h,
      stroke: style.state.stale,
      width: 1,
      spacing: 8,
      angle: -Math.PI / 4,
    },
    {
      kind: "text",
      layer: O,
      x: layout.plot.x + layout.plot.w - 6,
      y: layout.plot.y + 4,
      text: label,
      fill: style.state.stale,
      font: style.font,
      align: "right",
      baseline: "top",
    },
  ];
}

/** A small right-aligned loss counter ("−3" frames dropped). */
export function lossBadgeOps(
  layout: VizLayout,
  style: ResolvedVizStyle,
  dropped: number,
): DrawOp[] {
  if (dropped <= 0) return [];
  return [
    {
      kind: "text",
      layer: O,
      x: layout.plot.x + layout.plot.w - 6,
      y: layout.plot.y + layout.plot.h - 4,
      text: `−${dropped}`,
      fill: style.state.stale,
      font: style.font,
      align: "right",
      baseline: "bottom",
    },
  ];
}

/** Centered unsupported-capability cue — the visible fallback when a
 * backend cannot render at all. */
export function unsupportedOps(
  layout: VizLayout,
  style: ResolvedVizStyle,
  reason: string,
): DrawOp[] {
  const cx = layout.plot.x + layout.plot.w / 2;
  const cy = layout.plot.y + layout.plot.h / 2;
  return [
    {
      kind: "rectStroke",
      layer: O,
      x: layout.plot.x,
      y: layout.plot.y,
      w: layout.plot.w,
      h: layout.plot.h,
      stroke: style.state.unsupported,
      width: 2,
    },
    {
      kind: "text",
      layer: O,
      x: cx,
      y: cy,
      text: `unsupported: ${reason}`,
      fill: style.state.unsupported,
      font: style.font,
      align: "center",
      baseline: "middle",
    },
  ];
}

/** Top-right legend: color swatch + label per entry. */
export function legendOps(
  layout: VizLayout,
  style: ResolvedVizStyle,
  entries: readonly { readonly color: string; readonly label: string }[],
): DrawOp[] {
  const ops: DrawOp[] = [];
  let x = layout.plot.x + layout.plot.w - 6;
  const y = layout.plot.y + 4;
  for (const e of [...entries].reverse()) {
    ops.push(
      {
        kind: "text",
        layer: O,
        x,
        y,
        text: e.label,
        fill: style.tick,
        font: style.font,
        align: "right",
        baseline: "top",
      },
      {
        kind: "rect",
        layer: O,
        x: x - e.label.length * style.tickSize * 0.6 - 12,
        y: y + 2,
        w: 8,
        h: 8,
        fill: e.color,
      },
    );
    x -= e.label.length * style.tickSize * 0.6 + 20;
  }
  return ops;
}

/** Convenience: viewport span → px width and a value→x mapper over the
 * plot rect. */
export function xMapper(
  layout: VizLayout,
  viewport: Viewport,
): (v: number) => number {
  const px0 = layout.plot.x;
  return (v) => px0 + vpToPx(viewport, v);
}

export { vpUnitsPerPx };
