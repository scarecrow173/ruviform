---
id: "TSPEC-034Qx4POsptkCvTP9Kdtxr"
type: "test-spec"
status: "approved"
title: "Issue #187 Input Port contract tests"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/187"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
    - "docs/adr/issue-187-input-port-surface-shape-same-clock-duplex-via-clock-domain-correlatio.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #187 Input Port contract tests

## Purpose

T1-T3 の実装が spec の受入条件を満たすことを検証する。主検証面は `crates/ruviform-audio-device/tests/mock_input_backend.rs`（deterministic scripted fake + bounded SPSC handoff PoC）と audio-core の unit/doctest。実機 capture・HW latency は対象外（unverified として記録）。

## Test Matrix

| ID | 対象 | 検証内容 | 実装箇所 |
| --- | --- | --- | --- |
| C-1 | `AudioBufferView::try_new` | `channels == 0`・`channels*frames` overflow・`len` 不一致を `InvalidValue` で拒否・正常 view の shape | `buffer.rs` doctest + unit test |
| C-2 | `capture_stamp()` | `capture: Some` → `InputStamp{ticks,base:DEVICE_CLOCK_TIMEBASE,domain,provenance}`。`capture: None` → `None`。tick が `u64` に収まらない → `None` | `device.rs` unit test |
| C-3 | `DeviceError::ClockDiscontinuity` | `Copy`・`Display`・`#[non_exhaustive]` 維持 | `device.rs` unit test |
| C-4 | `OutputStream` default | `clock_domain`→`UNKNOWN`・`output_latency`→`Ok(None)`（既存実装無変更で compile） | `mock_backend.rs` 既存回帰 |
| M-1 | deterministic capture | fake が script 通りの 2ch+ block を `AudioBufferView` 経由で届け、内容・順序・frames が一致 | `mock_input_backend.rs` |
| M-2 | timestamp/frame position | `callback`/`capture` instant・`frame_position` が script 値と一致・epoch 内非減少・無損失時 `frames` 進行 | 〃 |
| M-3 | clock-domain 相関 | 同一非 `UNKNOWN` domain 一致で同一 clock 判定。`UNKNOWN` 同士・異 domain は非一致 | 〃 |
| M-4 | latency 報告 | `input_latency` が `StreamLatency{frames,coverage,provenance}` を返し `Ok(None)` 面も表現可能 | 〃 |
| M-5 | bounded handoff | view→pool block copy→`filled` `try_push` が全 block 順序通り非 RT drain で検証・recycle で pool 回復 | 〃 |
| M-6 | queue pressure | `filled` 満杯で overflow 計数（block しない）。`free` 枯渇で非ブロック | 〃 |
| M-7 | error 注入 | `Xrun`/`StreamInvalidated`/`ClockDiscontinuity`/`UnsupportedConfig`/`Unsupported` が error callback/open result で観測 | 〃 |
| M-8 | stream lifecycle | `start`/`stop`/`format`/`buffer_size`/`now`/`clock_domain`/`input_latency` の method 契約 | 〃 |
| RT-1 | alloc-0 callback | `assert_no_alloc` 窓内で callback→`try_new` view→pool copy→`try_push` が alloc 0 | `rt_` test |
| RT-2 | 満杯でも非ブロック | RT 窓内で `filled` full・`free` empty の script でも非ブロック終了 | `rt_` test |
| A-1 | cpal 列挙 | `Host::input_devices`/`default_input_device` が `CpalDevice` を返す（headless 環境で compile+run） | `mock_input_backend.rs` 内 cpal 側 test または `mock_backend.rs` 回帰 |
| A-2 | cpal `open_input_stream` | `Err(DeviceError::Unsupported)` を返す（capture 偽装なし） | 〃 |
| A-3 | cpal format mapping | `supported_input_formats`/`default_input_format` の mapping compile（実機 query は headless では空/エラー可） | 〃 |
| A-4 | dev-dep 分離 | `cargo tree -p ruviform-audio-device --edges normal` に `ruviform-rt` 不在 | manual CI check |
| R-1 | object safety | `Box<dyn Device>`/`Box<dyn InputStream>`/`InputDataCallback`/`InputErrorCallback` が dyn-compatible | compile-time |
| R-2 | 既存 output 回帰 | `mock_backend.rs` 全 test 非破壊・`play_sine`/`list_devices` compile | `mise run test` |

## RT Contract Assertions

- `rt_input_callback_*` test が `assert_no_alloc` 窓内で `InputDataCallback` を駆動し、view 構築・planar copy・SPSC `try_push` の alloc 0 を検証。
- `assert_no_alloc` の使用は既存 `rt_` test と同 pattern（global allocator counter）。

## Deterministic Fixtures

- capture block: `frames ∈ {4, 8}`・`channels = 2`・sample pattern `sample(ch, f) = (ch * 10_000 + f) * 0.001` で deterministic — (ch, f) ごとに値が異なり、転置・ずれを検出可能。
- timestamp: `callback`/`capture` instant は `MediaTime` の script 値（`capture = callback - input_latency` の関係を script で保持）。
- `frame_position`: script 開始値 + 各 block `frames` 進行。`ClockDiscontinuity` 注入時は position jump + error を script。

## Out of Scope / Unverified

- 実 cpal `build_input_stream`・実 HW latency 計測・実同一 clock duplex 駆動（`open_input_stream` → `Unsupported` を以て明示）。
- `map_stamp_to_frame` の engine frame 写像そのもの（`capture_stamp()` の `InputStamp` 形状のみ検証・engine 統合は後続）。
- `DuplexStream`・異 clock drift 補正・engine session 統合。

## Acceptance Traceability

| Spec AC | Test |
| --- | --- |
| 入力列挙・describe・open additive | C-4, R-1, A-1, M-8 |
| capture timestamp→frame | C-2, M-2 |
| 同一 clock 上の複数 track | M-3（domain 判定）+ M-5（複数 block） |
| latency/first-frame 契約 | M-4, M-2 |
| RT bounded・alloc-0 handoff | RT-1, RT-2, M-5, M-6 |
| 失敗面（定義済みのみ） | M-7, A-2 |
| additive・output 非破壊 | R-2, C-4 |
| 未対応を型で表現・偽装なし | M-4（`Ok(None)`）, A-2, M-3（`UNKNOWN`） |
