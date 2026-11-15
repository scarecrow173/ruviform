---
id: "SPEC-RF628pwpipeline0000001"
type: "spec"
status: "approved"
title: "Issue #628 pipeline wiring — scripted backend into production consumers"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-628-pipeline-wiring.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-628-pipeline-wiring.md"
  depends-on:
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-628-pipeline-wiring.md"
    - "docs/test-specs/issue-628-pipeline-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-628-scripted-backend.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-628-pipeline-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 pipeline wiring — scripted backend into production consumers

## What Should Be Built

`crates/ruviform-audio-test` に **`pipeline` module** を追加し、
2nd slice の scripted port-trait backend（`ScriptedDevice`/`ScriptedInputStream`/
`ScriptedOutputStream`/`MidiFeed`）を **本番消費者契約**へそのまま投入する
3 本の harness 脚を提供する:

1. **Output → Playback**: scripted output device を本番
   `ruviform_audio::Session::open(&dyn Device, OutputStreamConfig, GraphPlan)`
   経由で open/start し、manual `pump_output_next()` で本番 RT callback 駆動
   （`plan.drive`）を実走させる。captured PCM（`record_output`）を独立 oracle
   で Expected と照合する。
2. **Input → Recording-equivalent**: scripted input stream の data callback が
   本番 typed runtime 境界入力 `RtRuntime::input_writer` → `process` →
   `output_reader` へ供給する経路を実走させる。本番 Recording コンポーネントは
   未存在（Issue #187 deferred・Synvelis 側責務）のため、typed runtime 境界を
   優先経路とし、callback→runtime 間の bounded handoff で補完する。
3. **MIDI → Timestamp/Host adapter**: `MidiFeed`/`MockPort` 由来の
   `TimedMidiInput` を本番 `ruviform-audio-plugin-bridge::InputStage` の
   `pump`/`collect_block`（`ClockAnchor` + `map_stamp_to_frame`）へ投入し、
   `Event` 列と counter（late/rejected/dropped）を観測する。

## Why

Issue #628 の中核受入「Audio Input→本番Recording、Output→本番Playback、
MIDI→本番Timestamp/Host adapterをRuviformの既存契約のまま接続し、
Fake-only分岐を製品処理へ侵入させない」を、deterministic backend 上で
OS-L1 検証可能にする。scripted backend 単体（slice 2）では device 境界契約
までしか検証できず、本番 consumer（Session/typed runtime/InputStage）が
実際に scripted device を駆動したときの Expected/Observed 一致・fault 伝播は
未検証だった。

## Acceptance Criteria

- [ ] `Session::open` に `&ScriptedDeviceHandle`（`&dyn Device`）を渡し、
      `OutputStreamConfig` + `GraphPlan` で本番 Session を構築・start できる。
- [ ] Output leg: pump ごとに本番 callback が走り、`record_output` capture と
      Expected PCM（`PcmFixture` oracle 参照、自己参照共用なし）が一致する。
- [ ] Input leg: `open_input_stream` の data callback から bounded handoff を
      経て `RtRuntime::input_writer`/`process`/`output_reader` が駆動され、
      processed PCM が Expected（input PCM × gain）と一致する。
- [ ] MIDI leg: `TimedMidiInput` が `InputStage` の bounded staging →
      `collect_block` を通り、block 内 `Event` offset と `BlockInputStats`/
      cumulative counters が Expected と一致する。
- [ ] fault script（Xrun/ClockDiscontinuity/queue-full/Invalidate/open fault/
      late MIDI 等）が pipeline 経路を通過し、Observed 側に正直に記録される
      （偽成功にしない）。
- [ ] Expected は scenario/fixture から独立に導出し、Observed は本番 consumer
      境界から収集する。`RunMetadata`（`environment_kind=DeterministicBackend`）
      で run を記録できる。
- [ ] pipeline 経路の callback/processing 部分に `rt_` prefix テストがあり、
      `mise run test-rt` に収集され allocation-free 契約を検証する。
- [ ] 製品 crate（audio-core/audio/audio-device/midi-device/plugin-bridge）の
      src を変更しない。test-only 境界を維持する。
- [ ] deterministic backend の Pass を実 OS/実デバイス/実スレッド timing の
      証明と主張しない。到達不能面は NotRun/Deferred として明記する。

## Out of Scope（後続 slice / Deferred）

- `InputStage::attach()`・実 plugin endpoint 配線（Issue #630 runtime
  integration slice と棲み分け）。
- 本番 Recording コンポーネント自体（Issue #187 deferred）。
- 実 OS/実デバイス/実スレッド timing・xrun・duplex・latency の実走証拠
  （EXT-L3）。
- CI lane 統合・consumer oracle（AudioLens/Synvelis）・3OS 実走。
- `crates/ruviform-audio-test/src/midi.rs`・`src/backend*` の変更（他 Worker
  占有 / 前 slice 成果物）。
