---
id: "SPEC-034XTfa1sD2K4NXS4eKDfn"
type: "spec"
status: "approved"
title: "Issue #629 MIDI output path — WinMM midiOut* backend (2nd slice)"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-629-midi-output.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-629-midi-output.md"
    - "docs/plans/issue-629-midi-output.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-midi-output.md"
    - "docs/test-specs/issue-629-midi-output.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-midi-output.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 MIDI output path — WinMM midiOut* backend (2nd slice)

## What Should Be Built

`ruviform-midi-device` に **MIDI output 送信経路** を新設する。本 slice は
Issue #629 の 2nd slice であり、1st slice（investigation + capability
probe、`docs/discovery/issue-629-virtual-endpoint-survey.md`）が特定した
discovery gap #1「`midiOut*` 送信経路不在」を解消する。

- **`midi_output` port（`ruviform-audio-core`）** — `midi_input` の鏡像と
  して `MidiOutputEnumerator` / `MidiOutputPort` / `MidiOutput`
  （`send`）の 3 object-safe trait と `MidiOutputInfo` を追加する。
  `MidiPortOrigin` / `MidiPortCapabilities` 等の既存共通型は再利用し、
  表現の drift を起こさない。公開 enum は全て `#[non_exhaustive]`。
- **Scripted mock（`midi_output::mock`）** — `midi_input::mock` 先例に
  従い、送信 event の記録（Expected/Observed 照合）と `fail_next_open` /
  `fail_next_send` の故障注入を持つ deterministic fixture。
- **`encode_message`（`ruviform-midi-device`）** — `decode_message` の
  逆写像。canonical `MidiEvent` → `midiOutShortMsg` 用の packed
  status/data triple。pure・allocation-free・RT-safe。channel-voice
  語彙外（`group != ZERO`、将来 variant）は `None`。
- **WinMM output backend（`src/os/`）** — `midiOutGetNumDevs` /
  `midiOutGetDevCapsW` / `midiOutOpen` / `midiOutShortMsg` /
  `midiOutLongMsg` / `midiOutReset` / `midiOutClose` 経路。
  `midiOutMessage` + `DRV_QUERYDEVICEINTERFACE` は **呼ばない**
  （1st slice 実測で crash 既知 — exp E2 記録）。PortKey は
  index+name fallback のみ。非 Windows は `DeviceError::Unsupported`。
- **`SystemBackend` 配線** — input `input_ports()` と対称の
  `output_ports()` を追加。
- **Tests** — contract/mock テスト、`encode`→`decode` round-trip、
  `#[ignore]` の実デバイス smoke（`test-midi-real-device` lane が自動収集）。

本 slice が直接対象とするのは Issue AC2 output 側の前提（WinMM 実
Backend から仮想 MIDI Port へ Note/CC/Pitch Bend を **送信できる**
経路）。loopback send→receive round-trip 自体は loopMIDI/Windows MIDI
Services の provisioning 前提のため本 slice では **NotRun**
（`environment_kind=virtual-os-endpoint` を主張しない）。

## Why Now

- 1st slice の survey が gap #1「`midiOut*` 送信経路不在」を確定し、
  loopback pair 検出（双方向同名）と production view の乖離を記録した。
  send→receive round-trip（AC2）と仮想 endpoint 実走（AC1 関連）の
  前提として、まず本番 Backend 入口の output 側が必要。
- `MidiOutput`/`midi_output` は repo 内に存在しなかった（#628 design
  調査済み）。port を audio-core に置くことで engine/adapter の依存方向
  （adapter → port）を保ち、mock による OS-L1 検証を可能にする。
- 仮想優先 POLICIES（2026-11-15 改訂）では mock/send path の Pass は
  OS-L1 証拠であり、`virtual-os-endpoint` の Pass や実 OS L3 Pass と
  主張しない — 本 slice はその境界を守る。

## Users and Value

- **Ruviform 自身**: MIDI device adapter が双方向になり、engine 側の
  MIDI 送信 consumer（#253 系譜の出力経路）が port 契約越しに接続
  できる。
- **後続 slice（#629 残受入）**: loopMIDI/WMS loopback provisioning 上の
  send→receive round-trip、`midi_endpoint_probe` の output enumeration
  拡張、reconnect/xrun/permission 再現の土台。
- **AudioLens / Synvelis**: MIDI 送信を伴う検証（Synvelis の MIDI 出力
  記録等）の共通 adapter 経路。

## Scope

### In Scope

- `crates/ruviform-audio-core/src/midi_output.rs` + `midi_output/mock.rs`
  （port trait・info/capability 型・scripted mock）と `lib.rs` 公開。
- `crates/ruviform-midi-device/src/midi.rs` の `encode_message` 追加
  （`decode_message` と同格の RT 契約・unit test・round-trip）。
- `crates/ruviform-midi-device/src/os/windows_output.rs`（新設）または
  `os/windows.rs` への追記 — WinMM `midiOut*` backend。
- `os/mod.rs` の `enumerate_output_ports` platform 選択、
  `os/unsupported.rs` の `DeviceError::Unsupported`、
  `system.rs` の `MidiOutputEnumerator` 実装、`lib.rs` doc/export。
- `crates/ruviform-midi-device/tests/` の contract/mock テストと
  `#[ignore]` 実デバイス smoke。
- docs chain（spec/design/plan/task/test-spec/impl ir・exp）と
  `evidence/rf629/` の検証記録。

### Out of Scope

- loopMIDI/Windows MIDI Services provisioning 上の loopback
  send→receive round-trip（`virtual-os-endpoint` evidence — NotRun）。
- `examples/midi_endpoint_probe.rs` の output enumeration 拡張
  （lease 外 — evidence に「probe 拡張は後続」と記録）。
- 仮想 endpoint 経由の reconnect/xrun/permission 再現（AC5 本体）・
  WASAPI 側仮想 endpoint 実走（AC1）。
- Test Runtime（#628）との結線・本番 pipeline 配線・User-mode
  controller/runner。
- 専用 Windows runner の CI/手動タスク統合（`.github/**`・`mise.toml`）。
- `MidiPortOrigin::Virtual` 昇格（仮想 port の API 判別 — ユーザー指定に
  より対象外。origin は実測値 `Unknown` のまま正直に報告）。
- Synvelis #115/#117/#126・AudioLens #127 の consumer 照合（AC7）。
- macOS/Linux の MIDI output backend（`Unsupported` のまま）。
- `MidiEvent` への SysEx/System variant 追加（production vocabulary の
  拡張は別 Issue — `send_sysex` は byte 列をそのまま渡す経路としてのみ
  提供）。

## Requirements

- port trait は `midi_input` の 3 段 lifecycle（enumerator → port →
  open handle）を鏡像し、object-safe・`Box<dyn …>` で交換可能。
- `MidiPortOrigin`・`MidiPortCapabilities`・`DeviceError` 等の既存共通型を
  再利用し、同一概念の表現 drift を起こさない。
- クレート外から到達可能な全公開 enum に enum レベルの
  `#[non_exhaustive]`（`docs/adr/public-enum-non-exhaustive-policy.md`）。
- WinMM FFI は `#[cfg(windows)]` module 内の `#![expect(unsafe_code)]`
  + SAFETY コメント（`os/windows.rs` 先例）。`midiOutMessage` +
  `DRV_QUERYDEVICEINTERFACE` は呼ばない（crash 既知）。
- 3 OS lane を green に保つ: Win32 コードは全て `#[cfg(windows)]` の後ろ、
  非 Windows の output enumeration は `DeviceError::Unsupported`。
- 新規依存・feature 追加は行わない（`windows-sys 0.61` の
  `Win32_Media_Audio`/`Win32_Media_Multimedia` で必要 API は充足）。
- mock/send path の Pass は OS-L1 証拠であり、`virtual-os-endpoint` の
  Pass・実 OS L3 Pass と主張しない（fail-closed）。
- 実行していない検証を Pass と書かない。Skip=Pass にしない。

## Acceptance Criteria

- [ ] `midi_output` port（enumerator/port/output + info/capability）が
  `ruviform-audio-core` に追加され、`midi_input` と対称の契約
  （open は 1 回・close 後は invalidated・非 RT 列挙）を持つ。
- [ ] `midi_output::mock` が送信 event を記録し Expected/Observed 照合・
  `fail_next_open`/`fail_next_send` 故障注入を提供する。
- [ ] `encode_message` が channel-voice `MidiEvent` を packed triple に
  符号化し、`decode_message` との round-trip 恒等（MIDI 1.0 範囲値）と
  語彙外拒否の unit test を持つ。
- [ ] WinMM backend が `midiOut*` で output port を列挙・open・send
  （short/long）・reset・close でき、非 Windows は `Unsupported`。
- [ ] `SystemBackend` が `MidiOutputEnumerator` を実装し、
  `tests/` の contract/mock テストが Pass する。
- [ ] `#[ignore]` 実デバイス smoke が `test-midi-real-device` lane の
  収集対象として存在する（本 slice では NotRun と記録）。
- [ ] 検証記録（`evidence/rf629/`）が Case schema 語彙
  （`environment_kind`・Required/Deferred/NotApplicable ×
  Pass/Fail/NotRun/Unsupported）で正直に記録される。
- [ ] Issue #629 の残受入（round-trip・probe 拡張・reconnect/xrun・
  WASAPI・consumer 照合）は本 slice 対象外として docs に明記。

## Deferred Design Concerns

- loopback round-trip・probe 拡張・reconnect/xrun/permission・WASAPI
  実走・Test Runtime 結線・consumer 照合 — 後続 slice / 別 Issue。
  実体は `docs/designs/issue-629-midi-output.md` の Deferred Design
  Concerns に記録する。

## Sources and Relations

- `scarecrow173/ruviform#629`（本 Issue、open のまま — PR は `Refs`）、
  `scarecrow173/ruviform#627`（Closed — Case Schema v1・
  `environment_kind`・Test Depth の契約元）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`（1st slice 成果
  — capability matrix・gap #1・`midiOutMessage` crash 記録 E2）。
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
  （Case schema・`environment_kind` taxonomy の正本）。
- `POLICIES.md` §2.1（仮想優先の受入・証拠要件）・§5（ローカル検証と
  Daily CI の責務分離）。
- 先例: `crates/ruviform-audio-core/src/midi_input.rs` +
  `midi_input/mock.rs`（鏡像の正本）、
  `crates/ruviform-midi-device/src/midi.rs`（`decode_message`）、
  `crates/ruviform-midi-device/src/os/windows.rs`（WinMM FFI 先例）、
  `crates/ruviform-midi-device/src/system.rs`（`input_ports()` 先例）、
  `crates/ruviform-midi-device/tests/ports.rs`。
