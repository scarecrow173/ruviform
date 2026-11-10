---
id: "SPEC-rf253vst3midicanonpath001"
type: "spec"
status: "approved"
title: "Issue #253 VST3 MIDI 正規経路 slice（IMidiMapping CC→param 配送 + plugin→host reporting）"
created: "2026-11-12"
updated: "2026-11-12"
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
    - "docs/impl/ir/issue-253-midi-backend-fixture-and-vst3-translation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/plans/vst3-host-bridge.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/impl/ir/issue-253-midi-backend-fixture-and-vst3-translation.md"
  derived-by:
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-vst3-midi-canonical-path.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/317"
  defers:
    - "実 OS MIDI backend（Windows/macOS/Linux の port 列挙・open・native timestamp・device loss）と opt-in 実デバイス lane、実 DAW VST3 往復 — Issue #253 残スコープとして別 slice"
    - "MIDI2/UMP/MPE の generic capability — Issue #317"
  deferred-by: []
metadata:
  issue: 253
---

# Issue #253 VST3 MIDI 正規経路 slice

## 背景とスコープ

PR #537（`9de1b2f4`）で VST3 MIDI の双方向イベント翻訳と `MockBackend`
headless slice が入ったが、Issue #253 の判定コメントが残した P1 gate のうち
本 slice は次の 2 件だけを対象とする。

- **(b) `IMidiMapping` 正規経路**: VST3 `IMidiMapping` 経由の
  CC→Parameter 正規配送。controller 相当の interface を fixture が実装し、
  host bridge が非 RT 経路で assignment を snapshot、RT 経路で
  `inputParameterChanges`/`IParamValueQueue` に配送する。
- **(c) plugin→host reporting**: `ProcessData.outputEvents` /
  `outputParameterChanges` の null 配線を解消し、plugin 出力を
  canonical `Event` として host 側 consumer へ届ける。

対象外（Issue #253 に残る・本 slice では Pass に繰り上げない）:

- (a) 実 OS MIDI backend（port 列挙/open/native timestamp/device loss）
- (d) opt-in 実デバイス・実 plugin 検証 lane
- 実 DAW VST3 往復
- V9/V11/V12/V13 follow-up、MIDI2/UMP/MPE（Issue #317）

## 要件

### R1 IMidiMapping 正規経路（host→plugin）

- fixture controller（`GainController`）が `IMidiMapping` を実装し、
  少なくとも 1 つの deterministic mapping（CC→gain `ParamID`）を持つ。
- host bridge は `IMidiMapping` を `init` で解決し、assignment table を
  `prepare`（非 RT）で snapshot する。RT `process` 経路で plugin の
  `getMidiControllerAssignment` を呼ばない。
- CC / channel pressure / pitch bend の canonical `EventKind::Midi` は、
  mapping が存在すれば正規化値を対応 `ParamID` の queue へ配送する
  （JUCE `StoredMidiMapping` + `associateWith` の参照実装に倣う）。
- mapping が無い MIDI イベントは従来どおり `inputEvents` の
  `kLegacyMIDICCOutEvent` / note event として配送する（unmapped fallback）。
- frame offset は保存する。map 命中だが queue が無い・満杯の場合は
  defined skip として計数し、event 経路への fallback は行わない
  （JUCE の consumed semantics に一致）。
- mapping snapshot は prepare 時点のもの。`restartComponent` の
  `kMidiCCAssignmentChanged` 駆動 refresh は本 slice の defined boundary
  （handler は現行どおり `kNotImplemented` を返し、mapping は prepare
  ごとに再 snapshot される）。

### R2 plugin→host reporting

- host は `ProcessData.outputEvents`（第 2 の `IEventList`）と
  `outputParameterChanges`（書き込み可能な `IParameterChanges`）を
  `process` に供給する。
- plugin 側 component は `GainProcessor` が queue した report を
  `outputParameterChanges.addParameterData`/`addPoint` と
  `outputEvents.addEvent` 経由で書き込む。
- host は process 復帰後に出力 queue/event を drain し、canonical
  `Event`（`ParamValue` / `Midi`）として `PluginProcessor` の新しい
  reporting drain surface に蓄積する。
- 全経路は bounded: queue point・event・report はいずれも prepare 時に
  capacity 確保し、溢れは defined drop + 計数。RT 経路で allocation・
  blocking・I/O を行わない。
- neutral contract: `ruviform-plugin-core` に driver-neutral な
  plugin→host reporting 面を追加する（`ProcessResult` 変更ではなく
  drain 方式 — 設計判断は design doc / IR に記録）。

### R3 capability / 記録

- `ruviform-audio-plugin-bridge::cap` の VST3 `param_reporting` を
  実装結果に合わせて更新する（fixture in-process E2E 証明済みだが
  L1 wire / 実 DAW 未検証のため `Declared` が上限）。
- `midi_channel_voice` は `Declared` のまま（real-plugin/実 DAW 未検証）。
- impl record に neutral contract 追加の判断根拠・検証結果・残 NotRun を
  記録する。

## 受入条件（本 slice）

- fixture cdylib を in-process dlopen する E2E で、CC→`IMidiMapping`→
  `inputParameterChanges`→plugin processor への normalized 値到達を証明。
- unmapped CC が従来の MIDI event 経路に残ることを証明。
- plugin→host `outputEvents`/`outputParameterChanges` が canonical
  `Event` として drain できることを証明。
- bounded overflow・undefined/unsupported 入力の defined 挙動をテスト。
- canonical 検証 lane（`mise run` 群）が実行・記録される。
