---
id: "TASK-034XTg3GE8Q84dZ1stdcNY"
type: "task"
status: "in-progress"
title: "Issue #629 MIDI output path implementation task"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-629-midi-output.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-629-midi-output.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-midi-output.md"
    - "docs/specs/issue-629-midi-output.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-midi-output.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-midi-output.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-629-midi-output.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 MIDI output path implementation task

## Work

`ruviform-audio-core` に `midi_output` port（`MidiOutputEnumerator`/
`MidiOutputPort`/`MidiOutput`/`MidiOutputInfo`）と `midi_output::mock`
scripted fixture を追加し、`ruviform-midi-device` に `encode_message`
codec・WinMM `midiOut*` backend（`os/windows_output.rs`）・
`SystemBackend::output_ports` 配線を実装する。Issue #629 の 2nd slice
（discovery gap #1「`midiOut*` 送信経路不在」の解消）のみ —
loopback round-trip・probe 拡張・reconnect/xrun・WASAPI・Test Runtime
結線・CI lane・consumer 照合は後続 slice。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-629-midi-output.md`（approved）
- `docs/designs/issue-629-midi-output.md`（approved）
- `docs/plans/issue-629-midi-output.md`（approved）
- `docs/test-specs/issue-629-midi-output.md`（approved）
- `docs/discovery/issue-629-virtual-endpoint-survey.md`（gap #1・E2 crash）
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
- `POLICIES.md`

## Dependencies

- `ruviform_audio_core::midi_input`（鏡像の正本 — read-only、共有型
  `MidiPortOrigin`/`MidiPortCapabilities` を再利用）
- `ruviform_media_core::midi`（`MidiEvent`/`MidiKind`/`MidiValue`）
- `ruviform_midi_device::midi::decode_message`（codec 逆写像の相手）
- `ruviform_midi_device::os::windows`（WinMM FFI 先例）・
  `system.rs`（`input_ports()` 先例）
- `windows-sys 0.61` `Win32_Media_Audio`/`Win32_Media_Multimedia`
  （既存 feature で `midiOut*` API 充足 — 依存追加不要）

## Affected Files

- `crates/ruviform-audio-core/src/midi_output.rs`（新設）
- `crates/ruviform-audio-core/src/midi_output/mock.rs`（新設）
- `crates/ruviform-audio-core/src/lib.rs`（`pub mod midi_output;`）
- `crates/ruviform-midi-device/src/midi.rs`（`encode_message` + test）
- `crates/ruviform-midi-device/src/os/windows_output.rs`（新設）
- `crates/ruviform-midi-device/src/os/mod.rs`（`enumerate_output_ports`）
- `crates/ruviform-midi-device/src/os/unsupported.rs`（output 版）
- `crates/ruviform-midi-device/src/system.rs`（`MidiOutputEnumerator`）
- `crates/ruviform-midi-device/src/lib.rs`（doc/cfg 更新）
- `crates/ruviform-midi-device/tests/output_ports.rs`（新設）
- `docs/{specs,designs,plans,tasks,test-specs}/issue-629-midi-output.md`
- `docs/impl/{ir,exp}/issue-629-midi-output.md` + `*-exp.jsonl`
- `evidence/rf629/`（検証記録）
- `docs/*/README.md`（doc-maintenance / sync_impl_index 再生成）

## Done When

- [ ] `midi_output` port + mock が audio-core に追加され、
  `midi_input` と対称の契約（open 1 回・close 後 invalidated・non-RT）
  を持つ。
- [ ] `midi_output::mock` が送信 event 記録（Expected/Observed）と
  `fail_next_open`/`fail_next_send` one-shot 故障注入を提供する。
- [ ] `encode_message` が channel-voice `MidiEvent` を packed triple に
  符号化し、`decode_message` round-trip 恒等・語彙外拒否の unit test
  を持つ。
- [ ] WinMM backend が `midiOut*` で output port を列挙・open・
  send（short/long）・reset・close でき、`midiOutMessage`+
  `DRV_QUERYDEVICEINTERFACE` を呼ばない。非 Windows は `Unsupported`。
- [ ] `SystemBackend` が `MidiOutputEnumerator` を実装し、
  `tests/output_ports.rs` の contract/mock テストが Pass する。
- [ ] `#[ignore]` 実デバイス smoke が `test-midi-real-device` lane の
  収集対象として存在する（本 slice では NotRun と記録）。
- [ ] Issue #629 の残受入は本 slice 対象外として docs に明記されている。

## Verification

- [ ] `mise run fmt-check` — Pass
- [ ] `mise run check ruviform-audio-core` — Pass
- [ ] `mise run check ruviform-midi-device` — Pass
- [ ] `mise run lint ruviform-audio-core` — Pass
- [ ] `mise run lint ruviform-midi-device` — Pass
- [ ] `mise run test ruviform-audio-core` — Pass
- [ ] `mise run test ruviform-midi-device` — Pass
- [ ] `mise run test-rt` — 影響なし確認
- [ ] `mise exec -- prek run --all-files` — Pass
- [ ] codec round-trip・mock Expected/Observed・WinMM enumerate の
  honest capability をテストで確認
- [ ] `test-midi-real-device` lane / MSRV lane — NotRun（指定どおり）
