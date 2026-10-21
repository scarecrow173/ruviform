// ベンチ実行器（FW 非依存）。
// Rust 側 exp/render.rs が evaluate_script で __poc.start({backend, frames}) を
// 呼ぶと rAF ループを開始し、warmup 後に `frames` サンプルのフレーム間隔を記録する。
// 完了時に onDone(stats) → done メッセージ、致命的失敗は onError → bench_error。

import { createMeterHost } from "./meters";
import { createRenderer } from "./renderers";
import type { BackendName } from "./renderers";
import { FrameRecorder, summarizeFrames } from "./stats";
import { METER_CHANNELS, WorkloadGenerator } from "./workload";

/** warmup として記録対象外とする先頭フレーム数（初動コストを統計から除く）。 */
export const WARMUP_FRAMES = 12;
/** HUD テキストの更新間隔（DOM churn を workload から隔離するため逓減更新）。 */
const HUD_INTERVAL = 15;

export interface BenchElements {
  canvas: HTMLCanvasElement;
  metersEl: HTMLElement;
  hud: HTMLElement;
  onDone: (stats: Record<string, unknown>) => void;
  onError: (message: string) => void;
}

export type BenchState = "idle" | "running" | "done" | "error";

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
  async start(cfg: { backend: BackendName; frames: number }): Promise<void> {
    if (this.state !== "idle") return;
    this.state = "running";
    try {
      const { renderer, used, fallback, caps } = await createRenderer(
        cfg.backend,
        this.els.canvas
      );
      const meters = createMeterHost(this.els.metersEl, METER_CHANNELS);
      const gen = new WorkloadGenerator();
      const rec = new FrameRecorder();
      this.els.hud.textContent = `backend=${used} — running`;

      let rafCount = 0;
      const loop = (ts: number): void => {
        rafCount++;
        const recording = rafCount > WARMUP_FRAMES;
        if (recording) rec.tick(ts);

        const w0 = performance.now();
        const frame = gen.next();
        renderer.draw(frame);
        meters.setLevels(frame.levels);
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
