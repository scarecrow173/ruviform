---
id: "DSN-rf253vst3midicanonpath001"
type: "design"
status: "approved"
title: "Issue #253 VST3 MIDI 正規経路 slice 設計（IMidiMapping snapshot + plugin→host reporting）"
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
  implements:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/vst3-host-bridge.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-253-vst3-midi-canonical-path.md"
  derived-by:
    - "docs/plans/implement-issue-253-vst3-midi-canonical-path.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-253-vst3-midi-canonical-path.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  defers: []
  deferred-by: []
metadata:
  issue: 253
---

# 設計: IMidiMapping 正規経路 + plugin→host reporting

参照実装: JUCE `VST3PluginFormat` の `StoredMidiMapping` /
`MidiEventList::associateWith`（mapping を UI スレッドで snapshot し
RT 側は table lookup、mapped controller を `inputParameterChanges`
へ流し unmapped は legacy event のまま残す）。

## R1 IMidiMapping（host→plugin）

### fixture controller

`GainController` の `Class::Interfaces` に `IMidiMapping` を追加し、
`getMidiControllerAssignment(busIndex, channel, controller, id)` を実装:
bus 0・全 channel・`kCtrlVolume`(7) → `GAIN_PARAM.id`(0) を
`kResultTrue` で返し、それ以外は `kResultFalse`。`IMidiMapping2`
（MIDI2/UMP 系）は Issue #317 scope のため実装しない。

### host bridge

- `Vst3Plugin::init` で `controller.cast::<IMidiMapping>()`（失敗時
  `component.cast` に fallback — JUCE と同じ component→controller
  両対応の探索順を逆にした spec 優先順: `IMidiMapping` は
  `IEditController` の拡張なので controller 優先）。
- `prepare`（非 RT）で `MidiAssignmentTable::capture` が
  16 channel × `kCountCtrlNumber`(130) の `getMidiControllerAssignment`
  を走査し `[u32; 16*130]`（`kNoParamId` = unmapped）を構築。
  `self.midi_map: Option<Box<MidiAssignmentTable>>`。
- RT `process` では table lookup のみ — plugin 側 COM 呼出しなし。

### MIDI 振り分け（`crate::midi`）

`control_assignment(midi) -> Option<(channel, ctrlNumber, normalized)>`:

| `MidiKind` | ctrlNumber | normalized value |
| --- | --- | --- |
| `ControlChange` | controller number | `as_u7(value)/127.0` |
| `ChannelPressure` | `kAfterTouch`(128) | `as_u7(pressure)/127.0` |
| `PitchBend` | `kPitchBend`(129) | `bend_as_u14(value)/16383.0` |

NoteOn/NoteOff/PolyPressure/ProgramChange/その他は `None`
（JUCE の mapping domain と一致 — program change は連続 controller
ではないため mapping 対象外）。

`routed_assignment(midi, map) -> Option<(ParamID, f64)>` が
「このイベントは IMidiMapping 経路で消費されるか」を判定する
shared predicate — `fill_param_queues`（配送側）と `fill_event_list`
（抑制側）が同一判定を使う。

- `fill_param_queues`（host.rs）に `midi_map` を渡し、`EventKind::Midi`
  分岐を追加: routed なら対応 queue（`queue.id == ParamID`）へ
  `(offset, normalized)` を push。map 命中・queue 無し/満杯は
  `skipped` 計数。戻り値を `MidiBridgeStats` に変更し
  `param_routed`（map 命中数）/`skipped`（配送失敗）を計上。
- `fill_event_list`（midi.rs）に `midi_map` を渡し、routed event は
  event list に emit しない（計数は `fill_param_queues` 側 —
  二重計上しない）。unmapped は従来どおり `kLegacyMIDICCOutEvent`。
- `MidiBridgeStats` に `param_routed: u64` を追加。

## R2 plugin→host reporting

### neutral contract（plugin-core）

`PluginProcessor` に drain 方式の reporting surface を追加:

```rust
fn drain_output_reports(&mut self, out: &mut Vec<Event>) { let _ = out; }
```

- report の語彙は既存 `Event`/`EventKind` を再利用 —
  `ParamValue` = plugin→host param report、`Midi` = plugin 生成
  channel-voice event。driver-neutral で VST3 固有概念を含まない。
- `ProcessResult` や `process` シグネチャは変えない — 既存の全
  `PluginProcessor` 実装が default no-op で非破壊。
- `ErasedPlugin` に同名メソッドを追加し `Erased<P>` が forward。
  **non-RT only**（`out` への append は caller 側 allocation を許す）。
- plugin 側は `process` 中に bounded buffer へ queue、adapter が
  block 後に drain — pull 型で back-pressure が自然。

### plugin 側（component.rs / gain.rs）

- `GainProcessor` に `pending_reports: Vec<Event>`（prepare で
  `MAX_BLOCK_EVENTS + 8` capacity 予約）。`process` 冒頭で clear、
  event scan 中に queue:
  - 適用した gain `ParamValue` ごとに同 event を report（適用確認）。
  - `NoteOn` `MidiEvent` ごとに echo report（outputEvents 証明用の
    fixture 挙動 — 文書化された PoC  semantics であり音楽的機能ではない）。
  - capacity 超過は defined drop。
- `GainProcessor::drain_output_reports` が `out.append` で引き渡す。
- `GainComponent::process` は `processor.process` 復帰後・tail 記録前に
  `emit_reports` を呼び、`data.outputParameterChanges` /
  `data.outputEvents`（null 可）へ書き込む:
  - `ParamValue` report → `addParameterData(&param_id)` →
    `addPoint(offset, normalized)`（offset は block 内検査済み）。
  - `Midi` report → `midi::outgoing_event`（default build へ ungate）
    → `addEvent`。ppq は `ppq_position(transport.musical(), …)`。
  - null channel・変換不能 kind・範囲外 offset は defined skip、
    `report_stats`（`MidiBridgeStats` 再利用）で計数。
- `ComponentState` に `report_scratch: Vec<Event>`（component 生成時に
  capacity 確保）と `report_stats` を追加。

### host 側（host.rs）

- `BridgeParamQueue` に `writable: bool` を追加 — output queue は
  `addPoint` が bounded push（`MAX_EVENTS_PER_BLOCK` capacity、
  溢れ `kResultFalse`）、input queue は現行どおり `kNotImplemented`。
- `BridgeParameterChanges` に `writable: bool` — output 側は
  `addParameterData(id)` が既存 queue を `getParameterId` 照合で返す
  （無ければ null = defined reject、RT で alloc しない）。
- `prepare` で output queue 一式（declared param ごと）・
  `BridgeParameterChanges{writable:true}`・第 2 `BridgeEventList`
  （`outputEvents` 用）・`output_reports: Vec<Event>`
  （`MAX_EVENTS_PER_BLOCK` capacity）を構築。
- `process` は call 前に output queue/event list を clear、
  `ProcessData.outputParameterChanges`/`outputEvents` に ptr を配線、
  `kResultOk` 復帰後に `collect_output_reports(frames)`:
  - queue points → `block_offset` 検査 + `ParamValue::new` 検査 →
    `Event::new(offset, ParamValue)` を `output_reports` へ
    （cap 超過は `report_stats.skipped`）。
  - output events → `midi::incoming_event`（`pub(crate)` 化）で
    canonical `Event` へ翻訳して同様に蓄積。
- `Vst3Plugin::drain_output_reports` が `out.append(&mut
  self.output_reports)` — since-last-drain 蓄積、cap 到達後の
  drop は計数済み。

## 失敗・境界の定義

- `IMidiMapping` 非実装 plugin → `midi_map = None`、全 MIDI は
  従来の event 経路（mapping 無しと同じ挙動、graceful）。
- map 命中 ParamID が declared param に無い → queue 無しで
  `skipped` 計数、event 経路へは戻さない（JUCE consumed semantics）。
- plugin が `outputParameterChanges`/`outputEvents` に書かない
  （null ptr の host、または書き込みなし）→ report 0 件、drain は空。
- report の offset が block 外・値が非有限 → defined skip 計数。
- `restartComponent(kMidiCCAssignmentChanged)` → 現行どおり
  `kNotImplemented`；mapping は次回 `prepare` で再 snapshot。

## capability 更新

`cap.rs` Vst3 `param_reporting`: `Unavailable` → `Declared`。
fixture in-process E2E で plugin(cdylib)→bridge→`ErasedPlugin` drain
まで証明されるが、L1 wire verb・実 DAW host での consumer 配送は
未検証 — `Verified` にはしない。notes を実態に合わせて更新し、
全 backend `Unavailable` を仮定する test を Vst3=Declared に修正。
