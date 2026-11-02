---
id: "TASK-034Qx4SABRy9cBHiKXWlcL"
type: "task"
status: "done"
title: "Issue #187-T2: cpal adapter input surface"
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
  depends-on:
    - "docs/tasks/issue-187-t1-input-port-contract.md"
  blocks:
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
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/187"
  deferred-by: []
---

# Issue #187-T2: cpal adapter input surface

## Work

`ruviform-audio-device`（cpal）に T1 の入力 contract の非 RT 表面を実装する。実 `build_input_stream` plumbing は本タスク対象外 — `open_input_stream` は `Err(Unsupported)` を返す。

- `host.rs`: `Host::input_devices()`/`default_input_device()` → `cpal::HostTrait::input_devices`/`default_input_device` を `CpalDevice` wrap（output と同型の `collect_devices`/default wrap）。
- `device.rs`: `CpalDevice::default_input_format()` → `inner.default_input_config()` を `f32_format`/`synthesize_default_format` 経由で mapping。`supported_input_formats()` → `inner.supported_input_configs()` を `map_supported_input_range` で mapping。`open_input_stream` → `Err(DeviceError::Unsupported)`。
- `convert.rs`: `map_supported_input_range(SupportedInputConfigRange) -> Option<SupportedInputFormat>`（`map_supported_range` と同 F32 変換規則を `SupportedInputFormat` へ写像）。
- `CpalDevice` の `Device` impl が新 method を全て実装。`CpalOutputStream` は `OutputStream` default method に委譲（無変更）。
- cpal 型・feature を公開 API に露出しない（`SupportedInputConfigRange` は `convert` 内部）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-issue-187-input-port-headless-poc.md`
- `docs/designs/issue-187-audio-input-port-capture-clock.md`

## Dependencies

- `docs/tasks/issue-187-t1-input-port-contract.md`（input contract 定義）

## Affected Files

- `crates/ruviform-audio-device/src/host.rs`
- `crates/ruviform-audio-device/src/device.rs`
- `crates/ruviform-audio-device/src/convert.rs`

## Done When

- [ ] `Host::input_devices`/`default_input_device` が `CpalDevice` を返す（headless host でも compile）
- [ ] `CpalDevice` が `default_input_format`/`supported_input_formats`/`open_input_stream`（`Unsupported`）を実装
- [ ] `map_supported_input_range` が F32-capable range のみ `Some` を返す
- [ ] `open_input_stream` が `Err(DeviceError::Unsupported)`（capture 偽装なし）
- [ ] cpal 型を公開 API に露出していない

## Verification

- [ ] `mise run check`/`lint`/`test` green（`CpalDevice` object safety・`open_input_stream` → `Unsupported` を unit/integration test で確認）
