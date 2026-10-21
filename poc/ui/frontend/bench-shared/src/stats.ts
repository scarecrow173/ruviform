// rAF フレーム統計（FW 非依存）。
// - frame_time_ms: 連続する rAF タイムスタンプの差分列（vsync 待ち込みの実間隔）。
// - work_time_ms : 1 フレーム内のワークロード生成 + 描画呼出の経過時間。
// - dropped      : フレーム間隔 > 2*vsync の本数（vsync は収集間隔の中央値で推定）。
// - achieved_fps : 記録フレーム数 / 経過秒。
// percentile 集計は Rust 側 metrics.rs の責務（design「done メッセージ経由で集約」）
// のため、ここでは生サンプル列を保持して返す。

/** rAF 駆動のフレーム統計レコーダ。 */
export class FrameRecorder {
  private prevTs = 0;
  private firstTs = 0;
  /** フレーム間隔列（ms）。最初の tick は基準点確立のみで記録しない。 */
  readonly frameTimes: number[] = [];
  /** フレーム内作業時間列（ms）。 */
  readonly workTimes: number[] = [];

  /** rAF コールバック冒頭にタイムスタンプを渡す。 */
  tick(ts: number): void {
    if (this.prevTs > 0) {
      this.frameTimes.push(ts - this.prevTs);
    } else {
      this.firstTs = ts;
    }
    this.prevTs = ts;
  }

  /** フレーム内作業時間を記録する（performance.now 差分）。 */
  recordWork(ms: number): void {
    this.workTimes.push(ms);
  }

  /** 記録済みフレーム間隔数。 */
  get frames(): number {
    return this.frameTimes.length;
  }

  /** 基準点から最終フレームまでの経過 ms。 */
  get elapsedMs(): number {
    return this.frameTimes.length > 0 ? this.prevTs - this.firstTs : 0;
  }
}

/** 昇順ソート済み配列の中央値（空入力は 0）。 */
export function median(sortedCopy: number[]): number {
  if (sortedCopy.length === 0) return 0;
  const s = [...sortedCopy].sort((a, b) => a - b);
  const mid = s.length >> 1;
  return s.length % 2 === 1 ? s[mid] : (s[mid - 1] + s[mid]) / 2;
}

/** フレーム統計のまとめ（dropped / vsync / achieved_fps）。 */
export function summarizeFrames(rec: FrameRecorder): {
  vsync_ms: number;
  dropped: number;
  achieved_fps: number;
} {
  const vsync = median(rec.frameTimes);
  const dropped =
    vsync > 0 ? rec.frameTimes.filter((t) => t > 2 * vsync).length : 0;
  const achieved = rec.elapsedMs > 0 ? (rec.frames * 1000) / rec.elapsedMs : 0;
  return { vsync_ms: vsync, dropped, achieved_fps: achieved };
}
