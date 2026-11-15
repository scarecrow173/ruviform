---
id: "PLAN-RF628sBscripted0000003"
type: "plan"
status: "approved"
title: "Issue #628 実装計画 — scripted port-trait backend"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-628-scripted-backend.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-628-scripted-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/designs/issue-628-scripted-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 実装計画 — scripted port-trait backend

## Approach

`crates/ruviform-audio-test` に `backend` module を追加する。公開 API は
`src/backend.rs`、内部実装は `src/backend/` 配下に分割する:

- `src/backend.rs` — module doc・公開型の re-export・`ScriptedBackend`・
  `ScriptedDeviceHandle`・script 型（`DeviceSpec`/`OutputSpec`/
  `InputSpec`/`StreamScript`/`OpenFaults`/`StreamFault`/
  `StreamDirection`/`PumpReport`）。
- `src/backend/stream.rs` — `ScriptedOutputStream`/`ScriptedInputStream`・
  `StreamSlot`/`OutputCore`/`InputCore`・pump 実装。
- `src/backend/midi.rs` — `MidiFeed`（`MidiScenario` → `TimedMidiInput`）。

`DeviceShared`/`ScriptedDevice`（`Device` impl）は `backend.rs` 内に置く
（script 型と同じ公開面の裏側で密結合のため）。

## Work Items

### W1: Script 型 + Backend/Device/Handle 骨格

- `DeviceSpec`/`OutputSpec`/`InputSpec`/`StreamScript`/`OpenFaults`/
  `StreamFault`/`StreamDirection`/`PumpReport` を `src/backend.rs` に定義。
  公開 enum は `#[non_exhaustive]`。
- `ScriptedBackend`（`DeviceEnumerator` impl: enumerate error・default
  index・vanish フィルタ）・`add_device`（検証付き）・
  `ScriptedDeviceHandle`（vanish/reappear/fault 注入/pump 委譲/
  capture 参照）。
- `ScriptedDevice`（`Device` impl: name/id/format/supported/open 検証
  順序・`DeviceBusy` slot 管理）。

### W2: Stream 実装 + pump

- `src/backend/stream.rs` — `OutputStream`/`InputStream` impl、
  open 時検証、`start`/`stop`/`format`/`buffer_size`/`now`/
  `clock_domain`/`*_latency`、manual pump（timestamp/frame_position/
  capture/playback 構築、PCM fill、capture、fault 適用、Xrun gap）。
- `Drop` で slot の `dead` 化 + device slot 解放（ptr_eq）。

### W3: MIDI 薄 adapter

- `src/backend/midi.rs` — `MidiFeed::new`/`drain_due`/`pump_into_port`/
  `skipped`。`frame_base` への `convert_exact`、負値/不能 cue の
  `skipped` 計上、`(ticks, source_sequence)` sort。

### W4: テスト

- `tests/backend.rs` — enumerate/default/vanish、format/buffer/
  latency/clock_domain 報告、input PCM 再生（oracle 照合）、output
  capture（oracle 照合）、timestamp/frame_position 単調性、
  start/stop/再 open、DeviceBusy、Send 境界、MIDI feed/pump。
- `tests/backend_faults.rs` — enumerate/open/describe fault、
  StreamFault 全種、gap→Xrun、Invalidate→reconnect、FramePosition
  jump+ClockDiscontinuity、CaptureNone、Frames cycle、start/stop
  fault、permission/queue-full mapping、MIDI inject_dropped/
  fail_next_open。
- `tests/rt_contract.rs` — `rt_` 追加: output/input の callback 発火
  経路 alloc-free（`record_output` off・capture off で測定）。

### W5: Docs/evidence/検証

- crate `README.md`・`src/lib.rs` doc 更新（公開 API・正直な制限・
  test-only 境界・本 slice 分の harness 範囲明文化）。
- `docs/tasks/issue-628-scripted-backend.md`・
  `docs/test-specs/issue-628-scripted-backend.md` 作成。
- `docs/impl/ir/`・`docs/impl/exp/`（+`.jsonl`）作成、
  `evidence/rf628/` に検証記録、docs index 再生成
  （`node scripts/sync_impl_index.js --kind ir|exp --apply`）。
- `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`prek` 実行・
  記録、`review-protocol` レビュー、commit/push/PR。

## Ordering

W1 → W2 → W3 → W4 → W5。W2 が最大の作業単位。各 W 完了時に
`cargo check -p ruviform-audio-test`（補助証拠）で早期検出する。

## Risks

- port 契約の細部（`AudioBufferViewMut::try_new` の signature、
  `BufferSize`/`SupportedBufferSize` の variant、`InputStamp` field）
  は実装時に対象 file を再確認して合わせる。
- `PcmScenario` segment fixture の channels/sample_rate と open format
  の一致検証 — `PcmFixture` の accessor 名を確認。
- `assert_no_alloc` 計測中に `Mutex` poison/lock 順序で panic しない
  こと — pump 内で device lock を callback 中に取らない設計
  （必要 field は open 時に core へ複製）。
