// ipc ベンチページ（AC4 メッセージ境界計測）の薄い配線。
// 状態機械は Rust 側 exp/ipc.rs が所有し、このページは
// `ready` 通知 → `__poc.start` 受付 → 測定実行 → `done`/`bench_error` 報告
// と 1s 周期の `alive` ハートビートを行う（vanilla render ページと同じ構造）。
// Rust → JS 方向の受け口は __poc.pong / __poc.push / __poc.pushDone。

import { IpcBench, installApi, postMessage } from "@ui-poc/bench-shared";

const hud = document.getElementById("hud") as HTMLElement;

const bench = new IpcBench({
  hud,
  onDone: (stats) => postMessage({ type: "done", stats }),
  onError: (message) => postMessage({ type: "bench_error", message }),
  post: postMessage,
});

installApi({
  start: (cfg: unknown) => void bench.start(cfg),
  pong: (seq: number) => bench.onPong(seq),
  push: (seq: number) => bench.onPush(seq),
  pushDone: () => bench.onPushDone(),
});

// ページロード到達の通知（Rust 側 start 送信のトリガ）。
postMessage({ type: "ready", href: location.href, ua: navigator.userAgent });

// alive ハートビート: Rust 側の liveness 観測 + idle 検出（start 再送駆動）。
setInterval(() => {
  postMessage({
    type: "alive",
    state: bench.currentState,
    phase: bench.currentPhase,
  });
}, 1000);
