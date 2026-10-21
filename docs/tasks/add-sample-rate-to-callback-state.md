---
id: "TASK-034JOukDzGSLe1s0mtlzeg"
type: "task"
status: "todo"
title: "DataCallbackState への sample_rate 配線"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-audio-device/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-audio-device/src/device.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-issue-59-chunk-timestamp.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-issue-59-chunk-timestamp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-issue-59-chunk-timestamp.md"
  derived-by:
    - "synthesize-per-chunk-timestamps.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/59"
  defers: []
  deferred-by: []
---

# DataCallbackState への sample_rate 配線

## Work

1. `stream.rs::DataCallbackState` に `sample_rate: u32` フィールドを追加し、`new` の引数に追加
2. `device.rs::open_output_stream` の `DataCallbackState::new` 呼び出しで `negotiated.stream.sample_rate.0`（cpal `SampleRate` の `u32` 値）を渡す
3. `handle` で `write_interleaved` に `self.sample_rate` を渡す（`write_interleaved` シグネチャ変更は T2 と同時）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-59-chunk-timestamp.md](../plans/implement-issue-59-chunk-timestamp.md)

## Dependencies

- 先行タスク: なし（チェーン先頭）
- ブロック対象: [synthesize-per-chunk-timestamps.md](synthesize-per-chunk-timestamps.md)

## Affected Files

- `crates/ruviform-audio-device/src/stream.rs`
- `crates/ruviform-audio-device/src/device.rs`

## Implementation Notes

- `sample_rate` は `negotiated.stream.sample_rate`（cpal `SampleRate` newtype → `.0` で `u32`）
- open 時（非 RT）のみ設定・callback 内で変わらない値型フィールド

## Done Criteria

- [ ] `DataCallbackState` が `sample_rate: u32` を保持
- [ ] `device.rs` で negotiated の `u32` sample_rate を渡している
- [ ] `mise run check` で型エラーなし

## Verification

- [ ] `mise run check` パス

## Completion Notes

<!-- Record outcome, deviations, and evidence after implementation. -->
