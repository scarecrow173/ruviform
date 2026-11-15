---
id: "IMPL-628SB0000000000000001"
type: "impl"
status: "completed"
title: "Issue #628 scripted port-trait backend (2nd slice)"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "devin-worker"
relations:
  source: []
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-audio-test/src/backend/"
      - type: "file"
        path: "crates/ruviform-audio-test/src/backend.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/backend.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/backend_faults.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/backend_support/"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/README.md"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/rt_contract.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-628-scripted-backend.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-628-deterministic-test-runtime.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-628-scripted-backend.jsonl"
    rejected: []
---

# Issue #628 scripted port-trait backend (2nd slice)

## Summary

`crates/ruviform-audio-test` に `backend` module を追加し、
production port 契約（`ruviform_audio_core::device` の
`DeviceEnumerator`/`Device`/`InputStream`/`OutputStream`）の
deterministic scripted 実装を提供した。callback 発火は test driver
の manual pump（`pump_output_next`/`pump_input_next`）のみ — OS API・
thread・実デバイスなし。

- `ScriptedBackend`（`DeviceEnumerator`）: 固定 device 集合、
  enumerate-level error、default device pin、vanish 除外。
- `ScriptedDeviceHandle`（`Device` + test handle）: `vanish`/
  `reappear`、open fault 注入（persistent/once）、stream fault 注入
  （live core + spec 双方）、manual pump、output capture 参照。
- `StreamScript`/`StreamFault`/`OpenFaults`: frames cycle、latency、
  clock domain、callback-index fault（Error/Frames/FramePosition/
  CaptureNone/Invalidate）、start/stop fault queue。
- `stream.rs`: `OutputCore`/`InputCore` + slot 構造。scratch 再利用、
  `PcmScenario` replay（gap → silence + Xrun）、post-callback capture、
  `VirtualClock` timestamp（callback/playback/capture/frame_position/
  clock_domain を契約どおり報告）。
- `midi.rs`: `MidiFeed` — `MidiScenario` cue を `TimedMidiInput` へ
  stamp 付き変換し `MockPort` へ pump。`inject_dropped`/
  `inject_unsupported`/`fail_next_open` は既存 mock をそのまま利用。

## Design decisions

- **Slot + dead flag**: trait object と test handle の共有状態を
  `Arc<OutputSlot>`/`Arc<InputSlot>`（`Mutex<Core>` + `AtomicBool
  dead`）で橋渡し。invalidate/drop で dead を立て、direction の
  re-open を可能にする。
- **Phased open borrow**: `DeviceShared` の `as_mut`（fault 消費）→
  immutable 読み取り → slot 代入の 3 相で borrow checker と整合。
- **Pump prologue**: dead/invalidated → `StreamInvalidated`、
  vanished → invalidate latch + error callback 1 回通知 + 失敗、
  not-started → `Backend("not_started")`。
- **RT 境界**: callback firing 区間（view 構築・timestamp・callback
  起動・counter 更新）は allocation-free — `rt_` test で
  `assert_no_alloc` 実測。`record_output` capture と pump 自体は
  non-RT test-driver path。
- **`#[non_exhaustive]` 公開 enum**: `StreamFault`/`StreamDirection`
  に付与（repo policy）。`BufferSize`/`SupportedBufferSize` の
  non-exhaustive match は wildcard → `UnsupportedConfig`/reject。

## Validation

canonical 検証（`mise run`、Windows 11 x64・rustc 1.99.1）：

- `mise run fmt-check` — pass
- `mise run check ruviform-audio-test` — pass（`--locked --all-targets`）
- `mise run lint ruviform-audio-test` — pass（`-D warnings`）
- `mise run test ruviform-audio-test` — pass（backend 15 +
  backend_faults 9 + rt_contract 6 + 既存 suite、0 failed）
- `mise run test-rt` — pass（workspace 全体 `rt_` filter;
  `rt_scripted_backend_{output,input}_pump_does_not_alloc` を含む）

証拠: `evidence/rf628/scripted-backend-verification.md`

## Risks

- pump は test driver thread での manual firing — 実 RT thread の
  scheduling/priority は証明しない（OS-L1 の範囲内）。
- `record_output` は pump 内で `Vec` 成長し得る — `rt_` test では
  `record_output: false` で計測。
- `frames_cycle`/`Frames` fault の巨大値は open 時に scratch を
  `max_frames` まで確保（spec validation は `> 0` のみ要求）。

## Follow-ups

- production pipeline 配線・consumer oracle（AudioLens/Synvelis）・
  3 OS 実走・CI lane 追加は Issue #628 後続 slice（`.github/**`・
  `mise.toml` は本 slice 範囲外）。
