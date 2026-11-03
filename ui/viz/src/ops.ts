/**
 * DrawOp — the backend-neutral scene IR (Issue #260, §12.5).
 *
 * Scene builders produce an ordered op list in **plot layer first,
 * overlay layer last** order. Executors then either run the whole list
 * on one Canvas2D context or split it: plot ops onto the WebGL2 canvas,
 * overlay ops onto a stacked 2D canvas. Because geometry lives in the
 * ops — not in the backend — axis/selection/gap coordinates are
 * identical across backends by construction (the shared-fixture gate
 * compares op lists, not pixels).
 *
 * Coordinates are CSS px inside the element's drawing surface. Colors
 * are resolved CSS color strings (from VizStyle); the WebGL2 executor
 * parses them to RGBA via `color.ts` and declares `color-unparseable`
 * when it cannot.
 */

export type DrawLayer = "plot" | "overlay";

export type DrawOp =
  | {
      readonly kind: "rect";
      readonly layer: DrawLayer;
      readonly x: number;
      readonly y: number;
      readonly w: number;
      readonly h: number;
      readonly fill: string;
    }
  | {
      readonly kind: "rectStroke";
      readonly layer: DrawLayer;
      readonly x: number;
      readonly y: number;
      readonly w: number;
      readonly h: number;
      readonly stroke: string;
      readonly width: number;
    }
  | {
      /**
       * Column band: vertical extent `[top[i], bottom[i]]` at each
       * column's x position. `span` is either uniform columns
       * (`x = x0 + i·dx`, waveform envelopes) or explicit per-point x
       * (`xs`, e.g. log-spaced spectrum fills). Canvas2D rasterizes a
       * closed path; WebGL2 a triangle strip. Points must be finite —
       * scene builders split runs at NaN before emitting.
       */
      readonly kind: "band";
      readonly layer: DrawLayer;
      readonly span:
        | { readonly x0: number; readonly dx: number }
        | { readonly xs: readonly number[] };
      readonly top: readonly number[];
      readonly bottom: readonly number[];
      readonly fill: string;
    }
  | {
      /** Polyline stroke (butt caps, miter-less joins). */
      readonly kind: "line";
      readonly layer: DrawLayer;
      readonly pts: readonly number[];
      readonly stroke: string;
      readonly width: number;
    }
  | {
      /**
       * Convex polygon fill (fan-triangulated by WebGL2). The contract
       * limits this op to convex shapes — e.g. marker diamonds or the
       * meter clip triangle — so fan triangulation is always exact.
       */
      readonly kind: "poly";
      readonly layer: DrawLayer;
      readonly pts: readonly number[];
      readonly fill: string;
    }
  | {
      /**
       * Diagonal hatch fill over a rect — the non-color cue for
       * missing/stale regions (§12.6: states never rely on hue alone).
       * `spacing` is px between strokes, `angle` radians from +x.
       */
      readonly kind: "hatch";
      readonly layer: DrawLayer;
      readonly x: number;
      readonly y: number;
      readonly w: number;
      readonly h: number;
      readonly stroke: string;
      readonly width: number;
      readonly spacing: number;
      readonly angle: number;
    }
  | {
      /** Text is always an overlay-layer op. */
      readonly kind: "text";
      readonly layer: "overlay";
      readonly x: number;
      readonly y: number;
      readonly text: string;
      readonly fill: string;
      readonly font: string;
      readonly align: "left" | "center" | "right";
      readonly baseline: "top" | "middle" | "bottom" | "alphabetic";
    }
  | {
      /** Push a rectangular clip (Canvas2D save+clip; WebGL2 scissor). */
      readonly kind: "clipPush";
      readonly layer: DrawLayer;
      readonly x: number;
      readonly y: number;
      readonly w: number;
      readonly h: number;
    }
  | {
      readonly kind: "clipPop";
      readonly layer: DrawLayer;
    };

/** Split an op list into layers, preserving in-layer order. */
export function partitionOps(ops: readonly DrawOp[]): {
  readonly plot: DrawOp[];
  readonly overlay: DrawOp[];
} {
  const plot: DrawOp[] = [];
  const overlay: DrawOp[] = [];
  for (const op of ops) {
    (op.layer === "plot" ? plot : overlay).push(op);
  }
  return { plot, overlay };
}
