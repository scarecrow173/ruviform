---
id: "IMPL-034XVmf1QyR2KtHQqfxoPa"
type: "impl"
status: "completed"
title: "Issue #629 MIDI loopback round-trip verifier"
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
        path: "crates/ruviform-midi-device/tests/loopback.rs"
      - type: "file"
        path: "docs/specs/issue-629-midi-loopback.md"
      - type: "file"
        path: "docs/designs/issue-629-midi-loopback.md"
      - type: "file"
        path: "docs/plans/issue-629-midi-loopback.md"
      - type: "file"
        path: "docs/tasks/issue-629-midi-loopback.md"
      - type: "file"
        path: "docs/test-specs/issue-629-midi-loopback.md"
      - type: "file"
        path: "docs/impl/ir/issue-629-midi-loopback.md"
      - type: "file"
        path: "docs/impl/exp/issue-629-midi-loopback.md"
      - type: "file"
        path: "docs/impl/exp/issue-629-midi-loopback-exp.jsonl"
    modified:
      - type: "file"
        path: "crates/ruviform-midi-device/examples/midi_endpoint_probe.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/os/windows_output.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-midi-device/tests/output_ports.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/src/midi.rs"
      - type: "file"
        path: "docs/discovery/issue-629-virtual-endpoint-survey.md"
      - type: "file"
        path: "docs/designs/issue-629-virtual-endpoint-survey.md"
      - type: "file"
        path: "docs/test-specs/issue-629-virtual-endpoint-survey.md"
      - type: "file"
        path: "evidence/rf629/midi_endpoint_probe-windows-local.txt"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/tasks/issue-629-midi-loopback.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-629-midi-loopback.md"
    - "docs/plans/issue-629-midi-loopback.md"
    - "docs/specs/issue-629-midi-loopback.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-629-midi-loopback.md"
    - "docs/designs/issue-629-midi-loopback.md"
    - "docs/test-specs/issue-629-midi-loopback.md"
    - "docs/impl/exp/issue-629-midi-loopback.md"
    - "docs/impl/exp/issue-629-midi-loopback-exp.jsonl"
    - "docs/impl/ir/issue-629-midi-output.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-629-midi-loopback.md"
    - "docs/plans/issue-629-midi-loopback.md"
    - "docs/designs/issue-629-midi-loopback.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-629-midi-loopback.md"
  verified-by: []
  references:
    - "evidence/rf629/midi_endpoint_probe-windows-local.txt"
    - "evidence/rf629/midi_loopback_slice3_verification-windows-local.txt"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-629-midi-loopback.md"
      - "docs/impl/exp/issue-629-midi-loopback-exp.jsonl"
    rejected: []
---

# Issue #629 MIDI loopback round-trip verifier

## Summary

Issue #629 の 3rd slice。#658 merge 後に stale になった 1st slice の
`midi_endpoint_probe` を refresh（production output port view・
verdict 修正・Windows MIDI Services availability 検出）し、
provisioned MIDI loopback pair 前提の round-trip verifier を
`#[ignore]`-gated real-device test（`tests/loopback.rs`）として
実装した。audio-test `midi.rs` の stale doc comment と survey
docs 3 件を #658 後の実装に合わせて修正した。

## Context

`docs/tasks/issue-629-midi-loopback.md`（plan
`docs/plans/issue-629-midi-loopback.md`、spec
`docs/specs/issue-629-midi-loopback.md`、design
`docs/designs/issue-629-midi-loopback.md`、test-spec
`docs/test-specs/issue-629-midi-loopback.md`）に従う。
前提: 2nd slice（`scarecrow173/ruviform#658`、merge `7e7270f`）で
`midi_output` port + WinMM `midiOut*` backend が main に存在する。

## Implementation

### Changes

- `examples/midi_endpoint_probe.rs`: `print_port_view` に
  `backend().output_ports()` の同形式 view を追加、
  `print_wms_availability`（`powershell.exe` 経由の
  `Get-Command New-MidiLoopback` + `Get-Service midisrv`、best-effort
  `NotRun`）を追加、verdict を #658 後の実装に合わせて修正、
  module doc を双方向化に更新。
- `tests/loopback.rs`（新設）: `loopback_round_trip`（`#[ignore]`）
  — production input/output enumerator → `info().name` 双方向一致で
  pair 特定 → input 先 open → 固定 channel-voice 4 event
  （NoteOn/CC/PitchBend/NoteOff、ch3、独立リテラル oracle）送信 →
  `read` drain（3s deadline）→ 順序保存部分列一致 + stamp 契約
  （宣言 domain/`Carried`/非減少 ticks）→ close 後
  `StreamInvalidated` 確認 → verdict 出力。pair 未 provision・
  backend 不在・非 Windows は fail-closed `NotRun` early return。
  `real_device_silent_output_smoke`（`#[ignore]`）— `"GS Wavetable"`
  を name に含む output port へ NoteOn なしの非可聴
  PC/CC/NoteOff を send し実 WinMM 通過を確認（device 不在は
  `NotRun`）。
- `crates/ruviform-audio-test/src/midi.rs`: 冒頭 doc comment の
  「production MIDI output port は存在しない」を #658 後の実装に
  修正（sink capture が scheduler 駆動 trace の記録点である理由を
  明確化し、OS loopback verifier への参照を追加）。
- `src/os/windows_output.rs`（coordinator 承認の scope 拡張 —
  #658 latent defect の修正）: `open_at_index` が
  `midiOutOpen` の `MMSYSERR_INVALFLAG` のみを契機に
  `MIDI_IO_STATUS` なし `CALLBACK_FUNCTION` で 1 回 retry。
  fallback 経路では `MM_MOM_DONE` が来ないため `send_sysex` は
  `DeviceError::Unsupported` に正直 degrade（完了待ち不能時に
  driver 参照中の buffer を unprepare する危険を回避）。
  `send`（`midiOutShortMsg`）は completion 不要で無影響。
  module/`lib.rs`/API doc に degrade を明記。
- `tests/output_ports.rs` + `src/os/windows_output.rs` 内
  `real_device_send_smoke`: `send_sysex` の `Ok`/`Unsupported`
  両結果を contract-honest として受理し、観測結果を出力。
- survey docs: discovery の Known Code Gap #1 を resolved
  （merge `7e7270f` 参照）に更新 + Evidence 追記、design/test-spec
  に probe 出力契約（output view・WMS 検出・verdict 文言）を追記。

### Decisions During Implementation

- pair 照合は `info().name` のみ — port id 体系が方向で異なるため。
  name match は candidate であり `virtual-os-endpoint` の証明では
  ない（fail-closed 境界を test-spec R6 で固定）。
- 期待列は `MidiValue::from_u7`/`bend_from_u14` で wire 幅から
  canonical 構築した独立リテラル — `MidiScenario`（scheduler 駆動）
  は port 直接送信に非適合のため再利用せず。
- timestamp 検証は `ports.rs` 先例どおり宣言 domain・`Carried`・
  非減少 ticks。tick 差は driver-domain 観測値として記録するのみで
  send wall-clock との絶対 latency は主張しない。
- `midiOutMessage`+`DRV_QUERYDEVICEINTERFACE` は呼ばない（1st slice
  E2 crash 既知）— verifier は production port API のみ。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-629-midi-loopback.md` +
  `issue-629-midi-loopback-exp.jsonl` — probe refresh・WMS 検出・
  pair 特定・独立 oracle の試行記録、および `test-midi-real-device`
  lane で発覚した `midiOutOpen`/`MIDI_IO_STATUS` の潜在 defect の
  記録（試行 5）。

### Rejected

- なし。

## Validation

- `mise run fmt-check` — Pass
- `mise run check ruviform-midi-device` — Pass
- `mise run lint ruviform-midi-device` — Pass（`-D warnings`）
- `mise run test ruviform-midi-device` — Pass（新規 `#[ignore]` 2 本は
  未実行 — real-device lane 管轄）
- `mise run check|lint|test ruviform-audio-test` — Pass
  （doc comment のみの変更 — 直接影響先として検証）
- `mise exec -- prek run --all-files` — Pass
- Windows 実機 `midi_endpoint_probe` 再実行 — exit 0、
  `evidence/rf629/midi_endpoint_probe-windows-local.txt` 更新
  （production output view・WMS availability・refresh verdict を確認）
- `mise run test-midi-real-device` — 初回 FAILED（exit 101）:
  `midiOutOpen(CALLBACK_FUNCTION|MIDI_IO_STATUS)` が GS Wavetable
  Synth で `MMSYSERR_INVALFLAG`（#658 latent defect、PowerShell
  P/Invoke で `CALLBACK_FUNCTION` 単独 rc=0 を実測）。
  coordinator 承認の scope 拡張で `windows_output.rs` を修正後、
  **再実行 PASS（exit 0）**: lib smoke・`loopback_round_trip`
  （`NotRun — NOT_PROVISIONED`）・`real_device_silent_output_smoke`
  （GS Wavetable へ非可聴 send が実 WinMM を通過 —
  `verdict: Pass`）・`output_ports`/`ports` smoke 全て ok。
  `send_sysex` は fallback 経路で `Unsupported` degrade を実機確認。

## Risks

- name match が同名の無関係 port を pair と誤認し得る — round-trip
  が成立しなければ Fail（正直な失敗）であり false Pass ではない。
  candidate は verdict に全列挙する。
- loopback が event を変換する実装（UMP 翻訳等）では照合が Fail —
  観測列 dump が仕様相違の発見材料になる。
- `MIDI_IO_STATUS` 非対応 device（GS Wavetable Synth 実測）では
  `send_sysex` が `Unsupported` に degrade する — `MM_MOM_DONE`
  completion 付き SysEx はその device では利用不可。`send`
  （short message）は無影響。IO_STATUS 対応 device では従来どおり
  completion wait が動く。

## Follow-ups

- ~~`src/os/windows_output.rs` の `midiOutOpen` flag fallback~~ —
  coordinator 承認（Option A）で本 slice 内に修正済み
  （`MMSYSERR_INVALFLAG` のみ retry + `send_sysex` の
  `Unsupported` degrade）。
- loopMIDI/Windows MIDI Services の実 provisioning と
  `virtual-os-endpoint` evidence — 後続 slice。
- 仮想 endpoint 経由の reconnect/xrun/permission（AC5）・WASAPI
  実走（AC1）・Test Runtime（#628）結線・consumer 照合（AC7）—
  後続 slice / 別 Issue。
