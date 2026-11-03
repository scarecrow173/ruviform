/**
 * Canvas2D executor — the reference backend. Runs the full DrawOp list
 * on one `CanvasRenderingContext2D` (plot + overlay ops in order); when
 * used as the WebGL2 overlay painter it runs just the overlay subset.
 *
 * Coordinates are CSS px; the caller supplies the device-pixel ratio
 * and the executor sets the transform once per `execute`.
 */

import type { DrawOp } from "./ops.ts";

export interface VizExecutor {
  readonly kind: "canvas2d" | "webgl2";
  /** Backend-reported notes from the last execute (unsupported
   * substitutions, unparseable colors) — surfaced to the element's
   * capability/status channel, never silently dropped. */
  readonly notes: readonly string[];
  execute(ops: readonly DrawOp[]): void;
}

export class Canvas2DExecutor implements VizExecutor {
  readonly kind = "canvas2d" as const;
  readonly notes: string[] = [];

  private readonly ctx: CanvasRenderingContext2D;
  private readonly width: number;
  private readonly height: number;
  private readonly dpr: number;

  constructor(
    ctx: CanvasRenderingContext2D,
    width: number,
    height: number,
    dpr: number,
  ) {
    this.ctx = ctx;
    this.width = width;
    this.height = height;
    this.dpr = dpr;
  }

  execute(ops: readonly DrawOp[]): void {
    const { ctx } = this;
    this.notes.length = 0;
    ctx.save();
    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    ctx.clearRect(0, 0, this.width, this.height);
    for (const op of ops) this.exec(op);
    ctx.restore();
  }

  private exec(op: DrawOp): void {
    const { ctx } = this;
    switch (op.kind) {
      case "rect":
        ctx.fillStyle = op.fill;
        ctx.fillRect(op.x, op.y, op.w, op.h);
        break;
      case "rectStroke":
        ctx.strokeStyle = op.stroke;
        ctx.lineWidth = op.width;
        ctx.strokeRect(op.x, op.y, op.w, op.h);
        break;
      case "band": {
        const xs = bandXs(op);
        const n = op.top.length;
        if (n === 0) break;
        ctx.beginPath();
        ctx.moveTo(xs[0] ?? 0, op.top[0] ?? 0);
        for (let i = 1; i < n; i++) ctx.lineTo(xs[i] ?? 0, op.top[i] ?? 0);
        for (let i = n - 1; i >= 0; i--) {
          ctx.lineTo(xs[i] ?? 0, op.bottom[i] ?? 0);
        }
        ctx.closePath();
        ctx.fillStyle = op.fill;
        ctx.fill();
        break;
      }
      case "line": {
        if (op.pts.length < 4) break;
        ctx.beginPath();
        ctx.moveTo(op.pts[0] ?? 0, op.pts[1] ?? 0);
        for (let i = 2; i < op.pts.length; i += 2) {
          ctx.lineTo(op.pts[i] ?? 0, op.pts[i + 1] ?? 0);
        }
        ctx.strokeStyle = op.stroke;
        ctx.lineWidth = op.width;
        ctx.stroke();
        break;
      }
      case "poly": {
        if (op.pts.length < 6) break;
        ctx.beginPath();
        ctx.moveTo(op.pts[0] ?? 0, op.pts[1] ?? 0);
        for (let i = 2; i < op.pts.length; i += 2) {
          ctx.lineTo(op.pts[i] ?? 0, op.pts[i + 1] ?? 0);
        }
        ctx.closePath();
        ctx.fillStyle = op.fill;
        ctx.fill();
        break;
      }
      case "hatch": {
        ctx.save();
        ctx.beginPath();
        ctx.rect(op.x, op.y, op.w, op.h);
        ctx.clip();
        ctx.beginPath();
        for (const [x0, y0, x1, y1] of hatchLines(op)) {
          ctx.moveTo(x0, y0);
          ctx.lineTo(x1, y1);
        }
        ctx.strokeStyle = op.stroke;
        ctx.lineWidth = op.width;
        ctx.stroke();
        ctx.restore();
        break;
      }
      case "text":
        ctx.font = op.font;
        ctx.fillStyle = op.fill;
        ctx.textAlign = op.align;
        ctx.textBaseline = op.baseline;
        ctx.fillText(op.text, op.x, op.y);
        break;
      case "clipPush":
        ctx.save();
        ctx.beginPath();
        ctx.rect(op.x, op.y, op.w, op.h);
        ctx.clip();
        break;
      case "clipPop":
        ctx.restore();
        break;
    }
  }
}

/** Column x positions of a band op. */
export function bandXs(op: Extract<DrawOp, { kind: "band" }>): number[] {
  const n = op.top.length;
  const out = new Array<number>(n);
  if ("x0" in op.span) {
    for (let i = 0; i < n; i++) out[i] = op.span.x0 + i * op.span.dx;
  } else {
    for (let i = 0; i < n; i++) out[i] = op.span.xs[i] ?? 0;
  }
  return out;
}

/**
 * Diagonal hatch line segments covering a rect (shared by the 2D and
 * GL executors). Lines run along `angle` spaced `spacing` px apart,
 * each clipped to the rect's diagonal extent — the executor relies on
 * its clip for exact bounds.
 */
export function* hatchLines(
  op: Extract<DrawOp, { kind: "hatch" }>,
): Generator<readonly [number, number, number, number]> {
  const spacing =
    Number.isFinite(op.spacing) && op.spacing > 0 ? op.spacing : 6;
  const dx = Math.cos(op.angle);
  const dy = Math.sin(op.angle);
  const nx = -dy;
  const ny = dx;
  // Project the four corners onto the normal to find the covered range.
  let lo = Number.POSITIVE_INFINITY;
  let hi = Number.NEGATIVE_INFINITY;
  for (const [cx, cy] of [
    [op.x, op.y],
    [op.x + op.w, op.y],
    [op.x, op.y + op.h],
    [op.x + op.w, op.y + op.h],
  ] as const) {
    const p = cx * nx + cy * ny;
    if (p < lo) lo = p;
    if (p > hi) hi = p;
  }
  // Line length = rect diagonal + slack so the clipped stroke is solid.
  const len = Math.hypot(op.w, op.h) + spacing;
  const cx = op.x + op.w / 2;
  const cy = op.y + op.h / 2;
  const pc = cx * nx + cy * ny;
  const start = pc + Math.floor((lo - pc) / spacing) * spacing;
  for (let p = start; p <= hi; p += spacing) {
    const bx = cx + nx * (p - pc);
    const by = cy + ny * (p - pc);
    yield [bx - dx * len, by - dy * len, bx + dx * len, by + dy * len];
  }
}
