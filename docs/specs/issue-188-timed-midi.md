---
id: "SPEC-bg9fb3WGAfULNZMmr9jTMc"
type: "spec"
status: "proposed"
title: "Issue #188 Timed MIDI — input, timestamp and plugin delivery contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/synvelis/issues/12"
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
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/specs/issue-186-param-control-events.md"
    - "docs/specs/issue-189-rational-beat-variable-ppqn-m3.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-188-timed-midi-inventory.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-188-timed-midi-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/synvelis/issues/10"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  defers: []
  deferred-by: []
metadata:
  gate: "M3"
  issue: 188
---

# Issue #188 Timed MIDI — input, timestamp and plugin delivery contract

## What Should Be Built

Timestamp 付き MIDI 入力を Audio/Plugin Process へ**型・時刻・順序を維持したまま配送する最低限の汎用契約**を確定し、不足分を PoC 実装・検証する。DAW 固有の Recording/Take/Overdub/Piano Roll/Undo は Synvelis 所有として持ち込まない。

1. **MIDI 語彙**: Note On/Off・CC・Pitch Bend・Poly/Channel Pressure・Program Change を canonical 値（MSB-justified、MIDI1/UMP 両表現を無損失保持）で表す中立 `MidiEvent`/`MidiKind` と、`MidiChannel`/`MidiGroup`/`NoteKey` validated 型。System/SysEx は拡張余地として定義のみ。
2. **Timestamp / Clock Domain**: 入力イベントが native stamp（`ticks`+`base`+`domain`+`provenance`）を保持する `TimedInputEvent<P>` と、stamp→`EngineFrame` 写像権威（`ClockAnchor` pair、calibration provenance、負 delta/未知 domain/未校正の defined handling）。PPQN960 等への強制 quantize なし。
3. **MIDI Input Boundary**: `audio-core` の port trait パターンに沿った Enumerator→Port→Connection 型（open/close、外部機器/仮想入力/loopback 区別、backend 能力照会、normalized error）。OS backend 実装は比較記録のうえ別 Issue。
4. **Live→Block Pipeline**: input ring(bounded)→per-callback drain→stamp 写像→`EventList` への plan/live 安定 merge。block frame offset・0-frame flush・block 境界・容量超過 fault は #190/#196 と同一規約。
5. **Note Safety**: channel+key 稼働 note を追跡する bounded `NoteLedger` と、transport 不連続（tg 変化・Stop・device restart）時の自動 release/reset 供給。MIDI1 duplicate-pitch の voice 曖昧性は明文化する。
6. **Adapter 差分の正直な宣言**: CLAP Note 系/MidiEvent、VST3 `IEventList`/`LegacyMIDICCOutEvent`、L1 wire tag、L0 pass-through の実装状況を `cap.rs` に反映。未対応 kind/精度は defined drop（count 付き）で暗黙消失しない。
7. **RT 契約**: drain/写像/merge/ledger は no-alloc/no-block/no I/O・有界、非RT 準備（port open/close・anchor 校正・ring 構築・retire）と分離、過負荷は counter/fault で可観測。
8. **実装 Issue 分割案**: PoC で確定した契約を、実 backend adapter・typed TimedEvent port・VST3 event 実装・engine 録音 tap 等へ分割する提案。

## Why Now

Synvelis #12 の P0 MIDI 録音/再生が Ruviform 側の「高精度入力 timestamp + Audio/Plugin 同期配送」契約を必要とする。#189（BeatPosition/TempoMap）・#190（span scheduler/generation）・#186（EventList/defined rejection 先例）・#196（PluginNode/cap matrix）がマージ済みで配送骨格は揃ったが、MIDI 語彙・入力境界・clock domain 語彙・live merge・note safety が未実装。本 Issue が G3 受入証拠（設計 + 最小 PoC + Issue 分割）を出す位置にある。

## Users and Value

- **Synvelis**: 外部/仮想 MIDI 入力の録音、再生時の Note/CC/Bend 配送、Transport 連動の note 安全、adapter 能力に基づく UI 制限表示。
- **audio-lens 等の他 consumer**: 同一の中立契約で MIDI 機器入力を plugin/graph へ接続。
- **Ruviform 自身**: M3 gate（G3/G4）の受入証拠、実装 Issue 分割の正当化。

## Scope

### In Scope

- 棚卸し記録（discovery doc、実コード根拠付き、本書の E 節相当を収録済み）。
- MIDI 語彙・timestamp/Clock Domain・input boundary・merge 規約・note safety・adapter 差分・RT 契約の spec/ADR/design。
- `ruviform-media-core` への `midi`/`timed_input` module（語彙・stamp・anchor・mapper・NoteLedger）。
- `ruviform-plugin-core` への `EventKind::Midi` + `merge_call_events` + wire tag `1`。
- `ruviform-audio-core` への `midi` input port trait（enumerator/port/connection、port kind、backend caps、normalized error）と `MockMidiEnumerator` 実装。
- `ruviform-audio-plugin-bridge` への `InputStage`（bounded ring drain→map→merge→`NoteLedger`→`EventList`）と `NodeControl`/`NodeReport`/status 拡張。
- `ruviform-plugin-clap` の `NoteOnEvent`/`NoteOffEvent`/`MidiEvent` translation + non-ParamValue defined drop。VST3 は capability 宣言 + defined drop のみ（実装は分割 Issue）。
- Mock input→PluginNode→L0 plugin の E2E PoC テスト（`rt_` lane 含む）。

### Out of Scope

- Synvelis 固有: Track Arm、RecordingSession、Take/Comping、Loop Overdub、Quantize、Undo、Project Save Format、Piano Roll、Clip 編集、DAW 固有 input monitor ポリシー。
- OS MIDI backend の実装（midir / CoreMIDI / ALSA / WinMM 等は比較記録のうえ分割 Issue）。
- `PortFormat::TimedEvent` の typed payload 化・graph 配線（M3 後続 Issue）。
- VST3 `IEventList`/note-expression の実装、CLAP `NoteExpressionEvent`/`Midi2Event`/SysEx 転送（分割 Issue）。
- SysEx / System message の wire 化（語彙は拡張余地のみ）。
- 全 Backend/メッセージ型への完全対応保証。
- 汎用 Control Graph / input→graph 配線基盤の新設。

## Requirements

- R1: `MidiEvent` は `MidiChannel`(0..=15)/`MidiGroup`(0..=15)/`NoteKey`(0..=127) の validated 型と、`MidiKind`（NoteOn/NoteOff/CC/PitchBend/PolyPressure/ChannelPressure/ProgramChange + System/SysEx 拡張余地）を持つ。値は MSB-justified canonical 整数で MIDI1 7bit/14bit と UMP 16bit/32bit の両方を無損失保持できる。
- R2: `EventKind` に `Midi(MidiEvent)` を additive 追加（`#[non_exhaustive]` 維持、`ParamValue` 意味・wire 形・`reserved` ポリシー不変）。L1 wire は kind tag `1` を追加し、旧 codec は未知 tag を `Malformed` defined rejection する。
- R3: 入力イベントは `TimedInputEvent<P>{stamp, event}` で native stamp を保持する。stamp は `ticks`/`base`/`domain`/`provenance`（Carried/Derived/Estimated）を持ち、整数 Tick/PPQN への強制丸めを行わない。
- R4: stamp→`EngineFrame` 写像は `ClockAnchor{domain, stamp_ticks, stamp_base, frame, calibration}` を介する単一権威関数が行い、1 回の `NearestEven` 丸めで frame offset を決定する。未知 domain は `UnmappedDomain` defined handling、負 delta は defined clamp または rejection、anchor 未校正は `Estimated` provenance で明示。epoch 跨ぎは減算しない（`ClockEpochId` 不一致は fault）。
- R5: live input は bounded ring（容量上限・`try_push` full 検出）で RT 側へ handoff し、RT 側は per-callback `INPUT_DRAIN_MAX` budget で drain する。full/overflow は counter+fault で可観測にし、silent drop しない。
- R6: plan events と mapped live events は `merge_call_events` で安定 merge し `EventList` に入る。merge 規約は中立明文化: 同一 frame offset では plan-before-live（または同等の deterministic 規則）で、各 source 内の causal order を保持する。Note-Off は同一 frame・同一 channel+key での Note-On より先に配置される（Note-Off priority）。merge scratch 不足は defined fault（既存 `EventScratchOverflow` 拡張または相当）。
- R7: 0-frame flush・block 境界・`offset<block_frames`・非減少 offset 等の `EventList` 規約は live input にもそのまま適用される。block 境界上のイベントは現行規約（次ブロック先頭）に従い、時刻の一意性を保つ。
- R8: `NoteLedger` は channel+key で稼働 note を追跡する bounded 構造体で、RT で no-alloc。Note-On で登録・Note-Off で解除・容量超過は最古強制 release+fault。MIDI1 duplicate-pitch（同一 channel+key の重複 Note-On）の voice 曖昧性は明文化する。
- R9: transport 不連続（`TransportGeneration` 変化・`Stop`・`device_restart`）時は `NoteLedger` が active note の release events を offset 0 で自動供給する（stuck-note fail-safe）。CC64/AllNotesOff/AllSoundOff の chase 語彙を定義し、plugin reset/replace 時も同規約を適用する。
- R10: audio-core `midi` port trait は Enumerator（`input_ports`/`default_input_port`）→Port（`name`/`id`/`kind`/`capabilities`/`open`）→Connection（`close`/data callback/入力 stamp 供給）の 3 層で、`DeviceError` 相当の normalized error と `MidiPortKind{External,Virtual,Loopback,Unknown}` 区別を持つ。backend 能力は `MidiSupport{Unknown,Unsupported,Supported}` 相当の三値で、Unknown は supported 扱いしない（capability honesty）。
- R11: adapter capability は note/channel event 対応・MIDI1/2・Note Expression・SysEx の差分を `cap.rs` に明示する。未対応 kind/精度への変換要求は defined drop（count 付き）で、暗黙の parameter 変換/消失をしない。
- R12: RT 側処理（drain・写像・merge・ledger・EventList 構築・plugin call）は no-alloc/no-block/no OS I/O・有界。port open/close・anchor 校正・ring 構築・snapshot publish・retire は非RT。`rt_` 契約テストは `assert_no_alloc` 測定窓。
- R13: 公開 enum は `#[non_exhaustive]`、公開 API 例は doctest、全 public API に doc。
- R14: `TransportPlan<EventKind>`/`ScheduledEvent`/`TransportScheduler`/`NodeControl`/`NodeReport`/`TransportInfo`/`PortFormat`/`SignalDomain`/`ParamValue` 意味・`EventList` wire 形は不変。`AdapterCapability` への field 追加は publish=false workspace-internal として許容し IR に記録。
- R15: 消費者影響分類（synvelis/audio-lens: impacted/no impact/unknown）を記録し、検証不能は `unverified` と明示する。

## Acceptance Criteria

- [ ] AC-1: 棚卸し記録が `docs/discovery/issue-188-timed-midi-inventory.md` に存在し、論点 1〜8 ごとの不足と DAW 固有部分の分離を含む。
- [ ] AC-2: Clock Domain/stamp/provenance/anchor・merge 規約・note safety・adapter 差分・RT 契約が実 API に基づき仕様記述される（design doc）。
- [ ] AC-3: Mock input→`PluginNode`→L0 mock plugin で Note On/Off・CC・PitchBend が期待型・順序・frame offset で届くテストが存在する。
- [ ] AC-4: 実 Sample Clock 系統（`OutputTimestamp`/`ClockAnchor` 経由）での入力時刻→block frame offset 決定・block 境界・Loop・tempo change・重複 timestamp（同時刻安定順）・Note-Off priority・0-frame flush の挙動を再現するテストが存在する。
- [ ] AC-5: input ring overflow・unmapped domain・ledger overflow・merge scratch overflow・plan/live merge の欠落可視化が counter/fault で検証されるテストが存在する。
- [ ] AC-6: transport 不連続（tg 変化・Stop・device restart）時の `NoteLedger` release/reset・stuck-note 予防のテストが存在する。
- [ ] AC-7: 非RT 準備（anchor 校正・ring 構築・port open/close）/RT 有界処理（drain・写像・merge・ledger・配送）の PoC が `rt_` lane に整合し `mise run test-rt` が green。
- [ ] AC-8: adapter cap（note/channel event 対応・MIDI1/2・Note Expression）の宣言と、CLAP `NoteOnEvent`/`NoteOffEvent`/`MidiEvent` への translation（または defined drop）のテストが存在する。
- [ ] AC-9: `EventKind`/`TransportInfo`/`Graph`/`Control`/`ParamValue`/`EventList`/`ScheduledEvent` の互換維持と、実装 Issue 分割案が design/ADR に記録される。
- [ ] AC-10: 後方互換と機能不足一覧が discovery doc（E8/E10）に更新済み。
- [ ] AC-11: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` が green。consumer 影響分類（synvelis/audio-lens: impacted/no impact/unknown）を記録。

## Deferred Design Concerns

- OS MIDI backend 実装（midir / CoreMIDI / ALSA / WinMM）— PoC の Mock input が受入条件のため、実 backend 選定は比較記録（ADR）のうえ分割 Issue。
- `PortFormat::TimedEvent` typed payload 化・graph 配線（M3 後続）。
- VST3 `IEventList`/`NoteOnEvent`/`LegacyMIDICCOutEvent` 実装、note-expression。
- CLAP `NoteExpressionEvent`/`Midi2Event`/SysEx 転送、output events（plugin→host note 系）。
- L1 wire の MIDI kind tag `1` 配線・version negotiation・capability verb。
- engine 層の録音 tap（TimedInputEvent を消費者へ投影する観測 API）。
- `NoteLedger` の controller chase 拡張（CC64/全 CC reset、plugin preset restore 時の state re-send）。
- MIDI2/UMP の per-note controller・16bit velocity の全精度対応（canonical MSB-justified 値で両表現を保持するが、規格側写像は adapter が判断）。
- Synex / System message の語彙・wire 化。

## Sources and Relations

- Issue: ruviform#188（要件正本）、synvelis#12（consumer Recording 要求）、synvelis#10/#18/#20/#22（Timed Event/Note Safety/Native Timestamp 設計要求）、ruviform#184/#185/#186/#189/#190/#193/#196。
- 棚卸し: `docs/discovery/issue-188-timed-midi-inventory.md`（derives-from 元）。
- 前提契約: #190 spec/ADR（generation/span/receipt/EngineFrame）、#189（BeatPosition/TempoMap/PPQN）、#186（EventList/defined rejection/cap 先例）、#196（PluginNode/cap matrix/PlanVault）、境界 ADR（ownership split）。
