---
id: "DESIGN-lnTibw26AboiFgtq2O9IsE"
type: "design"
status: "approved"
title: "Issue #188 Timed MIDI — overview"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-media-core/src/midi.rs"
      - type: "file"
        path: "crates/ruviform-media-core/src/timed_input.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/midi.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/input.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/tests/midi_delivery.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-media-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/transport_bridge.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/control.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/node.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/cap.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/status.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/attach.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/designs/issue-186-param-control-events-detailed.md"
    - "docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md"
    - "docs/designs/issue-188-timed-midi-detailed.md"
    - "docs/discovery/issue-188-timed-midi-inventory.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-188-timed-midi.md"
    - "docs/adr/issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/synvelis/issues/12"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  defers: []
  deferred-by: []
metadata:
  gate: "M3"
  issue: 188
---

# Issue #188 Timed MIDI — overview

## Purpose

Timestamp 付き MIDI 入力を Audio/Plugin Process へ型・時刻・順序を維持して配送する汎用契約の PoC 設計。`docs/specs/issue-188-timed-midi.md` の R1〜R15 を実現し、`docs/adr/issue-188-timed-midi-contract-placement-clock-authority-and-backend-deferral.md` の配置決定（media-core 語彙・audio-core port・bridge machinery・no new crate・backend 延期）に従う。詳細 API は `docs/designs/issue-188-timed-midi-detailed.md`。

## Design Decisions

### D1: MIDI 語彙（`ruviform-media-core::midi`）

- `MidiChannel(u8 0..=15)`・`MidiGroup(u8 0..=15)`・`NoteKey(u8 0..=127)` の validated newtype（`ParamId`/`ParamValue` と同じパターン、`TryFrom`/`new` で範囲検証、違反は `MidiRangeError`）。
- `MidiKind`（`#[non_exhaustive]`、`Copy`）: `NoteOn{key,velocity}`・`NoteOff{key,velocity}`・`ControlChange{controller,value}`・`PitchBend{value}`・`PolyPressure{key,pressure}`・`ChannelPressure{pressure}`・`ProgramChange{program}`。System/SysEx は variant 拡張余地として doc 記載（wire 化は分割 Issue）。
- 値表現は **MSB-justified canonical 整数**: velocity/pressure は `u32`（MIDI1 7bit は `v<<25`、UMP 16bit は `v<<16` で正規化）、PitchBend は `i32`（14bit/32bit を同じく MSB 位置へ正規化、0=center）。`MidiValue` helper が `from_u7`/`from_u14`/`from_u16`/`as_u7`/`as_u14`/`as_u16` 変換を提供し、MIDI1 と MIDI2/UMP の両方を無損失保持。adapter が規格側精度へ変換する際の丸めは adapter の責務。
- `MidiEvent{group, channel, kind}` は `Copy`（`ParamEvent` と同程度サイズ、`Event` wire 形に収まる）。
- `NoteLedger`: channel+key（MIDI1 では channel+key+group）→ active 追跡の bounded 固定容量配列。`note_on`/`note_off`/`release_all(&mut [MidiEvent]) -> usize`/`is_active`。容量超過は最古を force-release し overflow flag を立てる。MIDI1 duplicate-pitch の曖昧性（同一 channel+key の重複 Note-On は per-count 管理、voice 個別識別不可）を doc で明記。

### D2: Timestamp / Clock Domain（`ruviform-media-core::timed_input`）

- `ClockDomainId(u64)` — 入力 clock domain の opaque 識別子（sample clock・backend monotonic・unknown を区別するための domain tag。生成/登録は非RT所有者の責務）。
- `InputStamp{ticks:u64, base:MediaTimeBase, domain:ClockDomainId, provenance:TimestampProvenance}` — native timestamp を保持（tick への強制丸めなし、base で timebase を明示）。
- `TimedInputEvent<P>{stamp:InputStamp, event:P}` — `Copy` の generic 型。`P` は PoC では `MidiEvent` だが将来的に他入力 source にも再利用可能。
- `ClockAnchor{domain, stamp_ticks, stamp_base, frame:EngineFrame, calibration:TimestampProvenance}` — 1 つの domain stamp ↔ `EngineFrame` の pair。校正は非RT（audio callback 初期・`SetClockAnchor` control）で行い RT 側は snapshot のみ。
- `map_stamp_to_frame(stamp, anchor, frame_base) -> Result<EngineFrame, MapRejection>` — anchor delta を `convert` で frame_base 単位に変換する単一権威関数（`media_time_ticks` と同じ権威パターン）。domain 不一致→`UnmappedDomain`、epoch 不一致→`EpochMismatch`、負 delta（anchor より過去）→`BeforeAnchor` defined rejection。丸めは単回 `NearestEven`。写像信頼度 = min(stamp.provenance, anchor.calibration) で `MappedInput{event, frame, provenance}` に付す。

### D3: MIDI Input Boundary（`ruviform-audio-core::midi`）

`audio-core::device` の `DeviceEnumerator`/`Device`/`OutputStream` パターンを入力方向へ反復（全て非RT object、`Box<dyn>`、`MidiInputError` normalized error）:

- `MidiInputEnumerator` — `input_ports() -> Vec<Box<dyn MidiInputPort>>`、`default_input_port()`。
- `MidiInputPort` — `id()`/`name()`/`kind() -> MidiPortKind{External,Virtual,Loopback,Unknown}`/`capabilities() -> MidiInputCaps`/`open(&format/callback) -> Box<dyn MidiInputConnection>`。
- `MidiInputCaps{timestamps:MidiSupport, sysex:MidiSupport, ump:MidiSupport, virtual_ports:MidiSupport}` — `MidiSupport{Unknown,Unsupported,Supported}` 三値（`InBlockTransportSupport` の honesty パターン）。Unknown は supported 扱いしない。
- `MidiInputConnection` — `close()`、data callback は backend/input thread 側で `&[TimedInputEvent<MidiEvent>]` を push。callback contract は RT（no alloc/no block/no panic/no re-entry）。
- `MockMidiEnumerator`/`MockMidiInput` — PoC・テスト用の制御された `TimedInputEvent` ソース（queue へ `push`、open/close、port kind/caps を宣言）。実 OS backend（midir 等）は分割 Issue。

### D4: EventKind 拡張と merge（`ruviform-plugin-core`）

- `EventKind::Midi(MidiEvent)` variant 追加（`#[non_exhaustive]` 維持、`Copy`、wire 形・`reserved` ポリシー不変）。`ParamValue` 意味・`EventList` 規約（非減少 offset・`offset<block_frames`・0-frame flush・安定 same-offset）は変更しない。
- L1 wire codec: kind tag `1` = Midi（tag `0`=ParamValue 不変、未知 tag は既存 `Malformed` defined rejection）。
- `merge_call_events(plan:&[Event], live:&[Event], scratch:&mut [Event]) -> Result<&mut [Event], MergeOverflow>`（`transport_bridge.rs` または `event.rs`、pure function、allocation-free）: 2 つの非減少 `Event` 列を frame offset で安定 merge。規約: (a) 各 source 内の causal order 保持、(b) 同一 offset の cross-source tie は plan-before-live、(c) 同一 offset・同一 channel+key では NoteOff が NoteOn より先（Note-Off priority、`MidiKind` にのみ適用）、(d) scratch 不足は `MergeOverflow` defined rejection（`EventScratchOverflow` 相当の report へ接続）。
- `merge_call_events` の出力は `EventList` 規約を満たす（両入力が規約を満たし merge が安定なら非減少は保持される）。

### D5: Live→Block Pipeline（`ruviform-audio-plugin-bridge::input`）

`PluginNode` の `CallSink` が plan path（`TransportScheduler`→`build_call_event_list`）と live path（`InputStage`）を merge して 1 つの `EventList` を構築する。

- `InputStage`（RT 側）: bounded SPSC ring（`rt::spsc`、capacity=`INPUT_RING_CAPACITY`）から per-callback `INPUT_DRAIN_MAX` budget で drain → stamp→frame 写像（`ClockAnchor` snapshot）→ frame offset 化 → mapped scratch（固定配列）へ積む。anchor 未供給時は `UnmappedDomain`/anchor-absent fault + defined handling（latest-frame flush ではなく defined drop/counter、silent drop しない）。
- `NodeControl::SetClockAnchor{anchor}` — 非RT（または audio callback 初期）から anchor snapshot を install。`NodeControl` は `#[non_exhaustive]` で additive。
- `AttachConfig`/`attach` に `input: Option<InputConfig>` を追加し `Attached` から input ring の producer 側（`InputTx`）を露出。
- `CallSink` 内の処理順: (1) scheduler span events → plan scratch、(2) `InputStage::drain` → live scratch、(3) `merge_call_events` → merged scratch → `EventList`、(4) `NoteLedger.feed(&events)`、(5) transport 不連続検出時は `NoteLedger::release_all` を offset 0 で prepend（先に ledger 由来 release を挿入してから merge、規約 D4-(c) で NoteOff が優先される）。
- fault 可視化: `InputQueueFull`（producer 側 `try_push` 失敗）、`InputDrained{count}`、 `InputUnmapped`、`InputBeforeAnchor`、`InputLedgerOverflow`、`InputMergeOverflow` を `NodeReport`/`NodeStatus` counter へ追加（`EventScratchOverflow`/`VaultExceeded` と同じ defined-fault パターン）。
- `EventList` 規約（0-frame flush・block 境界・非減少 offset）を live input にも適用。block 境界上は現行規約で次ブロック先頭。

### D6: Replay / transport 連動

- Recorded event の再生は既存 `TransportPlan<EventKind>` に `EventKind::Midi` を乗せるだけで動作（span 分割・loop occurrence・generation 切断・`build_call_event_list` は payload 汎用）。
- `NoteLedger` の不連続 release は `CallSink` が `TransportGeneration`/`ClockEpochId`/status 変化を検出して offset 0 に供給する汎用機構（plan live 両方に適用）。Stop/Seek/Loop wrap/device_restart 各規約は #190 の generation 規約を再利用。
- controller chase（CC64 等の state re-send）は `NoteLedger` の範囲外・分割 Issue。

### D7: Adapter 差分と honesty

- `cap.rs`: `AdapterCapability` に note/channel event 対応を示すフィールドを追加（`note_events:MidiSupport` 的な宣言・`channel_events`・`note_expression`）。L0=`Supported/Verified`（pass-through）、CLAP=`Supported`（PoC で `NoteOnEvent`/`NoteOffEvent`/`MidiEvent` translation 実装）、VST3=`Unavailable`（`IEventList` 未実装・分割 Issue）、L1=`Declared`（wire tag `1` 定義、実端末の動作は L1 lane の範囲外）。
- CLAP `push_events`: `EventKind::Midi` → `MidiKind` ごとに `NoteOnEvent`/`NoteOffEvent`（velocity f64、Pckn{port,channel,key,note_id}）または `MidiEvent`（CC/Bend/Pressure/PC が 7bit 表現可能な場合）へ変換。表現不能な精度は counted defined-drop。non-`ParamValue` の `let-else` skip を `match` + count 化。
- VST3 `fill_param_queues`: `EventKind::Midi` は counted defined-drop（cap 宣言で Unavailable と整合）。
- `translate_events`（CLAP plugin 側受信）は PoC では `ParamValue` のまま。Midi 受信側変換は分割 Issue。

### D8: RT / non-RT 分離

- RT（process callback）: ring drain・`map_stamp_to_frame`・`merge_call_events`・`NoteLedger`・`EventList` 構築・plugin call — 全て no-alloc/no-block/no I/O・有界（`INPUT_DRAIN_MAX`・`INPUT_RING_CAPACITY`・`MAX_EVENTS_PER_BLOCK`/scratch）。
- 非RT: port open/close・`MockMidiInput` 準備・anchor 校正・ring 構築・`InputConfig` attach・plan 構築・retire・`PushInput`（producer 側 `try_push` は呼び出し側 thread で実行、full は即座に observable）。

## Architecture

```text
MIDI device / virtual port
        │  (backend input thread, RT callback contract)
        ▼
TimedInputEvent<MidiEvent>{stamp{ticks,base,domain,provenance}, event}
        │  audio-core::midi port trait (Enumerator→Port→Connection)
        ▼
InputTx (bounded SPSC producer, non-RT or input thread)
        │  try_push — full → InputQueueFull counter
        ▼
┌─ PluginNode::process (RT) ────────────────────────────────┐
│ drain control (InstallPlan/SetClockAnchor/…)              │
│ InputStage::drain(budget=INPUT_DRAIN_MAX)                 │
│   └─ map_stamp_to_frame(stamp, anchor, frame_base)        │
│        └─ MapRejection → defined fault + counter          │
│ scheduler.run_callback → build_call_event_list (plan)     │
│ merge_call_events(plan, mapped_live) → EventList          │
│   └─ same-offset: causal order, plan-before-live,         │
│      NoteOff before NoteOn (same channel+key)             │
│ NoteLedger.feed(events)                                   │
│ tg/epoch/status discontinuity → release_all @offset 0     │
│ PluginEndpoint::process(buses, events, transport)         │
└────────────────────────────────────────────────────────────┘
        │  NodeStatus counters / NodeReport faults (bounded)
        ▼
Recorded/replay path: TransportPlanBuilder::event(pos, seq, EventKind::Midi(..))
        → install_plan → scheduler → same EventList merge path
```

## Alternatives

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 語彙を plugin-core に置く | ParamEvent と同層 | audio-core input port が plugin-core 依存か語彙分裂 | no |
| 新規 ruviform-midi-core crate | 層構造明示的 | 語彙規模に対し過剰、manifest/lock churn | defer（実 backend 追加時再評価） |
| live input を scheduler へ注入 | 配送権威単一化 | live は plan でなく per-callback drain が自然、span/occurrence 契約複雑化 | no |
| midir 実 backend 実装 | 実機検証可能 | timestamp 精度が OS 依存、PoC 受入条件は Mock で充足 | defer（ADR で比較記録） |
| VST3 `IEventList` 実装 | 規格カバレッジ | PoC 範囲外、cap honesty + defined drop で整合 | defer |

## Test Strategy

- media-core: `midi`/`timed_input` unit（範囲検証・canonical 値・ledger・mapper defined rejection）+ `rt_` no-alloc。
- plugin-core: `EventKind::Midi` roundtrip（wire tag `1`）・`merge_call_events` の安定 merge・NoteOff priority・overflow。
- bridge: `tests/midi_delivery.rs` — Mock input→PluginNode→L0 plugin で型・順序・offset・stamp→frame・block 境界・loop/tempo/重複 stamp/NoteOff priority/0-frame/overflow visibility/ledger release/`rt_` no-alloc。
- CLAP adapter: `EventKind::Midi`→`NoteOnEvent`/`NoteOffEvent`/`MidiEvent` translation・defined-drop count。
- VST3: cap honesty・counted defined-drop。
- 検証入口: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci`、CLAP/VST3 変更は `mise run test-l1-real-plugin`/`lint-l1-real-plugin` lane も。

## Consumer Impact

- **synvelis**: impacted — #12 の録音/再生に使う新 API。`TimedInputEvent`/input port/`EventKind::Midi`/`NoteLedger`/adapter cap を利用予定。
- **audio-lens**: no impact — `EventKind` は `#[non_exhaustive]` additive、既存 wire/挙動不変。新 API の利用は任意。
- **unverified**: 実 OS backend・実 CLAP/VST3 プラグインでの MIDI 配送は PoC 範囲外（Mock + L0 + adapter 単位テストのみ）。
