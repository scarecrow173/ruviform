---
id: "TST-rf253vst3midicanonpath001"
type: "test-spec"
status: "approved"
title: "Issue #253 VST3 MIDI 正規経路 slice テスト仕様"
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
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-253-vst3-midi-canonical-path.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
    - "docs/designs/issue-253-vst3-midi-canonical-path.md"
  derived-by: []
  verifies:
    - "docs/plans/implement-issue-253-vst3-midi-canonical-path.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# テスト仕様: Issue #253 VST3 MIDI 正規経路 slice

## Feature: IMidiMapping CC→param 正規配送

### Rule MM-01: fixture controller が IMidiMapping を公開する

- **Example**: `GainController` の `getMidiControllerAssignment` が
  bus 0・任意 channel・CC7 に `ParamID 0` を `kResultTrue` で返し、
  他 controller/範囲外 bus は `kResultFalse`。

### Rule MM-02: mapped CC が inputParameterChanges 経由で plugin に届く

- **Example**: host E2E（fixture cdylib in-process）で
  `ControlChange{controller:7, value:from_u7(64)}` を process に流すと、
  plugin processor が normalized `≈64/127` の gain `ParamValue` を適用し、
  出力 audio が `input × 64/127` になる。frame offset は保存される。

### Rule MM-03: unmapped CC は MIDI event 経路に残る

- **Example**: mapping に無い controller（例 CC10）の `ControlChange` は
  param queue に入らず `kLegacyMIDICCOutEvent` として plugin に届く
  （gain 不変 = audio passthrough、かつ plugin が `EventKind::Midi`
  として受領）。

### Rule MM-04: mapping 非対応 kind は event 経路のみ

- **Example**: `NoteOn`/`ProgramChange` は `IMidiMapping` の lookup 対象外
  で、常に event list 経路。

### Rule MM-05: queue 不在/満杯は defined skip

- **Example**: mapping が declared param に無い ParamID を返す場合、
  event は消費（suppressed）され `skipped` が計数される —
  legacy event への fallback はしない。

## Feature: plugin→host reporting

### Rule PR-01: outputParameterChanges が canonical ParamValue report になる

- **Example**: plugin が gain `ParamValue` を適用した block で、
  host の `drain_output_reports` が `Event{offset, ParamValue{param_id:0,
  value:適用値}}` を返す。

### Rule PR-02: outputEvents が canonical Midi report になる

- **Example**: plugin が `NoteOn` を出力した block で、drain が
  `EventKind::Midi(NoteOn)` を offset・channel・key・velocity 保存で返す。

### Rule PR-03: drain は since-last-drain で bounded

- **Example**: drain 後の再 drain は空。report は
  `MAX_EVENTS_PER_BLOCK` capacity で bounded、溢れは計数 drop。

### Rule PR-04: null output channel は defined skip

- **Example**: host が output ptr を null で渡した場合、plugin は
  report を書かず block は reject されない（skip 計数）。

### Rule PR-05: 不正 report 値/offset は defined skip

- **Example**: plugin が block 外 offset や非有限 param 値を書いた
  場合、host はその point を drop して計数し、他の report は配送する。

## Feature: capability / 非退行

### Rule CAP-01: cap.rs が実装結果を正直に宣言する

- **Example**: Vst3 `param_reporting == Declared`（fixture in-process
  E2E 証明、L1 wire/実 DAW 未検証）、他 backend は `Unavailable` のまま。
  `midi_channel_voice` は `Declared` のまま。

### Rule REG-01: 既存経路の非退行

- **Example**: param event・MIDI event・transport・lifecycle の既存
  host.rs/component.rs/gain.rs テストが全て pass。
