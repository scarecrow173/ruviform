// ベンチ実行器（FW 非依存）。
// Rust 側 exp/render.rs が evaluate_script で __poc.start({backend, frames}) を
// 呼ぶと rAF ループを開始し、warmup 後に `frames` サンプルのフレーム間隔を記録する。
// 完了時に onDone(stats) → done メッセージ、致命的失敗は onError → bench_error。

import { createMeterHost } from "./meters";
import { MutationMeter } from "./mutations";
import { createRenderer } from "./renderers";
import type { BackendName } from "./renderers";
import { FrameRecorder, summarizeFrames } from "./stats";
import {
  METER_CHANNELS,
  PEAK_COLUMNS,
  PreviewLayer,
  WorkloadGenerator,
} from "./workload";

/** warmup として記録対象外とする先頭フレーム数（初動コストを統計から除く）。 */
export const WARMUP_FRAMES = 12;
/** HUD テキストの更新間隔（DOM churn を workload から隔離するため逓減更新）。 */
const HUD_INTERVAL = 15;

/** FW 側が用意するメータ駆動 I/F（Solid の signal 駆動メータ等）。 */
export interface MeterDriver {
  setLevels(levels: Float32Array): void;
}

export interface BenchElements {
  canvas: HTMLCanvasElement;
  metersEl: HTMLElement;
  hud: HTMLElement;
  /** 指定時は `metersEl` への命令的 DOM 構築（createMeterHost）をスキップし、
   *  この driver へレベル更新を委譲する（AC5: FW 側メータ実装の接続点）。 */
  metersHost?: MeterDriver;
  /** 指定時はベンチ期間中の DOM mutation を MutationMeter で計装し、
   *  done.stats.dom_mutations に記録する（AC5 更新粒度比較）。 */
  observeEl?: HTMLElement;
  onDone: (stats: Record<string, unknown>) => void;
  onError: (message: string) => void;
}

export type BenchState = "idle" | "running" | "done" | "error";

/** Issue #57 高負荷再計測のワークロードスケール（未指定は基準値）。 */
export interface LoadConfig {
  /** ピーク列数（既定 PEAK_COLUMNS=1000）。 */
  peaks?: number;
  /** レベルメータチャンネル数（既定 METER_CHANNELS=8）。 */
  meters?: number;
  /** 720p 合成プレビューレイヤーを毎フレーム描画する（動画プレビュー近似）。 */
  preview?: boolean;
}

export class BenchRunner {
  private state: BenchState = "idle";
  private framesDone = 0;

  constructor(private readonly els: BenchElements) {}

  get currentState(): BenchState {
    return this.state;
  }

  get progress(): number {
    return this.framesDone;
  }

  /** `__poc.start` から呼ばれる実行本体。二重起動は冪等に無視する。 */
  async start(cfg: {
    backend: BackendName;
    frames: number;
    load?: LoadConfig;
  }): Promise<void> {
    if (this.state !== "idle") return;
    this.state = "running";
    try {
      const { renderer, used, fallback, caps } = await createRenderer(
        cfg.backend,
        this.els.canvas
      );
      const columns = cfg.load?.peaks ?? PEAK_COLUMNS;
      const meterChannels = cfg.load?.meters ?? METER_CHANNELS;
      // FW 側 driver があればそちらへ委譲し、無ければ vanilla と同じ
      // 命令的 DOM メータを `metersEl` 内に構築する。どちらが走ったかを
      // stats.meters_driver に残す（注入パス利用の証跡 — AC5）。
      const meters =
        this.els.metersHost ??
        createMeterHost(this.els.metersEl, meterChannels);
      const metersDriver = this.els.metersHost ? "injected" : "createMeterHost";
      const gen = new WorkloadGenerator(columns, meterChannels);
      const preview = cfg.load?.preview ? new PreviewLayer() : null;
      if (preview) this.els.canvas.insertAdjacentElement("afterend", preview.canvas);
      const rec = new FrameRecorder();
      this.els.hud.textContent = `backend=${used} — running`;

      // DOM mutation 計装（AC5）: メータ DOM の構築後・ループ開始前に開始し、
      // warmup 込みの全 rAF フレームにわたる更新レコードを計数する。
      const mutMeter = this.els.observeEl ? new MutationMeter() : null;
      if (mutMeter && this.els.observeEl) mutMeter.start(this.els.observeEl);

      let rafCount = 0;
      const loop = (ts: number): void => {
        rafCount++;
        const recording = rafCount > WARMUP_FRAMES;
        if (recording) rec.tick(ts);

        const w0 = performance.now();
        const frame = gen.next();
        renderer.draw(frame);
        meters.setLevels(frame.levels);
        preview?.draw();
        if (recording) rec.recordWork(performance.now() - w0);
        this.framesDone = rec.frames;

        if (rec.frames % HUD_INTERVAL === 0) {
          const est = rec.elapsedMs > 0 ? (rec.frames * 1000) / rec.elapsedMs : 0;
          this.els.hud.textContent = `backend=${used} | ${rec.frames}/${cfg.frames} | ${est.toFixed(1)} fps`;
        }

        if (rec.frames < cfg.frames) {
          requestAnimationFrame(loop);
        } else {
          this.state = "done";
          const dom = mutMeter?.stop() ?? null;
          const s = summarizeFrames(rec);
          this.els.hud.textContent = `backend=${used} | done ${rec.frames} frames | ${s.achieved_fps.toFixed(1)} fps`;
          this.els.onDone({
            backend_requested: cfg.backend,
            backend_used: used,
            fallback,
            caps,
            warmup_frames: WARMUP_FRAMES,
            frames: rec.frames,
            elapsed_ms: rec.elapsedMs,
            vsync_ms: s.vsync_ms,
            dropped: s.dropped,
            achieved_fps: s.achieved_fps,
            frame_time_ms: rec.frameTimes,
            work_time_ms: rec.workTimes,
            meters_driver: metersDriver,
            load: {
              peaks: columns,
              meters: meterChannels,
              preview: preview !== null,
            },
            dom_mutations: dom
              ? { ...dom, observed_frames: rafCount, meter_channels: meterChannels }
              : null,
          });
        }
      };
      requestAnimationFrame(loop);
    } catch (e) {
      this.state = "error";
      this.els.onError(e instanceof Error ? e.message : String(e));
    }
  }
}
