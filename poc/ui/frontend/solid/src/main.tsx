// Solid 版ベンチ UI（AC5 FW 比較候補。vanilla 参照 UI の FW 移植）。
// vanilla と同じ bench-shared ワークロード・状態機械・報告契約で、
// メータ群だけを Solid signal 駆動（meters.tsx の Meters コンポーネント）に置き換える。
// 状態機械は Rust 側 exp/frontend.rs が所有し、このページは
// `ready` 通知 → `__poc.start` 受付 → rAF ベンチ実行 → `done`/`bench_error` 報告
// と 1s 周期の `alive` ハートビートを行う（vanilla と同じ構造）。

import { render } from "solid-js/web";
import {
  BenchRunner,
  installApi,
  postMessage,
  type BackendName,
} from "@ui-poc/bench-shared";

import { Meters, type MeterHandle } from "./meters";

// Solid の ref コールバック経由で取得した DOM / driver の受け皿。
// render() は同期的に mount するため、戻った時点で全て束縛済み。
const els: {
  canvas?: HTMLCanvasElement;
  meters?: HTMLElement;
  meterHost?: MeterHandle;
  hud?: HTMLElement;
} = {};

function App() {
  return (
    <>
      <canvas
        id="wave"
        width={944}
        height={420}
        ref={(el) => {
          els.canvas = el;
        }}
      />
      <Meters
        ref={(h) => {
          els.meterHost = h;
        }}
        metersRef={(el) => {
          els.meters = el;
        }}
      />
      <div
        id="hud"
        ref={(el) => {
          els.hud = el;
        }}
      >
        idle — waiting for __poc.start
      </div>
    </>
  );
}

const appEl = document.getElementById("app") as HTMLElement;
render(App, appEl);

const runner = new BenchRunner({
  canvas: els.canvas as HTMLCanvasElement,
  metersEl: els.meters as HTMLElement,
  // メータ更新は Solid コンポーネント側の signal 駆動 driver へ委譲する
  // （vanilla は省略して createMeterHost の命令的 DOM 更新を使う）。
  metersHost: els.meterHost,
  hud: els.hud as HTMLElement,
  // AC5 更新粒度計装（vanilla と同一の MutationMeter 共有経路）。
  observeEl: appEl,
  onDone: (stats) => postMessage({ type: "done", stats }),
  onError: (message) => postMessage({ type: "bench_error", message }),
});

const BACKENDS: readonly BackendName[] = ["canvas2d", "webgl2", "webgpu"];

installApi({
  start(cfg: unknown) {
    const c = (cfg ?? {}) as { backend?: string; frames?: number };
    const backend = BACKENDS.includes(c.backend as BackendName)
      ? (c.backend as BackendName)
      : "canvas2d";
    const frames =
      typeof c.frames === "number" && c.frames > 0 ? Math.floor(c.frames) : 600;
    void runner.start({ backend, frames });
  },
});

// ページロード到達の通知（Rust 側 start 送信のトリガ）。
postMessage({ type: "ready", href: location.href, ua: navigator.userAgent });

// alive ハートビート: Rust 側の liveness 観測 + idle 検出（start 再送駆動）。
setInterval(() => {
  postMessage({
    type: "alive",
    state: runner.currentState,
    frames: runner.progress,
  });
}, 1000);
