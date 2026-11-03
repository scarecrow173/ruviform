/**
 * Axis tick selection and label formatting (display boundary only).
 *
 * Tick math is a *display* mapping — it never feeds back into dataset
 * queries. `niceStep` produces 1/2/5·10ⁿ steps; `logTicks` produces
 * decade + 2–9 sub-ticks for frequency axes. Units the elements declare
 * (`seconds`, `samples`, `frames`, `hz`, `db`, `unit`) only drive label
 * text, not geometry.
 */

export type AxisUnit = "seconds" | "samples" | "frames" | "hz" | "db" | "unit";

export interface Tick {
  readonly value: number;
  /** Decade boundary / rounded step — rendered with a label. */
  readonly major: boolean;
}

/** Smallest 1/2/5·10ⁿ step ≥ `raw`. Non-positive/degenerate input → 1. */
export function niceStep(raw: number): number {
  if (!(raw > 0) || !Number.isFinite(raw)) return 1;
  const exp = Math.floor(Math.log10(raw));
  const base = 10 ** exp;
  const f = raw / base;
  if (f <= 1) return base;
  if (f <= 2) return 2 * base;
  if (f <= 5) return 5 * base;
  return 10 * base;
}

/**
 * Linear ticks covering `[min, max)` spaced so neighboring majors are
 * at least `minPxPerTick` device-independent px apart (via `unitsPerPx`).
 * The first tick lands on the first multiple of the step ≥ `min`.
 */
export function linearTicks(
  min: number,
  max: number,
  unitsPerPx: number,
  minPxPerTick = 56,
): readonly Tick[] {
  if (
    !(max > min) ||
    !(unitsPerPx > 0) ||
    !Number.isFinite(unitsPerPx) ||
    !Number.isFinite(min) ||
    !Number.isFinite(max)
  ) {
    return [];
  }
  const step = niceStep(unitsPerPx * minPxPerTick);
  if (!(step > 0) || !Number.isFinite(step)) return [];
  const out: Tick[] = [];
  const first = Math.ceil(min / step - 1e-9) * step;
  // Cap iteration: a pathological step/span ratio must not hang the
  // draw loop (ticks beyond ~64 per axis are unreadable anyway).
  const maxTicks = 4096;
  let i = 0;
  for (let v = first; v < max && i < maxTicks; v += step, i++) {
    // Re-anchor through the step count to avoid f64 drift accumulation.
    const k = Math.round(v / step);
    const t = k * step + 0; // normalize -0 → 0 at the range edge
    if (t >= min && t < max) out.push({ value: t, major: true });
  }
  return out;
}

/**
 * Decade/sub-decade ticks over `[minHz, maxHz)` (`minHz > 0` required —
 * callers clamp a linear axis's 0 edge before asking for log ticks).
 * Decades are major; 2–9·10ᵏ products are minor (no label).
 */
export function logTicks(minHz: number, maxHz: number): readonly Tick[] {
  if (!(maxHz > minHz) || !(minHz > 0)) return [];
  const out: Tick[] = [];
  const lo = Math.floor(Math.log10(minHz));
  const hi = Math.ceil(Math.log10(maxHz));
  for (let e = lo; e <= hi; e++) {
    const base = 10 ** e;
    for (let m = 1; m <= 9; m++) {
      const v = m * base;
      if (v >= minHz && v < maxHz) out.push({ value: v, major: m === 1 });
    }
  }
  return out;
}

/**
 * Compact axis label for `v` in `unit`. `step` (the tick spacing)
 * adapts decimals for sub-second / fractional units so adjacent labels
 * stay distinguishable. dB and sample/frame indices render as integers.
 */
export function formatTick(v: number, unit: AxisUnit, step?: number): string {
  if (!Number.isFinite(v)) return "—";
  const s = step ?? (Math.abs(v) || 1);
  switch (unit) {
    case "seconds": {
      if (Math.abs(v) < 1e-12) return "0";
      if (s <= 0.001) return `${trim(v * 1000, 1)}ms`;
      if (s < 1) return `${trim(v, 2)}s`;
      if (s < 60) return `${trim(v, 1)}s`;
      const m = Math.floor(v / 60);
      const sec = v - m * 60;
      return `${m}:${sec < 9.5 ? "0" : ""}${trim(sec, 0)}`;
    }
    case "samples":
    case "frames":
      return String(Math.round(v));
    case "hz":
      if (Math.abs(v) >= 1000) return `${trim(v / 1000, 1)}k`;
      return `${trim(v, 0)}`;
    case "db":
      return `${trim(v, 0)}`;
    case "unit":
      if (s < 0.01) return trim(v, 3);
      if (s < 0.1) return trim(v, 2);
      if (s < 1) return trim(v, 1);
      return trim(v, 0);
  }
}

function trim(v: number, digits: number): string {
  const t = v.toFixed(digits);
  return t.includes(".") ? t.replace(/\.?0+$/, "") : t;
}
