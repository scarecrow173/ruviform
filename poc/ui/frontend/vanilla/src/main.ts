// vanilla 参照ベンチ UI（FW ランタイムゼロの下限。design「vanilla 実装を参照ハーネス」）。
// 状態機械は Rust 側 exp/render.rs が所有し、このページは
// `ready` 通知 → `__poc.start` 受付 → rAF ベンチ実行 → `done`/`bench_error` 報告
// と 1s 周期の `alive` ハートビートを行う。

import {
  BenchRunner,
  installApi,
  postMessage,
  type BackendName,
  type LoadConfig,
} from "@ui-poc/bench-shared";

const canvas = document.getElementById("wave") as HTMLCanvasElement;
const metersEl = document.getElementById("meters") as HTMLElement;
const hud = document.getElementById("hud") as HTMLElement;

const runner = new BenchRunner({
  canvas,
  metersEl,
  hud,
  // AC5 更新粒度計装: ベンチ期間中の DOM mutation を done.stats.dom_mutations に記録
  // （solid variant と同一の計装経路 — bench-shared の MutationMeter を共有）。
  observeEl: document.getElementById("app") as HTMLElement,
  onDone: (stats) => postMessage({ type: "done", stats }),
  onError: (message) => postMessage({ type: "bench_error", message }),
});

const BACKENDS: readonly BackendName[] = ["canvas2d", "webgl2", "webgpu"];

installApi({
  start(cfg: unknown) {
    const c = (cfg ?? {}) as {
      backend?: string;
      frames?: number;
      load?: LoadConfig;
    };
    const backend = BACKENDS.includes(c.backend as BackendName)
      ? (c.backend as BackendName)
      : "canvas2d";
    const frames =
      typeof c.frames === "number" && c.frames > 0 ? Math.floor(c.frames) : 600;
    void runner.start({ backend, frames, load: c.load });
  },
});

// ページロード到達の通知（Rust 側 start 送信のトリガ）。
postMessage({ type: "ready", href: location.href, ua: navigator.userAgent });

// alive ハートビート: Rust 側の liveness 観測 + idle 検出（start 取りこぼし時の再送駆動）。
setInterval(() => {
  postMessage({
    type: "alive",
    state: runner.currentState,
    frames: runner.progress,
  });
}, 1000);
