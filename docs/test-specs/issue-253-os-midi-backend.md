---
id: "TESTSPEC-rf253osmidibackend001"
type: "test-spec"
status: "approved"
title: "Issue #253 OS MIDI input backend test specification"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-253-os-midi-backend.md"
    - "docs/designs/issue-253-os-midi-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-253-os-midi-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-os-midi-backend.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-253-os-midi-backend.md"
    - "docs/designs/issue-253-os-midi-backend.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# Test Specification

Feature: OS MIDI input backend（Windows WinMM + 非対応 OS Unsupported）

## Feature 1: message decode（pure）

- Rule F1.1: `dwParam1` の status/data byte を channel-voice
  `MidiEvent` に正しく写像する（NoteOn/NoteOff/CC/ProgramChange/
  ChannelPressure/PolyPressure/PitchBend、channel・key・value の
  妥当性）。
  - Example: NoteOn ch3 key60 vel100 → `MidiEvent::note_on(3,60,100)`。
- Rule F1.2: 非 channel-voice（0xF0+ system、0xF1–0xF7
  common/realtime、status<0x80）は `None`（= `unsupported` 計数
  対象）を返す。
  - Example: status 0xF8（timing clock）→ None。
- Rule F1.3: data byte >0x7F 等の不正値は panic せず `None`。
  - Example: controller=200 → `MidiController::new` 失敗 → None。

## Feature 2: bounded queue / stats（pure）

- Rule F2.1: capacity 内の push は alloc なしで event を保持し、
  read が到着順に drain する。
  - Example: 3 event push → read(max=2) で先頭 2 件、残り 1 件。
- Rule F2.2: capacity 超過分は `dropped` に計数され event は
  蓄積されない（サイレント失われない）。
  - Example: capacity=2 に 3 push → dropped=1、event 2 件のみ。
- Rule F2.3: `unsupported` は `dropped` と独立に since-last-read
  で snapshot→reset される。
  - Example: unsupported を 2 回 push → 1 回目 read で 2、
    2 回目 read で 0。
- Rule F2.4: queue が `dead`（MIM_CLOSE 相当）なら read は
  `StreamInvalidated` を返す。

## Feature 3: timestamp stamp 構築

- Rule F3.1: `dwParam2` ticks を `InputStamp` に `TimeBase=1/1000`、
  `ClockDomainId`=subscription 採番 domain、`provenance=Carried`
  で保持する（adapter は `map_stamp_to_frame` を呼ばない）。
  - Example: ticks=1234 → stamp.ticks==1234, base.den==1000。

## Feature 4: Windows 公開面（cfg(windows)）

- Rule F4.1: `SystemBackend::input_ports` は `Ok`（空集合含む）
  を返す。列挙 port の `info()` は id 非空・name 非空・
  `origin=Unknown`・`capabilities`={ump:false, ts:true,
  domain:Some}・port 毎に domain が一意。
- Rule F4.2: 存在しない device id での open は
  `DeviceError::DeviceNotFound`（MMERR_BADDEVICEID 正規化）。
- Rule F4.3: 同一 port handle の 2 回目 open は `DeviceBusy`。
- Rule F4.4: 実 port があれば open→read→close が例外なく動作
  （device 不在環境ではこの test は enumerate 空で skip — Pass
  条件にしない）。

## Feature 5: 非対応 OS 境界（cfg(not(windows))）

- Rule F5.1: `input_ports` は `Err(DeviceError::Unsupported)`。
  全 target でコンパイルが通る。

## Feature 6: 結線

- Rule F6.1: `Box<dyn MidiInputEnumerator>`→`Box<dyn MidiInputPort>`
  →`Box<dyn MidiInput>` の trait-object 経路が型・object safety
  上成立し、`read` が `TimedMidiInput` を返す（`InputTx::pump`
  が要求する契約）。

## NotRun / Unsupported（正直な記録）

- 実 MIDI デバイスでの timestamp 単調性・実イベント受信:
  NotRun（実機 opt-in lane は別 slice）。`#[ignore]` test と
  `examples/list_midi_ports` を用意するが CI Pass 条件外。
- mid-stream device-loss の検出: WinMM 制約で NotRun/制限
  （design 明記）。
- macOS/Linux backend: Unsupported。
