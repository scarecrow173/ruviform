---
id: "TSPEC-O0zY9nWrOb2qwg92Z8nFZb"
type: "test-spec"
status: "approved"
title: "Issue #251 Stage W closeout test spec — 実機 lane 検証項目"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/251"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-251-stage-w-real-device-closeout.md"
    - "docs/designs/issue-251-stage-w-real-device-closeout-design.md"
  implemented-by: []
  depends-on: []
  blocks: []
  related:
    - "docs/test-specs/issue-565-wasapi-input-frame-position.md"
    - "docs/test-specs/issue-187-input-port-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-251-stage-w-real-device-closeout.md"
    - "docs/designs/issue-251-stage-w-real-device-closeout-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #251 Stage W closeout test spec: 実機 lane 検証項目

対象 lane は `tests/real_input_e2e.rs`（`#![cfg(feature = "real-device")]` + `#[ignore]` の
opt-in。入口は `mise run test-audio-real-input`）。本 spec は Issue #251 Stage W
（Windows+WASAPI）の実機受入項目と lane 内検証行の対応を規定する。

## Feature: input stream open→capture→handoff の実機動作

### Rule T1-1: capture test は delivery 契約の実機観測を assert する

`real_input_stream_captures_planar_audio` が open→start→planar callback→stop で
callbacks>0・frames>0・channel/domain 一致・timestamp 単調・non-finite=0 を assert する
（既存行）。WASAPI ではさらに `frame_position == Some` が全 callback につき、
position gap 数 ≤ `ClockDiscontinuity` 数（Issue #565 行）。

### Rule T1-2: 実効 input latency を実測記録する

`timestamp.capture == Some` の callback では `callback - capture` が実効 input latency
（buffering 経路の観測値）であり、min/max/mean を summary に記録する。
`capture > callback` の順序違反は契約違反として `assert_eq!(0)`。
latency 絶対値は環境依存のため assert しない — 実測値を正直に記録する。

- **Example**: 48 kHz・10 ms period で `callback - capture` が ~10–30 ms の値をとることを
  観測記録する。0 以下や桁外れ（秒級）は発見として記録する。

### Rule T1-3: `input_latency()` の `StreamLatency` 契約を実機観測する

WASAPI-native path の `input_latency()` は `Ok(Some(StreamLatency))` であり、
`frames`/`coverage`/`provenance` を lane 出力に開示する（`GetStreamLatency` 由来、
`coverage = Unknown` ラベル — 成分内訳不明な値を consumer に無条件加算させないための
port 設計）。cpal 経路では `Ok(None)`（portable query 非存在の正直応答）。
`output_latency()` も開示し、`Ok(None)` であれば「unsupported は型で表現」の記録とする。

## Feature: real same-clock duplex

### Rule T1-4: 同一 clock domain を実測する

input/output stream が同一 `ClockDomainId` を報告し `same_clock_domain` の verdict を
表示する（既存行の実走記録。WASAPI では `MONOTONIC` で `true` を期待）。

### Rule T1-5: 非同一 domain の rejection/fallback 判定を観測する

WASAPI 上では非同一 domain pair は構成不能（全 stream が `MONOTONIC`）のため、
contract 判定を lane 内で実測する: `same_clock_domain(domain, UNKNOWN) == false`、
`same_clock_domain(UNKNOWN, UNKNOWN) == false` — `UNKNOWN` 側に倒れる
rejection/fallback verdict を記録する。実 hardware の非同一 pair は `NotRun` と記録する。

## Feature: format 不一致の明示検証

### Rule T1-6: port negotiation が拒否する config は `Err(UnsupportedConfig)`

`real_input_open_rejects_unsupported_configs` が実機の supported 空間に対し
以下を投げ、全て `Err(DeviceError::UnsupportedConfig)` を assert する。
いずれも backend 未到達の port 内判定で deterministic。

- 非 F32 sample format（例: `S16`）。
- 全 F32 range の max を超える / min を下回る sample rate。
- いずれの range の `channels` にも一致しない channel layout（存在する場合）。
- `BufferSize::Fixed(0)`。
- bounded range のみの場合、`Fixed` > max bound（`Unknown` bound 報告時は skip）。

## Feature: device 切断 → typed error 通知

### Rule T1-7: 物理抜去で `StreamInvalidated` が error callback に届く

`real_input_device_removal_notifies_typed_error`（対話式・env var 二重ゲート）:
`RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS` 設定時のみ open+start→抜去指示表示→
window 内の error callback 観測。WASAPI では抜去 →
`AUDCLNT_E_DEVICE_INVALIDATED`/`OnDeviceRemoved` の 2 経路いずれからも
`DeviceError::StreamInvalidated` に正規化される。到達を assert する。
window 内に来なければ `NOT-EXERCISED` として pass（matrix では未実施記録）。
付随の `ClockDiscontinuity`/`Xrun` と post-removal の `stop()`/`now()` 応答は
実測記録（assert しない — worker 退出競合と device 非依存 query のため）。

### Rule T1-8: 未設定時は手順を表示して SKIP

env var 未設定では手動手順を `println!` で表示して pass（証拠 matrix では `NotRun`）。
標準 lane run を stalls させないための二重ゲート。

## Feature: xrun/underrun 観測

### Rule T1-9: 発生しなければ「観測なし」と正直に記録

capture test の `XRUNS`/`ERROR_EVENTS`/`LAST_ERROR_KIND` 計数を summary に出し、
発生 0 は「観測なし」として記録する — 発生させるための負荷操作は lane の責務外。

## 検証方法

| 層 | 方法 |
| --- | --- |
| headless 回帰 | `mise run test` / `test-rt`（mock/headless が green を維持） |
| lane ターゲット lint | `mise run lint-audio-real-input`（`-D warnings`、real-device feature 下の全 target） |
| 実機 | `mise run test-audio-real-input` — T1-1〜T1-6・T1-9 を実走記録。device 非接続時は loud `SKIP` で証拠行は `NotRun` |
| 実機（対話） | `RUVIFORM_REAL_INPUT_UNPLUG_WINDOW_SECS=<n>` で T1-7 を実施。未実施は手順記録つき `NotRun` |
| Stage M/L | 本 slice 対象外 — `NotRun` のまま matrix に残す |
