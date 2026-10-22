// ワークロード生成（FW 非依存。design AC2 正本）。
// スクロール波形のピーク列（PEAK_COLUMNS×2 ≈ 2,000 点/フレーム）と
// レベルメータ群（8ch attack/decay バリスティック）を決定論的に合成する。
// 実信号入力は存在しないため、種固定の疑似乱数 + 合成正弦波で
// 「実測相当の計算コスト」を毎フレーム支払う。

/** 波形ピーク列の列数。peaks 長は columns*2（min/max 交互）。 */
export const PEAK_COLUMNS = 1000;
/** レベルメータのチャンネル数。 */
export const METER_CHANNELS = 8;
/** 列あたりの min/max 抽出サブサンプル数。 */
const SUB_SAMPLES = 4;

/** 1 フレーム分の描画データ。 */
export interface FrameData {
  /** ピーク列。列ごとの [min,max] が交互に並ぶ [-1,1] 範囲の値（長さ columns*2）。 */
  peaks: Float32Array;
  /** ピーク列の列数（peaks.length / 2）。 */
  columns: number;
  /** レベルメータ値 [0,1]（METER_CHANNELS 分）。 */
  levels: Float32Array;
  /** 生成フレーム番号（0 始まり）。 */
  frame: number;
}

/**
 * 決定論ノイズ（mulberry32）。比較可能性のためシード固定 —
 * 3 バックエンドが同一データ列を描くことが AC2 の前提。
 */
function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** 合成信号: 3 成分の正弦波（x はスクロール時間軸）。 */
function signal(x: number): number {
  return (
    0.5 * Math.sin(x * 2 * Math.PI * 3.0) +
    0.28 * Math.sin(x * 2 * Math.PI * 7.3 + 1.7) +
    0.16 * Math.sin(x * 2 * Math.PI * 19.7 + 0.3)
  );
}

/**
 * フレームごとのワークロード生成器。
 * `next()` は内部バッファを使い回して返す（呼び出し側は同一フレーム内で消費すること）。
 * `columns`/`channels` は Issue #57 高負荷再計測のスケール引数
 * （既定値は Issue #28 PoC の基準ワークロードと同一）。
 */
export class WorkloadGenerator {
  private readonly peaks: Float32Array;
  private readonly levels: Float32Array;
  private readonly rng = mulberry32(0x5eed);
  private frame = 0;

  constructor(
    private readonly columns: number = PEAK_COLUMNS,
    private readonly channels: number = METER_CHANNELS
  ) {
    this.peaks = new Float32Array(columns * 2);
    this.levels = new Float32Array(channels);
  }

  /** 次フレーム分のピーク列 + メータ値を生成する。 */
  next(): FrameData {
    const t = this.frame / 60; // 仮想 60fps 時間軸（スクロール速度は実時間非依存で固定）
    const rng = this.rng;
    const peaks = this.peaks;
    const columnSpan = 0.5 / this.columns; // 表示窓幅 0.5 単位を列数へマップ
    for (let i = 0; i < this.columns; i++) {
      const base = t * 2.0 + i * columnSpan;
      let lo = 1;
      let hi = -1;
      for (let s = 0; s < SUB_SAMPLES; s++) {
        const v = signal(base + s * (columnSpan / SUB_SAMPLES)) + (rng() - 0.5) * 0.18;
        if (v < lo) lo = v;
        if (v > hi) hi = v;
      }
      peaks[i * 2] = lo;
      peaks[i * 2 + 1] = hi;
    }
    // メータ群: チャンネル別位相の |signal| に attack 速・decay 遅で追従。
    for (let ch = 0; ch < this.channels; ch++) {
      const target = Math.min(
        1,
        Math.abs(signal(t * 1.7 + ch * 0.9)) * 1.25 + (rng() - 0.5) * 0.08
      );
      const cur = this.levels[ch];
      this.levels[ch] =
        target > cur ? cur + (target - cur) * 0.55 : Math.max(0, cur - 0.045);
    }
    this.frame++;
    return {
      peaks,
      columns: this.columns,
      levels: this.levels,
      frame: this.frame - 1,
    };
  }
}

/** プレビュー canvas の内部解像度（720p — 動画プレビュー相当の更新レイヤー）。 */
export const PREVIEW_WIDTH = 1280;
/** 同上（高さ）。 */
export const PREVIEW_HEIGHT = 720;

/**
 * 動画プレビュー相当の合成レイヤー（Issue #57 高負荷再計測）。
 * 1280x720 の canvas へ毎フレームシーンを描画する — 実 `<video>` の
 * デコードコストは JS 側に現れないため、「コンポジタに毎フレーム
 * 更新される 720p レイヤーがある」状態の近似として使う。
 * 表示サイズは CSS で縮小し、内部解像度で描画コストを支払う。
 */
export class PreviewLayer {
  /** 挿入対象の canvas 要素（呼び出し側が DOM へ配置する）。 */
  readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private frame = 0;

  constructor() {
    this.canvas = document.createElement("canvas");
    this.canvas.width = PREVIEW_WIDTH;
    this.canvas.height = PREVIEW_HEIGHT;
    this.canvas.style.cssText =
      "width:480px;height:270px;display:block;margin-top:8px;";
    const ctx = this.canvas.getContext("2d");
    if (!ctx) throw new Error("preview canvas 2d context unavailable");
    this.ctx = ctx;
  }

  /** 1 フレーム分の合成プレビュー映像を描画する。 */
  draw(): void {
    const ctx = this.ctx;
    const w = PREVIEW_WIDTH;
    const h = PREVIEW_HEIGHT;
    const t = this.frame / 60;
    // カラーバー背景（全域塗り + 移動ブロック + タイムコード描字）。
    const bars = ["#7a7a7a", "#c8c828", "#28c8c8", "#28c828", "#c828c8", "#c82828", "#2828c8"];
    const bw = w / bars.length;
    for (let i = 0; i < bars.length; i++) {
      ctx.fillStyle = bars[i];
      ctx.fillRect(i * bw, 0, bw, h);
    }
    const x = ((t * 240) % (w + 200)) - 100;
    ctx.fillStyle = `hsl(${(t * 60) % 360},80%,55%)`;
    ctx.fillRect(x, h * 0.3, 200, h * 0.4);
    ctx.fillStyle = "#ffffff";
    ctx.font = "48px monospace";
    ctx.fillText(`TC ${this.frame}`, 40, h - 40);
    this.frame++;
  }
}
