# Issue #628 deterministic test runtime core — 実装実験記録（2026-11-14）

`docs/tasks/issue-628-deterministic-test-runtime.md` の first slice
（deterministic core）実装で試行・観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-628-deterministic-test-runtime.md`](../ir/issue-628-deterministic-test-runtime.md)、
イベントログは
[`issue-628-deterministic-test-runtime-core.jsonl`](issue-628-deterministic-test-runtime-core.jsonl)
を参照。

## 試行 1: PCM `fill` の planar slice 比較（test bug）

- **目的**: position independence（R2）を `fill(0, N)` と
  `fill(512, M)` の bit 一致で検証する。
- **観測**: 初回テストは `b`（ch0+ch1 の 512 frame 連続）を `a` の
  末尾 1024 sample と直接比較し、Sine で失敗。原因は planar layout
  の slice 設計ミス（`a[ch*1024+512 .. ch*1024+1024]` が正しい対応）。
- **結論**: 実装ではなくテスト側の比較を channel 毎に修正。全 pattern
  で bit 一致を確認。

## 試行 2: `usize::from(u32)` の不存在

- **観測**: `usize::from(frames)`（`frames: u32`）が E0277。
  `From<u32> for usize` は std に存在しない（u8/u16 のみ）。
- **結論**: `frames as usize`（64-bit target で損失なし、clippy も
  通過）に変更。`u16 → usize` は `usize::from` を維持。

## 試行 3: `assert_no_alloc` の `#[global_allocator]` 要件

- **観測**: `static ALLOC: AllocDisabler` のみでは計測が無効
  （dead_code warning + 実際には allocator 未差替え）。
- **結論**: 先例（`mock_input_backend.rs` 等）どおり
  `#[cfg(debug_assertions)] #[global_allocator]` を付与。
  `rt_contract.rs` で clock read / PCM fill / `run_callback`+sink の
  4 test が `mise run test-rt` の `rt_` filter で実際に収集・通過した
  ことを出力で確認。

## 試行 4: `ScheduledEvent::new` の generation/ticks 既定値

- **観測**: `ScheduledEvent::new` は `position_ticks: 0`・
  `plan_generation: 0` を stamp する（実 ticks は
  `TransportPlanBuilder::event` が設定）。直接 `SpanSink::span` を
  呼ぶ rt test では ctx 側を generation 0 に合わせる必要があった。
- **結論**: production plan 経路（builder 経由）では問題なし。
  sink 直叩き test はその制約をコメントで明記。

## 試行 5: clippy cast lint への対応

- **観測**: deterministic spec の IEEE-754 演算（f64 phase → f32
  sample、u64→f64 変換）に `cast_precision_loss` /
  `cast_possible_truncation` / `cast_sign_loss` が集中。
- **結論**: 安易な `#[allow]` ではなく reason 付き `#[expect]` を
  対象関数に局所付与（spec であることを理由として記述）。
  `unnecessary_fallible_conversions` は `usize::from`/`as` で解消。
