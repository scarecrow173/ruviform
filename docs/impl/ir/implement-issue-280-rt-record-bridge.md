---
id: "IMPL-fRkEOw218RGZ6hKMK50kFs"
type: "impl"
status: "completed"
title: "OBS-05 RT固定長 Record Bridge・Clock・欠落・Correlation 統合 (Issue #280)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/280"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/src/record/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/record/clock.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/record/correlation.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/record/event.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/rt_bridge.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-observability/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/tasks/p280-t1-record-vocabulary-clock-correlation-event.md"
    - "docs/tasks/p280-t2-rt-bridge-transport-drop-loss-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-280-rt-record-bridge.md"
    - "docs/tasks/p280-t1-record-vocabulary-clock-correlation-event.md"
    - "docs/tasks/p280-t2-rt-bridge-transport-drop-loss-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/plans/implement-issue-280-rt-record-bridge.md"
    - "docs/test-specs/issue-280-rt-record-bridge-tests.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# OBS-05 RT固定長 Record Bridge・Clock・欠落・Correlation 統合 (Issue #280)

## Summary

`ruviform-observability` に固定長 Record 語彙(`record` モジュール)と
RT→非 RT の bounded SPSC Record Bridge(`rt_bridge` モジュール)を追加した。
RT 経路は `ruviform_rt::channel` を再利用し、alloc 禁止・ロックなし・
bounded を `assert_no_alloc` 窓の rt_ テストで pin した。Clock 統合は
`ruviform-media-core` の `InputStamp`/`ClockDomainId`/`ClockEpochId`/
`TimestampProvenance` をそのまま運び、欠落計測は共有ドロップカウンタ +
`dropped_before` 位置情報、Correlation は `CorrelationToken`/
`CorrelationBook`(非 RT 発行・分類)で実現した。依存方向は
observability → rt / media-core の一方向で、`ruviform-rt →
ruviform-observability` の逆依存は生じていない(ADR-BbCUPCZvdC16eKp8RlfHvT)。

## Context

- 実施タスク: docs/tasks/p280-t1-record-vocabulary-clock-correlation-event.md,
  docs/tasks/p280-t2-rt-bridge-transport-drop-loss-lifecycle.md
- 上流: spec(要求・AC)・ADR(依存方向の決定)・design(Record 語彙・bridge
  契約・RT/non-RT 操作行列・セッション位相)・test-spec(証拠行列)
- 動機: カウンタのみだった観測基盤に固定長イベント輸送を追加し、RT スレッド
  から非 RT 集約側へ alloc・ブロッキング・無制限遅延なしでレコードを運ぶ
  契約を確立する(OBS-07 collector/#289 の下敷き)。

## Implementation

### Changes

- `src/record/clock.rs` — `ClockContext { stamp: InputStamp, epoch:
  ClockEpochId }`。Clock 語彙は media-core 製を再輸出せず参照するのみで、
  変換権は observability に持たせない。
- `src/record/correlation.rs` — `CorrelationToken`(NonZeroU64)、
  `CorrelationLink { token, epoch }`、`CorrelationBook`(非 RT 発行機:
  `issue`/`remint`/`expire`/`classify`)。分類は linked /
  `UnknownToken` / `SupersededEpoch` / `Expired`。
  `correlation: None` は意図的非リンクであり無効リンクと区別する。
- `src/record/event.rs` — `EventId`、`RecordHeader`(event/version/
  source identity/domain generation/correlation/clock/confidentiality/
  reporting impact)、`EventRecord { header, value, reserved }`。
  固定長・`Copy`・生 PCM/信号バッファを構造的に排除。
- `src/rt_bridge.rs` — `bridge(capacity) -> (RtRecorder<R>,
  RecordSink<R>)`。`ruviform_rt::channel` 再利用(独自リング実装なし)。
  `RtRecorder::try_record` は bounded・非ブロッキング・alloc なし、
  フル時は drop-on-full で共有カウンタを加算し、次受理レコードの
  `dropped_before` で位置付き損失として報告する。`RecordSink::drain`
  (FIFO)/`drain_final`(書き手静止後のみ)/`recorded`/`dropped`/
  `drained_total`/`lag()`/`recorded_reader()`/`dropped_reader()`
  (OBS-02 `CounterReader` 互換)を提供。
- `Cargo.toml` — `ruviform-rt`・`ruviform-media-core` への path 依存を
  追加(一方向・既定 feature なし・外部クレート追加なし)。
- `src/lib.rs` — `record`・`rt_bridge` モジュールを export。既存の
  `core`/`source`(OBS-02)API は不変。
- `README.md` — OBS-05 語彙・輸送契約・カウンタ・セッション位相
  (Prepare/Publish/ON/OFF/Retire/FinalDestroy)・RT/non-RT 操作行列・
  依存方向を追記。
- `crates/README.md` — observability の索引行に `record`/`rt_bridge`
  と一方向依存を反映。
- `Cargo.lock` — `cargo` 依存解決による自動更新(手編集なし)。

### Decisions During Implementation

- リングバッファは新規実装せず `ruviform_rt::spsc::channel` を再利用
  (AGENTS.md の重複実装禁止・ADR の葉プリミティブ再利用方針)。
- ドロップしたレコードを通常イベントとして偽装せず、受理側の
  `dropped_before` で「どの受理位置の前に何件落ちたか」を伝える方式に
  した(受理・欠落・lag の三分離: `lag() = recorded - drained_total`)。
- `ReceivedRecord::record`/`dropped_before` は `&self` receiver
  (generic `R` を by-value `self` で消費する const fn は E0493)。
- `CorrelationBook::issue` は `next` の 0 スキップ後に
  `unwrap_or(NonZeroU64::MIN)` で panic 経路を排除
  (clippy `missing_panics_doc` 対応としてではなく contract 単純化)。
- `capacity == 0` は `ruviform_rt::channel` の既存契約どおり panic。
- 最終 drain・終端破棄・CorrelationBook 操作・Source 登録系は非 RT のみ。
  RT 書き手は registry lock・登録ポートに触れない(設計の操作行列どおり)。

## Related Experiments

### Adopted

なし(実験ログは作成していない)。

### Rejected

なし。

## Validation

| 検証 | 実行 | 結果 |
| --- | --- | --- |
| `mise run fmt` (`cargo fmt --all`) | 実行 | Pass |
| `mise run check` (`cargo check --workspace --all-targets`) | 実行 | Pass |
| `mise run lint` (`cargo clippy --workspace --all-targets -- -D warnings`) | 実行 | Pass |
| `mise run test` (`cargo test --workspace`) | 実行 | Pass — 全 test target 緑 |
| `mise run test-rt` (rt_ フィルタレーン) | 実行 | Pass — `rt_bridge::tests` 全件 + `rt_try_record_and_counters_allocate_nothing` を含む |
| `cargo test -p ruviform-observability --all-targets` | 実行 | Pass — unit 60 + doctest 6(補助証拠) |

実施環境: Windows 11 x64 / Git Bash / Rust 1.99.0 (mise pin)・MSRV 1.89
準拠コード・unsafe なし。

主な pinned 証拠(test-spec の証拠行列に対応):

- alloc 禁止・bounded: `rt_try_record_and_counters_allocate_nothing`
  (`assert_no_alloc` 窓で受理パスとドロップパスの両方)。
- overflow/位置付き損失: `full_ring_drops_and_reports_positional_loss`。
- FIFO drain: `bridge_delivers_fifo_with_zero_dropped`。
- lag ≠ loss: `consumer_lag_is_delay_not_loss`。
- カウンタの OBS-02 互換: `counters_survive_as_registrable_readers`。
- ライフサイクル統合: `obs02_lifecycle_retire_final_drain_removed`
  (`SourceState`/`RegistrationLease`/`RetiredSource` 経路)。
- clock/correlation 輸送: `event_record_travels_the_bridge`
  (domain/epoch/provenance/token/epoch の保持)。
- 書き手静止後 drainability: `recorder_drop_leaves_sink_drainable`、
  クロススレッド: `bridge_cross_thread_delivery`、
  `zero_capacity_panics`、`endpoints_are_send`。

## Risks

- 残リスク: 実機/他 OS での RT 計測(OBS-08 #290 の範囲)は未実施。
  `assert_no_alloc` は Linux/Windows の alloc 観測による構造的証拠であり、
  実機オーディオスレッドでの確定遅延証拠は別 Issue の責務。
- `drain_final` は書き手静止の外部保証が前提(契約どおり。呼び出し側の
  セッション管理が静止を保証する)。
- Rollback: `record/`・`rt_bridge.rs`・`Cargo.toml` 依存 2 行・lib.rs
  export 2 行を除去すれば完全に元に戻る(既存 API 非変更・追加のみ)。

## Follow-ups

- OBS-07(#289): 本 bridge の `RecordSink` から収集する collector/
  snapshot 側の実装(本 Issue の非対象)。
- OBS-08(#290): 実機 RT 計測証拠。
- OBS-03(#278) 系: snapshot/trace 変換は Record 語彙の consumer として
  後続で接続(本実装は変換権を持たない)。
