# Issue #631 RealOsGuiTest window-close/DPI 観測面 — 実装実験記録（2026-11-16）

`docs/tasks/issue-631-window-surface.md` の実装で試行・観測した事項の
記録。実装結果の正本は
[`docs/impl/ir/issue-631-window-surface.md`](../ir/issue-631-window-surface.md)、
イベントログは
[`issue-631-window-surface-exp.jsonl`](issue-631-window-surface-exp.jsonl)
を参照。

## 試行 1: close 応答の適用タイミング（armed queue）

- **目的**: `Veto`/`Defer` を `RealOsGuiTest` 経由で実現する。
- **観測**: `WindowDriver::request_close` は `CloseRequested` emit と
  同じ同期 flush 内で unanswered request を default `Approve` する。
  caller 側が `respond_close` を `UiJob` で後追い送信しても、既定
  Approve が先に適用され必ず負ける。
- **結論**: `arm_close_response(window, decision)` で per-window FIFO
  の `CloseDecision` queue を caller thread から直接 push し、app
  thread の event handler が `CloseRequested` 分岐で同期的に消費・
  `drv.respond_close` を呼ぶ設計にした（`examples/
  real_window_lifecycle.rs` の既存 veto 経路と同型）。これにより
  `Veto`/`Defer` が既定 `Approve` に確実に先行する。

## 試行 2: 観測 log の分離（`Action` 除外）

- **観測**: `WindowLifecycle` は `Action(Intent)` を含むが、観測面に
  intent を混ぜると lifecycle/geometry 遷移の記録として不純になる。
- **結論**: `WindowLifecycleEvent` を `Action` を除く専用
  `#[non_exhaustive]` enum とし、intent は既存 intents channel に
  残す。`observe()` は `Action` で `None` を返す。

## 試行 3: 並列実行干渉説の検証と棄却

- **観測**: 初回 `cargo test --test real_window_lifecycle` で両 test が
  `scenario command timed out — gpui app thread unresponsive`。
  既存 xvfb lane は parallel real-window test の干渉を文書化している
  ため並列実行を疑ったが、`--test-threads=1` でも同様に失敗。
- **結論**: 並列干渉説を棄却。真因は「最終 window の destroy で GPUI
  platform loop が終了し UiJob pump が死ぬ」ことで、target 破棄後の
  `settle_until`/`window_geometry` が reply を得られず timeout して
  いた。baseline 既存 story test は同環境で pass し、環境起因でも
  なかった。

## 試行 4: sentinel window による pump 生存保証

- **観測**: sentinel window を残した構成に変更すると、並列・serial
  両方で 2 passed / 0 failed（約 4.5s）に安定。
- **結論**: sentinel window は flaky 回避の workaround ではなく、
  GPUI の last-window-exit アーキテクチャに対する構造的必須条件。
  test-spec/design に恒久的な契約として記録し、将来の変更で外れない
  ようにした。

## 試行 5: `arm_close_response` の receiver 規約

- **観測**: 当初 `&mut self` を検討したが、本 API は共有
  `Arc<Mutex<…>>` のみを触り UI round-trip を伴わない。
- **結論**: `window_focused`/`native_slot_*` 等の local query API と
  同じ `&self` receiver に揃えた。`request_close`/`respond_close`/
  `window_geometry` は app thread 操作のため `UiJob` 経由の `&self`
  （内部で `call`）とした。
