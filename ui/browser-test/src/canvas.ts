/**
 * Canvas2D / WebGL2 inspection (Issue #310).
 *
 * Semantic pixel probes — the harness asserts *what the canvas
 * contains* (fill coverage, explicit gap regions, backend identity),
 * never a golden image. Screenshot remains auxiliary evidence only.
 *
 * `fillProbe` reads the plot canvas through `getImageData` and
 * reports per-column painted ratios; `backendKind`/`stats` come from
 * the element's own published surface (`stats`, `backendKind`,
 * `rvf-backend` event), so a WebGL2-less environment degrades to a
 * recorded "capability unavailable" — evidence, not a failure.
 */

import { expect } from "@playwright/test";
import type { Locator } from "@playwright/test";
import type { HarnessSession } from "./harness.ts";
import type { ControlHandle } from "./control.ts";

/** Column-wise fill ratios for one canvas region. */
export interface FillProbeResult {
  /** Painted-pixel ratio per sampled column (0..1), left→right. */
  readonly columns: number[];
  /** Mean painted ratio across the sampled region (0..1). */
  readonly mean: number;
  /** Canvas pixel size actually read. */
  readonly width: number;
  readonly height: number;
  /**
   * `false` when the canvas has no readable 2D context (e.g. it is
   * bound to a WebGL backend — pixel probes are a Canvas2D path;
   * assert `lastOpCount`/`framesDrawn` there instead).
   */
  readonly readable: boolean;
}

/**
 * Read the element's plot canvas (`canvas.plot` inside the viz shadow)
 * and report the fraction of fully-painted rows per column within the
 * given normalized rect (`x0..x1`, `y0..y1` of the canvas box). A
 * pixel counts as painted when alpha > 0 — the plot surface starts
 * transparent, so coverage is signal, not background.
 */
export async function fillProbe(
  c: ControlHandle,
  region: { x0: number; x1: number; y0: number; y1: number },
  columns = 24,
): Promise<FillProbeResult> {
  const canvas = c.part("plot-canvas").first();
  const count = await canvas.count();
  if (count === 0)
    throw new Error(`${c.selector}: no part="plot-canvas" canvas`);
  return canvas.evaluate(
    (el, args) => {
      const cv = el as HTMLCanvasElement;
      const ctx = cv.getContext("2d") as CanvasRenderingContext2D | null;
      if (ctx === null) {
        return {
          columns: [],
          mean: 0,
          width: cv.width,
          height: cv.height,
          readable: false,
        };
      }
      const x0 = Math.floor(cv.width * args.x0);
      const x1 = Math.ceil(cv.width * args.x1);
      const y0 = Math.floor(cv.height * args.y0);
      const y1 = Math.ceil(cv.height * args.y1);
      const img = ctx.getImageData(x0, y0, x1 - x0, y1 - y0);
      const cols: number[] = [];
      const w = x1 - x0;
      for (let i = 0; i < args.columns; i++) {
        const cx0 = Math.floor((w * i) / args.columns);
        const cx1 = Math.floor((w * (i + 1)) / args.columns);
        let painted = 0;
        let total = 0;
        for (let x = cx0; x < cx1; x++) {
          for (let y = 0; y < img.height; y++) {
            total++;
            if (img.data[(y * img.width + x) * 4 + 3]! > 0) painted++;
          }
        }
        cols.push(total === 0 ? 0 : painted / total);
      }
      const mean = cols.reduce((a, b) => a + b, 0) / (cols.length || 1);
      return {
        columns: cols,
        mean,
        width: cv.width,
        height: cv.height,
        readable: true,
      };
    },
    { ...region, columns },
  );
}

/**
 * Parse a CSS `#rgb`/`#rrggbb`/`#rrggbbaa` color into `[r,g,b]`
 * (alpha ignored — the canvas composites; what we match is hue).
 * Returns `undefined` for non-hex values the probe can't compare.
 */
export function parseHexColor(css: string): [number, number, number] | undefined {
  const m = /^#([0-9a-f]{3,8})$/i.exec(css.trim());
  if (m === null) return undefined;
  let h = m[1]!;
  if (h.length === 3 || h.length === 4) {
    h = h
      .split("")
      .map((ch) => ch + ch)
      .join("");
  }
  if (h.length !== 6 && h.length !== 8) return undefined;
  return [
    parseInt(h.slice(0, 2), 16),
    parseInt(h.slice(2, 4), 16),
    parseInt(h.slice(4, 6), 16),
  ];
}

export interface ColorProbeResult {
  /** Pixels within `tolerance` (per-channel) of `color` / total pixels. */
  readonly ratio: number;
  readonly matching: number;
  readonly total: number;
  readonly readable: boolean;
}

/**
 * Normalized probe space. `"canvas"` maps `[0,1]` onto the whole
 * backing store; `"plot"` maps onto the element's real plot rect
 * (`lastLayout_.plot`) — the contract space for data-vs-gutter
 * assertions (axis labels and margins are not data).
 */
export type ProbeSpace = "canvas" | "plot";

/** Resolve a normalized region to backing-store pixels (`"plot"`
 * space reads the element's real layout; the fallback is the whole
 * canvas). */
async function regionBox(
  canvas: Locator,
  region: { x0: number; x1: number; y0: number; y1: number; space?: ProbeSpace | undefined },
): Promise<{ x0: number; x1: number; y0: number; y1: number }> {
  return canvas.evaluate((el, args) => {
    const cv = el as HTMLCanvasElement;
    if (args.space === "plot") {
      const host = (cv.getRootNode() as ShadowRoot).host as
        | { lastLayout_?: { plot?: { x: number; y: number; w: number; h: number } } }
        | undefined;
      const plot = host?.lastLayout_?.plot;
      if (plot !== undefined) {
        const dpr = cv.clientWidth > 0 ? cv.width / cv.clientWidth : 1;
        return {
          x0: Math.floor((plot.x + args.x0 * plot.w) * dpr),
          x1: Math.ceil((plot.x + args.x1 * plot.w) * dpr),
          y0: Math.floor((plot.y + args.y0 * plot.h) * dpr),
          y1: Math.ceil((plot.y + args.y1 * plot.h) * dpr),
        };
      }
    }
    return {
      x0: Math.floor(cv.width * args.x0),
      x1: Math.ceil(cv.width * args.x1),
      y0: Math.floor(cv.height * args.y0),
      y1: Math.ceil(cv.height * args.y1),
    };
  }, region);
}

/**
 * Fraction of pixels matching `color` in the given normalized region —
 * the non-color-contract oracle: the gap cue is a *different* color
 * (`--rvf-viz-gap`) than the series stroke (`--rvf-viz-waveform-stroke`),
 * so a spec proves "explicit gap mark" and "no zero-filled line through
 * the gap" independently of how much paint each cue costs.
 */
export async function colorCoverage(
  c: ControlHandle,
  region: { x0: number; x1: number; y0: number; y1: number },
  color: string,
  tolerance = 16,
  space?: ProbeSpace,
): Promise<ColorProbeResult> {
  const rgb = parseHexColor(color);
  if (rgb === undefined) {
    throw new Error(`colorCoverage: can't parse color "${color}"`);
  }
  const canvas = c.part("plot-canvas").first();
  if ((await canvas.count()) === 0) {
    throw new Error(`${c.selector}: no part="plot-canvas" canvas`);
  }
  const box = await regionBox(canvas, { ...region, space });
  return canvas.evaluate(
    (el, args) => {
      const cv = el as HTMLCanvasElement;
      const ctx = cv.getContext("2d") as CanvasRenderingContext2D | null;
      if (ctx === null) {
        return { ratio: 0, matching: 0, total: 0, readable: false };
      }
      const img = ctx.getImageData(
        args.box.x0,
        args.box.y0,
        args.box.x1 - args.box.x0,
        args.box.y1 - args.box.y0,
      );
      const [r, g, b] = args.rgb;
      const tol = args.tolerance;
      let matching = 0;
      const total = img.width * img.height;
      for (let i = 0; i < img.data.length; i += 4) {
        if (img.data[i + 3] === 0) continue;
        if (
          Math.abs(img.data[i]! - r) <= tol &&
          Math.abs(img.data[i + 1]! - g) <= tol &&
          Math.abs(img.data[i + 2]! - b) <= tol
        ) {
          matching++;
        }
      }
      return { ratio: total === 0 ? 0 : matching / total, matching, total, readable: true };
    },
    { box, rgb, tolerance },
  );
}

/**
 * Count of pixels *nearer to `color` than to `bg`* in the region —
 * the anti-aliasing-safe variant of {@link colorCoverage}. A 1 px
 * stroke rarely leaves a pure-color core: every painted pixel is a
 * blend of ink and background, so exact/tolerance matching misses it.
 * A pixel counts as "ink" when it sits closer in RGB space to the
 * ink color than to the backdrop — which is exactly what a composited
 * stroke or hatch produces.
 */
export async function colorAffinity(
  c: ControlHandle,
  region: { x0: number; x1: number; y0: number; y1: number },
  color: string,
  competitors: readonly string[],
  space?: ProbeSpace,
): Promise<ColorProbeResult> {
  const rgb = parseHexColor(color);
  const others = competitors.map((c2) => parseHexColor(c2));
  if (rgb === undefined) {
    throw new Error(`colorAffinity: can't parse color "${color}"`);
  }
  if (others.some((o) => o === undefined)) {
    throw new Error(
      `colorAffinity: can't parse competitor in ${JSON.stringify(competitors)}`,
    );
  }
  const canvas = c.part("plot-canvas").first();
  if ((await canvas.count()) === 0) {
    throw new Error(`${c.selector}: no part="plot-canvas" canvas`);
  }
  const box = await regionBox(canvas, { ...region, space });
  return canvas.evaluate(
    (el, args) => {
      const cv = el as HTMLCanvasElement;
      const ctx = cv.getContext("2d") as CanvasRenderingContext2D | null;
      if (ctx === null) {
        return { ratio: 0, matching: 0, total: 0, readable: false };
      }
      const img = ctx.getImageData(
        args.box.x0,
        args.box.y0,
        args.box.x1 - args.box.x0,
        args.box.y1 - args.box.y0,
      );
      const [fr, fg, fb] = args.rgb;
      let matching = 0;
      const total = img.width * img.height;
      for (let i = 0; i < img.data.length; i += 4) {
        if (img.data[i + 3] === 0) continue;
        const dr = img.data[i]! - fr;
        const dg = img.data[i + 1]! - fg;
        const db = img.data[i + 2]! - fb;
        const dInk = dr * dr + dg * dg + db * db;
        let nearestOther = Infinity;
        for (const o of args.others) {
          if (o === undefined) continue;
          const or = img.data[i]! - o[0]!;
          const og = img.data[i + 1]! - o[1]!;
          const ob = img.data[i + 2]! - o[2]!;
          const d = or * or + og * og + ob * ob;
          if (d < nearestOther) nearestOther = d;
        }
        if (dInk < nearestOther) matching++;
      }
      return { ratio: total === 0 ? 0 : matching / total, matching, total, readable: true };
    },
    { box, rgb, others },
  );
}

/** Assert the plot canvas is non-blank after a draw (Canvas2D path). */
export async function expectNonBlank(c: ControlHandle): Promise<FillProbeResult> {
  const p = await fillProbe(c, { x0: 0, x1: 1, y0: 0, y1: 1 });
  if (!p.readable) return p;
  expect(
    p.mean,
    `${c.selector}: plot canvas is blank (mean ${p.mean})`,
  ).toBeGreaterThan(0);
  return p;
}

/**
 * Assert the viz element produced draw ops — the backend-agnostic
 * "something painted" oracle (works when the canvas is WebGL-bound
 * and pixel reads are unavailable).
 */
export async function expectDrawOps(
  session: HarnessSession,
  key: string,
): Promise<VizStat> {
  const st = await vizStat(session, key);
  expect(st.framesDrawn, `${key}: no frames drawn`).toBeGreaterThan(0);
  expect(st.lastOpCount, `${key}: frame had no draw ops`).toBeGreaterThan(0);
  return st;
}

/**
 * Assert an explicit gap: the column band covering `gapX0..gapX1`
 * paints significantly less than the reference band `refX0..refX1` —
 * the NaN/missing contract (explicit gap marks, never zero-filled).
 */
export async function expectGapRegion(
  c: ControlHandle,
  gap: { x0: number; x1: number },
  ref: { x0: number; x1: number },
  band: { y0: number; y1: number },
): Promise<{ gap: FillProbeResult; ref: FillProbeResult }> {
  const gapP = await fillProbe(c, { ...gap, ...band });
  const refP = await fillProbe(c, { ...ref, ...band });
  if (gapP.readable && refP.readable) {
    expect(
      refP.mean,
      `${c.selector}: reference band must be painted`,
    ).toBeGreaterThan(0.05);
    expect(
      gapP.mean,
      `${c.selector}: gap band must paint less than data (gap ${gapP.mean.toFixed(3)} vs ref ${refP.mean.toFixed(3)})`,
    ).toBeLessThan(refP.mean * 0.6);
  }
  return { gap: gapP, ref: refP };
}

/** `stats` entry for one viz key from `__rvf.stats()`. */
export interface VizStat {
  readonly framesDrawn: number;
  readonly lastOpCount: number;
  readonly backendKind: string;
  readonly statusText: string;
}

/** Viz stat map (`key → VizStat`). */
export async function vizStats(
  session: HarnessSession,
): Promise<Record<string, VizStat>> {
  return (await session.stats()) as Record<string, VizStat>;
}

/** One viz's stat entry (throws if the key is absent). */
export async function vizStat(
  session: HarnessSession,
  key: string,
): Promise<VizStat> {
  const s = await vizStats(session);
  const st = s[key];
  if (st === undefined) {
    throw new Error(`no viz stat for "${key}" — have ${Object.keys(s)}`);
  }
  return st;
}

/** Deterministic viz tick: pumpNow + drawNow for every registered viz. */
export function drawAll(session: HarnessSession): Promise<void> {
  return session.drawAll();
}

/** Whether the page reports a WebGL2-capable backend on `key`. */
export async function backendKind(
  session: HarnessSession,
  key: string,
): Promise<string> {
  return (await vizStat(session, key)).backendKind;
}
