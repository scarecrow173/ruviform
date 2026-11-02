---
id: "TASK-034Qx4QQvbZvxt6cuQGGid"
type: "task"
status: "done"
title: "Issue #187-T1: ruviform-audio-core input port contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-187-input-port-headless-poc.md"
  implemented-by: []
  depends-on: []
  blocks:
    - "docs/tasks/issue-187-t2-cpal-input-surface.md"
    - "docs/tasks/issue-187-t3-mock-input-poc.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-187-input-port-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #187-T1: ruviform-audio-core input port contract

## Work

`crates/ruviform-audio-core` に入力 contract を additive に追加する。`device.rs` の output trait パターンを入力方向へ反復する。

- `DeviceEnumerator`: `input_devices()`/`default_input_device()`。
- `Device`: `default_input_format()`/`supported_input_formats()`/`open_input_stream(config, data_cb, err_cb)`。
- `InputStream`: `start`/`stop`/`format`/`buffer_size`/`now`/`clock_domain`/`input_latency`。`Send`。
- `OutputStream`（default method のみ追加）: `clock_domain()`→`UNKNOWN`、`output_latency()`→`Ok(None)`。
- 値型: `InputStreamConfig`/`SupportedInputFormat`/`InputTimestamp{callback: MediaTime, capture: Option<MediaTimestamp>}`/`InputCallbackInfo{timestamp, clock_domain, frame_position}`/`StreamLatency{frames, coverage, provenance}`/`LatencyCoverage{HardwareOnly,HardwareAndBuffering,Unknown}`（`#[non_exhaustive]`）。
- callback: `InputDataCallback = Box<dyn for<'a> FnMut(AudioBufferView<'a, f32>, InputCallbackInfo) + Send>`、`InputErrorCallback = Box<dyn FnMut(DeviceError) + Send>`。
- `InputCallbackInfo::capture_stamp()` — `capture: Some` なら `InputStamp{ticks, base: DEVICE_CLOCK_TIMEBASE, domain, provenance}` を構成し `Some` を返す（`None`/tick overflow → `None`）。
- `DeviceError::ClockDiscontinuity` variant 追加 + `Display`。
- `buffer.rs`: `AudioBufferView::try_new(&'a [S], channels, frames)`（`AudioBufferViewMut::try_new` の immutable 対応版・doctest 付き）。
- 全 API に `**RT contract:**` doc ラベル・`Option`/`UNKNOWN` で「報告不能」を型表現・callback instant を capture へ暗黙補完しない・adapter 合成 `frame_position` 禁止を doc に明記。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-187-input-port-headless-poc.md`
- `docs/designs/issue-187-audio-input-port-capture-clock.md`

## Dependencies

- `ruviform_audio_core` → `ruviform_media_core`（`MediaTime`/`ClockDomainId`/`MediaTimestamp`/`TimestampProvenance`/`InputStamp`/`DEVICE_CLOCK_TIMEBASE`）の既存依存。

## Affected Files

- `crates/ruviform-audio-core/src/device.rs`
- `crates/ruviform-audio-core/src/buffer.rs`

## Done When

- [ ] `DeviceEnumerator`/`Device`/`InputStream`/`OutputStream` が上記契約を持つ（`OutputStream` は default method のみ）
- [ ] 値型・callback type alias が定義され `AudioBufferView<'a, f32>` を `InputDataCallback` に渡す
- [ ] `capture_stamp()` が `InputStamp` を構成し `capture: None`/tick overflow → `None`
- [ ] `DeviceError::ClockDiscontinuity` + `Display` が追加される
- [ ] `AudioBufferView::try_new` が view contract を持ち doctest で検証
- [ ] 全公開 enum `#[non_exhaustive]`・`**RT contract:**` ラベル付き doc

## Verification

- [ ] `mise run check`/`lint`/`test` green（既存 output 回帰含む・object safety は T3 で検証）
