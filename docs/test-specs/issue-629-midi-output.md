---
id: "TSPEC-034XTfGCCaBBZvQJ72kOjc"
type: "test-spec"
status: "approved"
title: "Issue #629 MIDI output path test spec"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/specs/issue-629-midi-output.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-629-midi-output.md"
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
    - "docs/designs/issue-629-midi-output.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-midi-output.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-629-midi-output.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 MIDI output path test spec

## Purpose

`midi_output` port・scripted mock・`encode_message` codec・WinMM
`midiOut*` backend が、input 側と対称の lifecycle 契約・honest な
capability 宣言・Expected/Observed 可能な送信記録を持つことを保証
する。本 slice の対象は send path の存在と契約のみであり、loopback
round-trip・probe 拡張・reconnect/xrun・WASAPI 実走は対象外
（後続 slice）。

## Feature

MIDI Output Send Path — port trait + mock + codec + WinMM backend

## Rules

- **R1 port 契約の鏡像**: `MidiOutputEnumerator::output_ports` /
  `MidiOutputPort::{info,open}` / `MidiOutput::{send,send_sysex,close}`
  は `midi_input` と対称 — open は 1 回（2 回目は `DeviceBusy`）、
  close 後の send は `StreamInvalidated`、全 method は non-RT
  （OS API 呼出・alloc 可）。
- **R2 honest capability**: output port は `supports_ump=false`・
  `provides_timestamps=false`・`clock_domain=None`・
  `origin=Unknown`（WinMM）を宣言する。timestamp を捏造しない。
- **R3 codec 対称性**: `encode_message` は `decode_message` の逆写像
  — MIDI 1.0 範囲値では `decode(encode(e)) == e`、`group != ZERO`・
  語彙外は `None`。値の narrowing は `MidiValue::as_*` の規定写像。
- **R4 Expected/Observed**: mock は送信 `MidiEvent`/SysEx byte 列を
  順序保持で記録し、test が独立した期待列と照合できる
  （被試験コードが期待値を生成しない）。
- **R5 故障注入**: `fail_next_open`/`fail_next_send` は one-shot で
  消費され、retry で回復可能（transient refusal のモデル）。
- **R6 platform 境界**: 非 Windows の `output_ports` は
  `Err(DeviceError::Unsupported)`。Windows の Win32 コードは全て
  `#[cfg(windows)]` 配下。
- **R7 公開 enum**: クレート外から到達可能な全公開 enum に
  `#[non_exhaustive]`。
- **R8 fail-closed 証拠**: mock/codec の Pass は OS-L1 証拠であり、
  `virtual-os-endpoint`・実 OS L3 を主張しない。実デバイス smoke は
  `#[ignore]` + `test-midi-real-device` lane 収集、本 slice では
  NotRun。

## Examples

- **E1 mock round-trip**: `MockBackend` に port 登録 → `output_ports`
  → `open` → `send(NoteOn/CC/PitchBend)` → `sent_events()` が送信順の
  期待列と一致 → `close` → 以後の `send` は `StreamInvalidated`。
- **E2 故障注入**: `fail_next_open(DeviceBusy)` で 1 回目 open が
  `DeviceBusy`、retry で成功。`fail_next_send(StreamInvalidated)` で
  次の send が失敗し記録されない、後続 send は成功。
- **E3 codec round-trip**: `encode_message` → packed triple →
  `decode_message` で全 channel-voice variant（NoteOn/Off・CC・
  ProgramChange・ChannelPressure・PolyPressure・PitchBend center/
  max/min）が恒等復元。`group != ZERO` は `None`。
- **E4 codec 語彙外/範囲**: 非 0 group、未知 `MidiKind`（将来
  variant）は `None`。canonical 値の narrowing（例: u16 由来の
  velocity）は `as_u7` 結果と一致。
- **E5 WinMM enumerate（Windows）**: `SystemBackend::output_ports` が
  `Ok` を返し、各 port の `info` が `origin=Unknown`・
  `supports_ump=false`・`provides_timestamps=false`・非空 id を持つ
  （device 0 台でも Pass — empty は正常）。
- **E6 非 Windows**: `output_ports` が `Err(DeviceError::Unsupported)`
  （compile-time cfg で検証、実機は CI lane）。
- **E7 実デバイス smoke（`#[ignore]`）**: enumerate → open →
  NoteOn/NoteOff/CC/PitchBend/SysEx send → close。device 不在なら
  正直に early return（eprintln 記録）。`test-midi-real-device` lane
  が自動収集 — 本 slice では NotRun。

## Guarantees

- `midi_output` port が `midi_input` と対称の lifecycle・honest
  capability 契約を持ち、`Box<dyn …>` で交換可能。
- mock が Expected/Observed 照合と one-shot 故障注入を提供し、
  OS なしで send path の契約試験ができる。
- `encode_message`/`decode_message` が round-trip 恒等と語彙外拒否を
  unit test で証明する。
- WinMM backend が `midiOut*` で列挙・open・send・close でき、
  `midiOutMessage`+`DRV_QUERYDEVICEINTERFACE` を呼ばない。
- 非 Windows は `Unsupported` で fail-closed、3 OS lane の compile/
  test が green。

## Non-goals

- loopback send→receive round-trip・`virtual-os-endpoint` 証拠・
  `midi_endpoint_probe` 拡張・reconnect/xrun/permission・WASAPI・
  Test Runtime 結線・CI lane 統合・consumer 照合 — 後続 slice。
- `MidiPortOrigin::Virtual` 昇格・Windows MIDI Services endpoint id。
- `MidiKind`/`MidiEvent` への SysEx/System variant 追加
  （`send_sysex` は byte 経路のみ）。
- timestamp 付き output（WinMM に存在しない — 捏造しない）。

## Risk

- mock が input mock と責務を混同して drift する → module doc・
  test で Expected/Observed 契約を明示。
- `send_sysex` の `MHDR_DONE` 未到達で `DeviceBusy` が続く →
  `close`/`midiOutReset` の done 回収契約に依存（design 記録）。
- codec narrowing が想定外の値を生む → `MidiValue::as_*` の
  round-to-nearest を test で固定値検証。
- `#[cfg(any(windows, test))]` で非 Windows test build に codec が
  現れ dead code/lint になる → `pub(crate)` fn は test 内使用で
  dead_code 回避、lint lane で確認。
