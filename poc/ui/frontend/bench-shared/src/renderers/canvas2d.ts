// Canvas2D バックエンド（参照下限。2D コンテキストのパス描画）。
// フレーム毎に columns 列の min/max 帯を ~2,000 頂点のパスとして fill する。

import type { FrameData } from "../workload";
import type { Renderer } from "./types";

export class Canvas2DRenderer implements Renderer {
  readonly backend = "canvas2d" as const;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly w: number;
  private readonly h: number;

  constructor(canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("canvas2d context unavailable");
    }
    this.ctx = ctx;
    this.w = canvas.width;
    this.h = canvas.height;
  }

  draw(frame: FrameData): void {
    const { ctx, w, h } = this;
    const mid = h / 2;
    const amp = h * 0.45;
    ctx.fillStyle = "#0d1117";
    ctx.fillRect(0, 0, w, h);
    ctx.strokeStyle = "#30363d";
    ctx.beginPath();
    ctx.moveTo(0, mid);
    ctx.lineTo(w, mid);
    ctx.stroke();

    // ピーク帯: 上辺を max で進み、下辺を min で戻る ~2,000 頂点の多角形。
    const { peaks, columns } = frame;
    const step = w / (columns - 1);
    ctx.beginPath();
    for (let i = 0; i < columns; i++) {
      const x = i * step;
      const y = mid - peaks[i * 2 + 1] * amp;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    for (let i = columns - 1; i >= 0; i--) {
      ctx.lineTo(i * step, mid - peaks[i * 2] * amp);
    }
    ctx.closePath();
    ctx.fillStyle = "#58a6ff";
    ctx.fill();
  }
}
