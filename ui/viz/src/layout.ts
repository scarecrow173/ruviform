/**
 * Drawing-surface layout for the Reference Viz renderers.
 *
 * The layout carves the canvas into a left tick-label gutter, a bottom
 * tick-label strip, and the plot rect the data draws into. Gutters are
 * derived from the resolved tick size so geometry tracks the font
 * tokens — but a font/label change is a *layout* invalidation, not a
 * dataset re-query (§12.5).
 */

import type { ResolvedVizStyle } from "./style.ts";

export interface Rect {
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
}

export interface VizLayout {
  /** Full drawing surface in CSS px. */
  readonly width: number;
  readonly height: number;
  /** Data drawing area. */
  readonly plot: Rect;
  readonly leftGutter: number;
  readonly bottomGutter: number;
}

export interface LayoutInput {
  readonly width: number;
  readonly height: number;
  /** Reserve the left gutter for Y tick labels. */
  readonly yAxis?: boolean;
  /** Reserve the bottom strip for X tick labels. */
  readonly xAxis?: boolean;
  readonly style: ResolvedVizStyle;
}

export function computeLayout(input: LayoutInput): VizLayout {
  const top = 4;
  const right = 8;
  const left = input.yAxis === true ? Math.ceil(input.style.tickSize * 4 + 10) : 8;
  const bottom =
    input.xAxis === true ? Math.ceil(input.style.tickSize + 10) : 8;
  const w = Math.max(0, input.width - left - right);
  const h = Math.max(0, input.height - top - bottom);
  return {
    width: input.width,
    height: input.height,
    plot: { x: left, y: top, w, h },
    leftGutter: left,
    bottomGutter: bottom,
  };
}

/** Split a lane rect into `count` channel lanes with `gapPx` between
 * them (top→bottom order). Returns one rect per channel. */
export function channelLanes(
  area: Rect,
  count: number,
  gapPx: number,
  orientation: "vertical" | "horizontal" = "vertical",
): Rect[] {
  const n = Math.max(1, count);
  const out: Rect[] = [];
  if (orientation === "horizontal") {
    const gap = Math.min(gapPx, count > 1 ? area.w / (count * 4) : 0);
    const w = (area.w - gap * (n - 1)) / n;
    for (let i = 0; i < n; i++) {
      out.push({ x: area.x + i * (w + gap), y: area.y, w, h: area.h });
    }
  } else {
    const gap = Math.min(gapPx, count > 1 ? area.h / (count * 4) : 0);
    const h = (area.h - gap * (n - 1)) / n;
    for (let i = 0; i < n; i++) {
      out.push({ x: area.x, y: area.y + i * (h + gap), w: area.w, h });
    }
  }
  return out;
}
