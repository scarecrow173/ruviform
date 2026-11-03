/**
 * Viewport — a visible axis span mapped to pixels.
 *
 * Mirrors `crates/ruviform-viz/src/timeline.rs` `Viewport` semantics:
 * the axis unit is caller-chosen (seconds, samples, frames); this module
 * only maps positions. `start` is inclusive, `end` exclusive, `px > 0`.
 */

export interface Viewport {
  /** Axis position at the near edge (inclusive). */
  readonly start: number;
  /** Axis position at the far edge (exclusive). */
  readonly end: number;
  /** Pixels the span maps to (`> 0`). */
  readonly px: number;
}

/** Creates a viewport; returns `undefined` on `end <= start`, non-finite
 * endpoints, or `px <= 0` (same rejection contract as the Rust `new`). */
export function viewport(
  start: number,
  end: number,
  px: number,
): Viewport | undefined {
  if (!(end > start) || !Number.isFinite(start) || !Number.isFinite(end)) {
    return undefined;
  }
  if (!(px > 0)) return undefined;
  return { start, end, px };
}

/** `end - start` axis units covered. */
export function vpSpan(v: Viewport): number {
  return v.end - v.start;
}

/** Axis units per pixel. */
export function vpUnitsPerPx(v: Viewport): number {
  return vpSpan(v) / v.px;
}

/** Fractional pixel position of `u` (0 = near edge; may fall outside
 * `[0, px)` when `u` is outside the span). `NaN` propagates. */
export function vpToPx(v: Viewport, u: number): number {
  return (u - v.start) / vpUnitsPerPx(v);
}

/** Axis position at the center of pixel `p` (`0..px`). */
export function vpToUnit(v: Viewport, p: number): number {
  return v.start + (p + 0.5) * vpUnitsPerPx(v);
}

/** A viewport zoomed by `factor` around `anchor` (the anchor's pixel
 * position is preserved). `factor > 1` zooms in; `px` is carried over.
 * Returns `undefined` on non-positive or non-finite factor. */
export function vpZoom(
  v: Viewport,
  factor: number,
  anchor: number,
): Viewport | undefined {
  if (!(factor > 0) || !Number.isFinite(factor)) return undefined;
  const span = vpSpan(v) / factor;
  const t = (anchor - v.start) / vpSpan(v);
  return viewport(anchor - span * t, anchor + span * (1 - t), v.px);
}

/** A viewport panned by `dpx` pixels (positive = content moves right,
 * i.e. earlier axis positions become visible). */
export function vpPan(v: Viewport, dpx: number): Viewport {
  const d = dpx * vpUnitsPerPx(v);
  return { start: v.start - d, end: v.end - d, px: v.px };
}

/** Clamp a viewport to `[lo, hi)` axis bounds without changing its span
 * (clamps position, not zoom — a pan past the data edge slides back). */
export function vpClamp(v: Viewport, lo: number, hi: number): Viewport {
  const span = vpSpan(v);
  if (span >= hi - lo) return { start: lo, end: hi, px: v.px };
  const start = Math.min(Math.max(v.start, lo), hi - span);
  return { start, end: start + span, px: v.px };
}
