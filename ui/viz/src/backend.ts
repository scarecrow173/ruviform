/**
 * Backend selection + capability/fallback contract (Issue #260,
 * §12.5).
 *
 * `createVizBackend` picks a renderer for the element's canvases and
 * reports exactly what it can paint:
 *
 * - `"canvas2d"` — one context executes the whole op list (plot +
 *   overlay, in order). The reference backend: every op is native.
 * - `"webgl2"` — the GL canvas takes the **plot** subset; the stacked
 *   2D overlay canvas takes the **overlay** subset (text, selection,
 *   markers, stale badges). `needsOverlayCanvas` is `true`; the element
 *   supplies and sizes the second canvas.
 *
 * Fallback policy: preference `"auto"` tries WebGL2 first (per-backend
 * suitability, not a forced uniform backend), then Canvas2D; explicit
 * `"webgl2"` also degrades to Canvas2D with a recorded reason rather
 * than failing blank. When no context is usable at all the result is
 * `null` and the element shows the DOM-level unsupported status —
 * never a silent empty surface.
 */

import { Canvas2DExecutor, type VizExecutor } from "./canvas2d.ts";
import { partitionOps } from "./ops.ts";
import { WebGl2Executor } from "./webgl2.ts";
import type { DrawOp } from "./ops.ts";

export type BackendPreference = "auto" | "canvas2d" | "webgl2";

export interface VizBackendCapabilities {
  /** Where text lands — always `"overlay"` today: WebGL2 cannot
   * rasterize text, and Canvas2D draws it on the same context. */
  readonly text: "native" | "overlay";
  /** Declared notes for the selected backend (e.g. overlay split). */
  readonly notes: readonly string[];
}

export interface VizBackend {
  readonly kind: "canvas2d" | "webgl2";
  readonly capabilities: VizBackendCapabilities;
  /** Present when the preferred backend could not be used. */
  readonly fallback: { readonly requested: BackendPreference; readonly reason: string } | null;
  /** Whether the element must supply a stacked 2D overlay canvas. */
  readonly needsOverlayCanvas: boolean;
  /** Run a full op list (plot + overlay, correctly routed). */
  execute(ops: readonly DrawOp[]): void;
}

export interface BackendSize {
  /** CSS px. */
  readonly width: number;
  readonly height: number;
  readonly dpr: number;
}

export interface BackendCanvases {
  /** Bottom canvas — GL or 2D plot surface. */
  readonly plot: HTMLCanvasElement;
  /** Stacked 2D canvas for overlay ops (WebGL2 mode); may be the same
   * element as `plot` when unused. */
  readonly overlay: HTMLCanvasElement | null;
}

export function createVizBackend(
  canvases: BackendCanvases,
  pref: BackendPreference,
  size: BackendSize,
): VizBackend | null {
  const order: Exclude<BackendPreference, "auto">[] =
    pref === "canvas2d" ? ["canvas2d"] : ["webgl2", "canvas2d"];
  let fallback: VizBackend["fallback"] = null;

  for (const kind of order) {
    if (kind === "webgl2") {
      const backend = tryWebGl2(canvases, size);
      if (backend !== null) {
        return { ...backend, fallback };
      }
      if (pref === "webgl2" || pref === "auto") {
        fallback = {
          requested: pref,
          reason: "webgl2 context or program unavailable",
        };
      }
      continue;
    }
    const ctx = get2d(canvases.plot);
    if (ctx !== null) {
      const executor = new Canvas2DExecutor(
        ctx,
        size.width,
        size.height,
        size.dpr,
      );
      return {
        kind: "canvas2d",
        capabilities: { text: "native", notes: [] },
        fallback,
        needsOverlayCanvas: false,
        execute: (ops) => executor.execute(ops),
      };
    }
  }
  return null;
}

function tryWebGl2(
  canvases: BackendCanvases,
  size: BackendSize,
): Omit<VizBackend, "fallback"> | null {
  if (canvases.overlay === null) return null;
  let gl: WebGL2RenderingContext | null = null;
  try {
    gl = canvases.plot.getContext("webgl2", {
      alpha: true,
      antialias: true,
    }) as WebGL2RenderingContext | null;
  } catch {
    gl = null;
  }
  if (gl === null) return null;
  const overlayCtx = get2d(canvases.overlay);
  if (overlayCtx === null) return null;
  let plot: VizExecutor;
  try {
    plot = new WebGl2Executor(gl, size.width, size.height, size.dpr);
  } catch {
    return null;
  }
  const overlay = new Canvas2DExecutor(
    overlayCtx,
    size.width,
    size.height,
    size.dpr,
  );
  return {
    kind: "webgl2",
    capabilities: {
      text: "overlay",
      notes: ["plot ops on webgl2; overlay ops on stacked canvas2d"],
    },
    needsOverlayCanvas: true,
    execute: (ops) => {
      const { plot: plotOps, overlay: overlayOps } = partitionOps(ops);
      plot.execute(plotOps);
      overlay.execute(overlayOps);
    },
  };
}

function get2d(canvas: HTMLCanvasElement): CanvasRenderingContext2D | null {
  try {
    return canvas.getContext("2d") as CanvasRenderingContext2D | null;
  } catch {
    return null;
  }
}
