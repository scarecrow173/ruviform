---
id: "PLAN-034Qx4ylQvpVDutx6EqnfJ"
type: "plan"
status: "approved"
title: "Implement Issue #187 Audio Input Port: additive input contract, cpal input surface, mock capture->handoff RT PoC"
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
  implements:
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-187-audio-input-port-capture-clock.md"
    - "docs/adr/issue-187-input-port-surface-shape-same-clock-duplex-via-clock-domain-correlatio.md"
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-188-timed-midi.md"
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

# Implement Issue #187 Audio Input Port: additive input contract, cpal input surface, mock capture->handoff RT PoC Implementation Plan

## Goal

既存 output 中心の `ruviform_audio_core::device` に入力 contract（列挙・format query・`InputStream`・callback・capture timestamp/clock-domain/latency/discontinuity 語彙）を additive に追加し、cpal adapter の非 RT 入力表面と deterministic mock capture→handoff の RT 契約 PoC を実装する。

## Scope

1 実装 stream（T1 契約 → T2 cpal adapter → T3 mock PoC の順序・依存）。実機 input plumbing・`DuplexStream`・engine 統合は含まない。

## Upstream Documents

- `docs/specs/issue-187-audio-input-port-capture-clock.md`
- `docs/adr/issue-187-input-port-surface-shape-same-clock-duplex-via-clock-domain-correlatio.md`

## Design Inputs

- `docs/designs/issue-187-audio-input-port-capture-clock.md`（approved — port 契約表・timestamp/latency 意味論・RT 契約・mock 構造）

## File Map

- `crates/ruviform-audio-core/src/device.rs` — input contract（trait method・`InputStream`・値型・callback・`StreamLatency`/`LatencyCoverage`/`InputTimestamp`/`InputCallbackInfo`・`DeviceError::ClockDiscontinuity`）
- `crates/ruviform-audio-core/src/buffer.rs` — `AudioBufferView::try_new`
- `crates/ruviform-audio-device/src/host.rs` — `Host::input_devices`/`default_input_device`
- `crates/ruviform-audio-device/src/device.rs` — `CpalDevice` 入力 method（format query → mapping・`open_input_stream` → `Unsupported`）
- `crates/ruviform-audio-device/src/convert.rs` — `map_supported_input_range` 等 input mapping
- `crates/ruviform-audio-device/Cargo.toml` — dev-dep `ruviform-rt`
- `crates/ruviform-audio-device/tests/mock_input_backend.rs` — scripted fake + deterministic handoff PoC

## Implementation Sequence

- [ ] T1: `AudioBufferView::try_new` + `device.rs` input contract（`DeviceEnumerator`/`Device` method・`InputStream`・`InputStreamConfig`/`SupportedInputFormat`/`InputTimestamp`/`InputCallbackInfo`/`InputDataCallback`/`InputErrorCallback`/`StreamLatency`/`LatencyCoverage`・`OutputStream` default `clock_domain`/`output_latency`・`DeviceError::ClockDiscontinuity`・`capture_stamp()`）
- [ ] T2: cpal adapter — `Host` input 列挙・`CpalDevice` input format query/`open_input_stream`→`Unsupported`・`map_supported_input_range`
- [ ] T3: `mock_input_backend.rs` — `FakeInput*`・deterministic scripted capture→pool copy→SPSC handoff・`rt_` alloc-0 契約テスト・同一 clock/断絶/error/queue pressure ケース
- [ ] impl record を in-progress で作成し T1-T3 の検証 evidence を記録
- [ ] `git merge origin/main`・生成物（docs index）再生成・`mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 最終検証

## Dependencies and Risks

- `ruviform_audio_core` が `ruviform_media_core`（`ClockDomainId`/`MediaTimestamp`/`TimestampProvenance`/`InputStamp`/`MediaTime`）へ既に依存していることを確認（`map_stamp_to_frame` 権威の再利用）。
- `ruviform-rt` は dev-dep のみ（mock handoff）。adapter normal 依存に混入させない。
- mock は cpal 実装を証明しない — 実 input plumbing・実 HW latency は未検証として記録。

## Task Breakdown

- `docs/tasks/issue-187-t1-input-port-contract.md`
- `docs/tasks/issue-187-t2-cpal-input-surface.md`（depends-on T1）
- `docs/tasks/issue-187-t3-mock-input-poc.md`（depends-on T1・T2 は非必須だが同一 workspace で検証）

## Verification

- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` exit 0（既存 output 回帰含む）
- [ ] 既存 `Box<dyn Device>`/`OutputStream` 実装（`CpalDevice`/`FakeStream`/`StubStream`）が無変更で compile（default method の additive 検証）
- [ ] `rt_` 契約テストで callback→view→copy→`try_push` 経路の alloc 0・`filled` 満杯時 overflow 計数・`free` 枯渇時の非ブロック
- [ ] `cargo tree -p ruviform-audio-device --edges normal` に `ruviform-rt` 不在

## Open Questions / Gaps

- cpal の `InputCallbackInfo::timestamp()` が `callback`/`capture` instant を持つことは cpal ドキュメント上既知だが、実 mapping（`OutputStreamTimestamp` と同 epoch fallback 再利用）の実装・実機検証は後続 Issue。
- `open_input_stream` が `Unsupported` を返す実機差分（cpal `build_input_stream` 非対応 host の列挙結果）は実機未検証。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が逐次実装）
- Independent work streams: T1 → T2 → T3 の依存順（T3 の mock は T1 のみに依存・T2 と並列可だが同一 worker が逐次実施）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `Device` への method 追加は trait 実装の source-breaking（`CpalDevice`・mock fake の同時更新を T1/T3 で実施）。third-party `Device` impl への影響は公開 enum `#[non_exhaustive]` policy と同枠組み（semver-major 期間外の未発表 crate・publish=false）として扱う。
- `OutputCallbackInfo` は pub field 構造体で field 追加不可のため触れない — input 側は別構造体 `InputCallbackInfo` に全 metadata を集約。
