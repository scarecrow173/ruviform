---
id: "TASK-034XVm2WqXrYsZaBcDeFgH"
type: "task"
status: "in-progress"
title: "Issue #629 MIDI loopback round-trip verifier implementation task"
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
        path: "docs/tasks/issue-629-midi-loopback.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-629-midi-loopback.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-midi-loopback.md"
    - "docs/specs/issue-629-midi-loopback.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-midi-loopback.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-midi-loopback.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-629-midi-loopback.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 MIDI loopback round-trip verifier implementation task

## Work

`midi_endpoint_probe` を #658 merge 後の実装に合わせて refresh し
（production output port view・verdict 修正・WMS availability 検出）、
`crates/ruviform-midi-device/tests/loopback.rs` に provisioned pair
前提の round-trip verifier（`#[ignore]`-gated）と GS Wavetable
silent-send smoke を実装する。audio-test `midi.rs` の stale doc
comment と survey docs 3 件を修正し、evidence と docs index を揃える。
Issue #629 の 3rd slice のみ — 実 provisioning・`virtual-os-endpoint`
evidence・reconnect/xrun・WASAPI・Test Runtime 結線・CI lane・
consumer 照合は後続 slice。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-629-midi-loopback.md`（approved）
- `docs/designs/issue-629-midi-loopback.md`（approved）
- `docs/plans/issue-629-midi-loopback.md`（approved）
- `docs/test-specs/issue-629-midi-loopback.md`（approved）
- `docs/discovery/issue-629-virtual-endpoint-survey.md`（gap #1・E2 crash）
- `docs/specs/issue-629-midi-output.md`（2nd slice chain — output port 契約）
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
- `POLICIES.md`

## Dependencies

- `ruviform_midi_device::backend()` → `SystemBackend`（input/output
  両 enumerator — read-only 参照）
- `ruviform_audio_core::midi_input` / `midi_output`（port trait —
  read-only 参照）
- `ruviform_media_core`（`MidiEvent`/`MidiKind`/`MidiValue`/
  `TimedInputEvent`/`TimestampProvenance`）
- `tests/ports.rs::real_device_open_read_observe_close`・
  `tests/output_ports.rs::real_device_send_smoke`（`#[ignore]`
  real-device lane 先例）
- `mise.toml` `test-midi-real-device`（`--ignored` 自動収集 — 変更不要）

## Affected Files

- `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs`（refresh）
- `crates/ruviform-midi-device/tests/loopback.rs`（新設）
- `crates/ruviform-midi-device/src/os/windows_output.rs`（coordinator
  承認の scope 拡張 — #658 latent `midiOutOpen`/`MIDI_IO_STATUS`
  defect 修正 + `send_sysex` の `Unsupported` degrade）
- `crates/ruviform-midi-device/src/lib.rs`（degrade の doc 反映）
- `crates/ruviform-midi-device/tests/output_ports.rs`（degrade 受理）
- `crates/ruviform-audio-test/src/midi.rs`（doc comment のみ）
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
- `docs/designs/issue-629-virtual-endpoint-survey.md`
- `docs/test-specs/issue-629-virtual-endpoint-survey.md`
- `docs/{specs,designs,plans,tasks,test-specs}/issue-629-midi-loopback.md`
- `docs/impl/{ir,exp}/issue-629-midi-loopback.md` + `*-exp.jsonl`
- `evidence/rf629/`（検証記録）
- `docs/*/README.md`（doc-maintenance / sync_impl_index 再生成）

## Done When

- [ ] probe が production output port view を表示し、verdict が
  「send path は実装済み、残 gap は provisioned pair」を正しく述べ、
  WMS availability を報告する。Windows 実機再実行で evidence 更新。
- [ ] `tests/loopback.rs` の `loopback_round_trip` が pair 存在時に
  Note/CC/PitchBend/NoteOff の内容・順序を独立期待値と照合し、
  timestamp domain/provenance/ticks を観測・記録する。
- [ ] pair 未 provision 環境では verifier が fail-closed の NotRun を
  記録し、`virtual-os-endpoint` Pass を主張しない。
- [ ] `real_device_silent_output_smoke` が GS Wavetable Synth 名指しの
  非可聴 send（PC/CC/NoteOff）で実 WinMM 通過を確認する
  （device 不在は NotRun）。
- [ ] audio-test `midi.rs` の stale doc comment が #658 後の実装に
  一致し、survey docs で gap #1 resolved・probe 出力契約
  （output view）が記録される。
- [ ] Issue #629 の残受入は本 slice 対象外として docs に明記。

## Verification

- [ ] `mise run fmt-check` — Pass
- [ ] `mise run check ruviform-midi-device` — Pass
- [ ] `mise run lint ruviform-midi-device` — Pass
- [ ] `mise run test ruviform-midi-device` — Pass
- [ ] `mise run check|lint|test ruviform-audio-test` — Pass
- [ ] `mise exec -- prek run --all-files` — Pass
- [ ] Windows 実機 probe 再実行 + `test-midi-real-device` 実機実行の
  evidence を `evidence/rf629/` に記録
- [ ] `test-midi-real-device` lane / MSRV lane — MSRV は NotRun
  （daily CI 管轄）。real-device lane は実機で実行し結果を記録
