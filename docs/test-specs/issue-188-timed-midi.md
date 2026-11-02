---
id: "TSPEC-7XFD0mdqRqdtKwGpmWUa66"
type: "test-spec"
status: "approved"
title: "Issue #188 Timed MIDI — test spec"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-188-timed-midi.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-188-timed-midi-detailed.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #188 Timed MIDI — test spec

## Purpose

Issue #188 の受入条件（AC-3〜AC-8）を検証するテストの規約。Timestamp 付き MIDI 入力→Audio/Plugin 配送契約が、native stamp 破壊・silent drop・誤写像・stuck note・RT 違反・誇大 capability をしないことを固定する。契約が変更されるまで維持。

## Feature

Timed MIDI — native timestamp preservation, clock-domain mapping to `EngineFrame`, bounded live-input merge into `EventList`, note safety (`NoteLedger` + discontinuity release), and honest adapter capability (CLAP translation, VST3 defined drop, L0 pass-through, L1 wire tag).

## Rules

- Rule: `MidiChannel`/`MidiGroup`/`NoteKey` は範囲外値を defined rejection（`MidiRangeError`）し、`MidiKind` の canonical 値（MSB-justified `u32`/`i32`）は MIDI1 7bit/14bit と UMP 16bit/32bit の両方を無損失保持する。
- Rule: `TimedInputEvent<MidiEvent>` は native stamp（`ticks`/`base`/`domain`/`provenance`）を保持し、整数 Tick/PPQN への強制丸めをしない。消費者は stamp をそのまま受け取れる。
- Rule: `map_stamp_to_frame` は anchor delta を `convert` で frame_base 単位へ単回 `NearestEven` 丸めで変換する。`UnmappedDomain`/`EpochMismatch`/`BeforeAnchor` は全て defined rejection で、epoch 跨ぎの減算・暗黙 clamp をしない。
- Rule: `EventKind::Midi` は `Event` wire 形・`EventList` 規約（非減少 offset・`offset<block_frames`・0-frame flush・安定 same-offset）を変えない。wire codec は tag `0`=ParamValue・`1`=Midi で往復同一値を返し、未知 tag は `Malformed`。
- Rule: `merge_call_events` は 2 つの非減少 `Event` 列を安定 merge し、同一 offset の cross-source tie は plan-before-live、同一 `{group,channel,key}` の NoteOff は NoteOn より先に配置される。scratch 不足は `MergeOverflow` defined rejection で silent drop しない。
- Rule: 入力 ring は bounded（`INPUT_RING_CAPACITY`）で、`try_push` 失敗は `InputQueueFull` counter + report を発行する。per-callback drain は `INPUT_DRAIN_MAX` budget を超えない。
- Rule: `NoteLedger` は channel+key で稼働 note を追跡し、容量超過は最古を force-release + overflow flag（drop しない）。`release_all` は全稼働 note の release event を供給する。MIDI1 duplicate-pitch は count 管理。
- Rule: transport 不連続（`TransportGeneration` 変化・Stop・`device_restart`）で active note の release event が offset 0 で供給される（stuck-note fail-safe）。plugin reset/replace 時も同規約。
- Rule: RT 経路（drain・写像・merge・ledger・`EventList` 構築・plugin call）は no-alloc/no-block/no OS I/O・有界で、非RT 準備（anchor 校正・port open/close・ring 構築）と分離される。
- Rule: adapter capability は `cap.rs` が正直に宣言し、`Declared`/`Unknown`/`Unavailable`/`Unsupported` を supported 扱いしない。未対応 kind/精度は counted defined-drop で、暗黙の parameter 変換・消失をしない。

## Examples

- Given: `MidiChannel::new(16)` When: 構築 Then: `Err(MidiRangeError)`。
- Given: `MidiKind::NoteOn{key:64, velocity:MidiValue::from_u7(127)}` と `MidiValue::from_u16(65535)` When: `as_u16` Then: 7bit source は `0xFFFF` に正規化済み（MSB-justified）、16bit source は `0xFFFF` を保持。
- Given: `TimedInputEvent{stamp:{ticks:1000,base:1/1e9,domain:D,provenance:Carried}, event:NoteOn}` と `ClockAnchor{domain:D,stamp_ticks:0,frame:F0,calibration:Verified}` When: `map_stamp_to_frame`（frame_base=1/48000）Then: `frame=F0+48`、`provenance=Carried`。
- Given: stamp domain が anchor と不一致 When: `map_stamp_to_frame` Then: `Err(UnmappedDomain)`、deliver しない。
- Given: plan events `[Param@32, Param@64]` と live events `[Midi@64 NoteOff, Midi@64 NoteOn]` When: `merge_call_events` Then: `[Param@32, Param@64, NoteOff@64, NoteOn@64]`（plan-before-live、NoteOff priority）。
- Given: input ring capacity=4 に 5 push When: 5回目 Then: `Err(QueueFull)` + `input_queue_full` counter++。
- Given: `NoteLedger` に NoteOn{ch0,key60} When: `is_active` Then: true。When: tg 不連続で `release_all` Then: NoteOff{ch0,key60} を供給。
- Given: NoteOn{ch0,key64} → NoteOn{ch0,key64}（duplicate）→ NoteOff{ch0,key64} When: `is_active` Then: まだ active（count=1）、2回目の NoteOff で inactive。
- Given: ledger 満杯で新 NoteOn When: 最古 entry Then: force-release（overflow flag）、新規は登録される。
- Given: backend=Clap When: `capability_for` Then: `note_events=Supported`。backend=Vst3 Then: `note_events=Unavailable`。
- Given: `EventKind::Midi(NoteOn{key:60,velocity})` When: CLAP `push_events` Then: `NoteOnEvent{velocity:f64}` が入力 list に追加される。VST3 `fill_param_queues` Then: counted drop、param queue 不変。

## Guarantees

- 入力イベントの native stamp・型・順序が Mock input→PluginNode→L0 plugin で保証される。
- 写像の定義済み拒否（UnmappedDomain/EpochMismatch/BeforeAnchor）が全て検出・可観測化される。
- block 境界・Loop・tempo change・重複 timestamp・NoteOff priority・0-frame flush の挙動が再現可能に検証される。
- overflow（ring・ledger・merge scratch）が counter/fault で可観測化され silent drop がない。
- transport 不連続時の stuck-note fail-safe（release_all→offset 0）が検証される。
- adapter 差分（CLAP translation・VST3 defined drop・cap honesty）が検証される。
- RT 経路が `assert_no_alloc` で allocation-free。

## Non-goals

- OS backend 実装・実機検証（Mock input のみ）。
- VST3 `IEventList`/note-expression 実装、CLAP `NoteExpressionEvent`/`Midi2Event`/SysEx。
- L1 wire verb の end-to-end 実端末検証（codec roundtrip のみ）。
- engine 録音 tap・controller chase・SysEx/System 語彙。
- Synvelis 固有 Recording/Take/Overdub/Piano Roll/Undo。

## Risk

- 規約を落とすと: native timestamp 破壊（録音精度喪失）、silent drop（入力喪失・非同期バグの原因）、stuck note（音が鳴り続ける）、RT 違反（audio dropout）、誇大 capability 宣言による consumer の誤判断、merge 順序の非決定性（同一 frame イベントの不定順）。
