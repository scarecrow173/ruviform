---
id: "TASK-9cTXJ0IYeniJ7ZmRBuYibG"
type: "task"
status: "done"
title: "Issue #565-T1: WASAPI-native input path — frame_position 供給"
created: "2026-11-11"
updated: "2026-11-11"
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
    - "docs/plans/issue-565-wasapi-input-frame-position-plan.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-565-wasapi-input-frame-position.md"
    - "docs/designs/issue-565-wasapi-input-frame-position-design.md"
  blocks: []
  related:
    - "docs/test-specs/issue-565-wasapi-input-frame-position.md"
---

# Issue #565-T1: WASAPI-native input `frame_position`

## 作業項目

- [ ] `crates/ruviform-audio-device/Cargo.toml`: `windows` dep（cfg windows、0.62、必要 features）
- [ ] `src/lib.rs`: `#[cfg(windows)] mod wasapi_input;`
- [ ] `src/wasapi_input.rs`: COM guard、endpoint 解決、IAudioClient 初期化、worker thread、packet loop（`device_position`/`qpc_position` → `InputCallbackInfo`）、`PositionTracker`、`DefaultDeviceMonitor` 相当、`WasapiInputStream` port impl、Drop
- [ ] `src/device.rs`: `CpalDevice` fields（`host_name`/`default_input`）+ `open_input_stream` native 分岐
- [ ] `src/host.rs`: `CpalDevice::new` 新 signature 対応
- [ ] `src/stream.rs`: `instant_to_media_time` pub(crate) 化 + doc 修正
- [ ] `tests/real_input_e2e.rs`: WASAPI `frame_position` Some/連続性 assert
- [ ] 単体 test: `PositionTracker`、`packet info` 構築、`map_hresult`、`rt_` non-alloc（Windows 限定 compile）

## 完了条件

- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `build` / `test-msrv` / `lint-audio-real-input` が pass（実機 lane は実行可否で判定）
- headless で `frame_position` 伝播・`ClockDiscontinuity` 写像・RT non-alloc を検証
