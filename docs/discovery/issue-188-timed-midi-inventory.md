---
id: "DISC-qiDOIoAYQ5bcRQ5rpis6kI"
type: "discovery"
status: "resolved"
title: "Issue #188 Timed MIDI — existing API inventory and gap analysis"
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
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
    - "docs/designs/issue-186-param-control-events-detailed.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/discovery/issue-186-param-control-events-inventory.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/186"
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

# Issue #188 Timed MIDI — existing API inventory and gap analysis

## Summary

Issue #188 が要求する「高精度 Timestamp 付き MIDI 入力 → Audio/Plugin Process への型・時刻を維持したイベント配送」の棚卸しを実施した。結論: **scheduled（plan）配送経路は #190+#196 で実装済みで `ScheduledEvent<P>`/`TransportPlan` は payload 汎用**、RT no-alloc・bounded fault・generation 切断のパターンは全て再利用可能。実ギャップは (a) MIDI 語彙（Note/CC/Bend/Pressure/PC、channel/group）の不在、(b) MIDI 入力境界（port 列挙/open/close/backend 能力）の不在、(c) 入力 timestamp の Clock Domain/Reliability 語彙と stamp→EngineFrame 写像権威の不在、(d) live input → `EventList` への merge 経路の不在、(e) stuck-note fail-safe（NoteLedger/Reset/Chase）の不在、(f) adapter 側 non-ParamValue の暗黙 drop。信頼度: 高（全対象コードを実読確認済み、main@d118a87）。

## Exploration Goal

- Issue #188 の設計・PoC 論点 1〜8 を既存 API にマッピングし、実現可能範囲・不足・規格差分を確定する。
- DAW 固有部分（Synvelis 所有の Recording/Take/Overdub/Piano Roll/Undo）と汎用基盤部分を分離する。
- MIDI Backend（OS/ライブラリ）採否を判断する比較材料を集める。

## Key Issues and Assumptions

- `EventKind` は `ParamValue` のみ。`Event` は `reserved: [u64; 2]` の拡張枠を持ち、`#[non_exhaustive]` で variant 追加は non-breaking（内部 exhaustive match 箇所はコンパイルで検出される）。
- `ScheduledEvent<P>`/`TransportPlan<P>`/`TransportScheduler<P>` は payload ジェネリック — `EventKind::Midi` を載せれば再生経路は現状のまま動作する。`bound_event`（`transport_generation: Some(tg)`）は live inject/one-shot 用途を #190 設計時点で #188 領域として予約済み。
- 入力 timestamp の Clock Domain/Reliability 語彙は存在しない。`TimestampProvenance`（Carried/Derived/Estimated、media-core `observe`）はメディア stream 用だが provenance 概念はそのまま再利用可能。
- MIDI 入力境界（port 列挙・open/close・外部/仮想区別・backend 能力）は一切存在しない。`audio-core::device` の `DeviceEnumerator`/`Device`/`OutputStream` が確立済みの port-trait パターン。
- `EngineFrame{epoch,frame}` が frame 位置の唯一権威。`ClockEpochId` は `device_restart` で更新。`DEVICE_CLOCK_TIMEBASE`（1/1e9 ns）+ `OutputTimestamp`/`OutputCallbackInfo` が device clock を callback へ供給。
- CLAP/VST3 adapter は non-`ParamValue` kind を `let-else` で暗黙 skip（`push_events`/`fill_param_queues`/`translate_events`）。wire codec は kind tag `0`=`ParamValue` のみで未割当 tag は `Malformed` defined rejection。
- `NoteLedger`（稼働 note 追跡）・NoteOff fail-safe・controller chase の機構は存在しない。`ParamDispatchLedger`（scheduled/dispatched/reported 区別）が観測区別の先例パターン。

## Alternatives and Comparison

MIDI イベント語彙・入力境界・同期の配置候補。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. `MidiEvent` 語彙を `ruviform-media-core`（Primitives）に置き、`EventKind::Midi(MidiEvent)` で plugin wire に載せる | media-core は最下層で全層から参照可能。audio-core input port・plugin-core EventKind・bridge・adapter の全消費者が同じ型を共有。語彙は「規格中立データモデル」そのもの | 語彙の所有が plugin-core から media-core へ一部移る（`ParamEvent` は plugin-core 残留で非対称に見える） | **yes** |
| B. `MidiEvent` を `ruviform-plugin-core` に置く | `ParamEvent` と同じ配置で局所的 | audio-core（domain foundation）の input port が extension 層の plugin-core に依存できない → 入力境界は別の重複型を持つことになり語彙が分裂 | **no** |
| C. 新規 `ruviform-midi-core` domain foundation crate を新設 | 層構造上最も明示的 | 語彙 + port trait のためだけに crate/Cargo.toml/README/CI lane を増設。語彙自体は小さく、audio-core `device` port パターンに沿った module で充足 | maybe（実 backend adapter 追加時に再評価） |
| D. MIDI input port を `ruviform-audio-core::midi` module に置く | `device.rs` の `DeviceEnumerator`/`Device`/`OutputStream`/`DeviceError` パターンをそのまま MIDI input 向けに反復。新 crate 不要 | audio-core の名前空間は「audio」だが device port は実質「platform device boundary」として確立済み | **yes** |
| E. 入力 timestamp → frame 写像を media-core の純粋関数 + anchor snapshot として所有 | `media_time_ticks`/`event_engine_frame` と同じ「単一権威」パターン。RT 側は整数演算のみ。非RT 側が anchor 校正を所有 | anchor 校正自体は非RTポリシー（別 Issue で拡張） | **yes** |
| F. live input を `TransportScheduler` へ注入する | scheduler が全イベントの単一配送権威 | live input は plan でなく per-callback drain が自然。scheduler 内部に第2ソースを増やすと span/occurrence/ordering 契約が複雑化 | no（delivery 側で merge） |
| G. bridge `CallSink` で plan events と mapped live events を offset 安定 merge | `build_call_event_list` の出力と同じ wire 形（`EventList`）を維持。scratch 事前確保で RT no-alloc | scratch が2系統分必要・merge 規約の中立明文化が必要 | **yes** |
| H. OS MIDI backend（midir / CoreMIDI / ALSA / WinMM）を本 Issue で選定・実装 | 実機検証が可能 | midir の timestamp 精度は OS 依存（CoreMIDI host ticks / ALSA seq tick / WinMM は無 timestamp で host 時刻推定）。PoC では Mock input が受入条件。実 backend は比較→別 Issue | **defer（ADR で比較記録）** |

## Tentative Conclusions and Hypotheses

- **[confirmed]** 同期の正本: `EngineFrame{epoch,frame}` が配送時刻の唯一権威（1 frame = frame_base tick、PoC では 1 frame = 1 sample）。入力 timestamp は `ClockAnchor{domain, stamp_ticks, stamp_base, frame, calibration}` snapshot を介して 1 回だけ rational 写像される（`NearestEven`、#189 と同じ単回丸め方針）。
- **[confirmed]** `TimestampProvenance` を provenance 語彙として再利用し、写像信頼度は「stamp provenance × anchor calibration」の弱い方を継承する。未知 domain は `UnmappedDomain` defined handling（即時配送+fault、silent drop/誤配置なし）。
- **[confirmed]** `EventKind::Midi(MidiEvent)` は wire 形を変えない追加（`ParamValue` の意味・`EventList` 規約・`TransportInfo`・`ScheduledEvent`/`TransportPlan`・graph `PortFormat`・L0/L1/CLAP/VST3 の現行挙動は全て不変）。L1 wire は kind tag `1` を追加する additive 拡張 — 旧 codec は未知 tag を `Malformed` defined rejection するため暗黙の誤配送はない。
- **[confirmed]** Replay 経路は `TransportPlanBuilder::<EventKind>` に `EventKind::Midi` を乗せるだけで既存の span 分割・loop occurrence 再発火・generation 切断・`build_call_event_list` がそのまま使える。tempo change/seek/loop/stop での Note-off/Reset は span ctx の `transport_generation` 不連続を trigger として `NoteLedger` が release events を offset 0 で供給する汎用機構として配置する。
- **[confirmed]** 欠落可視化: input ring overflow・unmapped domain・late event・ledger overflow・scratch merge overflow は全て `NodeStatus` counter + `NodeReport` fault で可観測（`EventScratchOverflow`/`VaultExceeded` の先例）。
- **[confirmed]** PPQN960 破壊的 quantize なし: 入力は native stamp（`InputStamp{ticks, base, domain, provenance}`）を保持し、`TimedInputEvent` 全体が Copy 値型として消費者へ引き渡される。配送時の frame offset 写像は「表示位置」であり stamp 自体は破壊されない。
- **[confirmed]** adapter 差分の正直な宣言: `cap.rs` に note/channel event 能力フィールドを追加し、未対応は `Declared`/`Unavailable`/`Unsupported` で表明。`push_events`/`fill_param_queues` の non-ParamValue skip を暗黙 → defined drop（count 付き）へ修正する。
- **[hypothesis]** NoteLedger の ledger full → force-release oldest + fault が stuck-note 予防の正しい優先順位（drop より release が安全）。

## Open Questions

- MIDI2/UMP の 16bit velocity・32bit bend・per-note controller をどこまで語彙化するか（PoC では MSB-justified canonical value で両方を無損失表現、規格側写像は adapter が判断）→ design で確定。
- 実 backend（midir vs 直 OS API）の比較は本 Issue で ADR 記録までか、実 adapter 先行実装までか → Issue 分割で確定。
- input event の engine 側「録音 tap」を bridge の report channel で見せるか、engine 層の別機構とするか → design で最小形を確定。

## Promotion Candidates

- [x] spec-doc needed: MIDI 語彙・入力境界・Clock Domain/Reliability・merge/ordering・note safety・adapter 差分・RT 契約・互換性の受入条件
- [x] adr-doc needed: 語彙配置（media-core）・入力 port 配置（audio-core）・同期権威（EngineFrame+ClockAnchor）・backend 選定延期・NoteSafety 配置の決定

## Sources and Relations

- Issue: ruviform#188（本棚卸しの要求元）、synvelis#12（consumer Recording 要求）、synvelis#18/#20/#22（Timed Event/Note Safety/Native Timestamp 設計要求）、ruviform#184/#186/#189/#190/#193/#196。
- 実コード（main@d118a87 時点）: `crates/ruviform-plugin-core/src/{event,param,param_write,transport,transport_bridge,wire,plugin,processor}.rs`、`crates/ruviform-media-core/src/{transport_schedule,music,time,observe}.rs`、`crates/ruviform-audio-core/src/{device,buffer,format}.rs`、`crates/ruviform-audio-plugin-bridge/src/{node,control,status,cap,dispatch,target,vault,attach}.rs`、`crates/ruviform-plugin-clap/src/{adapt,host}.rs`、`crates/ruviform-plugin-vst3/src/{block,host}.rs`、`crates/ruviform-rt/src/{spsc,snapshot,retire}.rs`、`crates/ruviform-graph/src/port.rs`。

## Evidence

以下は main@d118a87 の実コード読取結果。各項は Issue #188 設計・PoC 論点 1〜8 に対応する。

### E1. MIDI Input Boundary（論点 1）

- 既存: MIDI 入力の型・trait・列挙・open/close は**一切存在しない**。
- 再利用可能なパターン: `ruviform_audio_core::device` が `DeviceEnumerator`（`output_devices`/`default_output_device`）→`Device`（`name`/`id`/`default_output_format`/`supported_output_formats`/`open_output_stream`）→`OutputStream`（`start`/`stop`/`format`/`buffer_size`/`now`）の 3 層 port-trait を定義。全て非RT object、`Box<dyn>` 経由、`DeviceError` で normalized error。data callback は backend RT スレッド上の RT-contract section（no alloc/no block/no panic/no re-entry）。
- timestamp 供給: `OutputTimestamp{callback,playback:Option<MediaTime>}` + `OutputCallbackInfo` が `DEVICE_CLOCK_TIMEBASE`（1/1e9 s、ns）の device clock を callback へ供給。`OutputStream::now()` は非RT query。
- **不足**: (a) MIDI input port 型（enumerator/port/connection）。(b) 外部機器/仮想入力/loopback の区別語彙。(c) backend 能力照会（timestamp 有無・SysEx・UMP）。(d) 入力 callback の timestamp 供給規約。(e) `DeviceError` 相当の MIDI error 語彙。

### E2. イベント型（論点 2）

- 既存: `EventKind{ParamValue(ParamEvent)}`（`#[non_exhaustive]`、`Copy`）。`Event{frame_offset:u32, kind:EventKind, reserved:[u64;2]}`（`reserved` は future payload kinds のための opaque 拡張枠と明記済み）。`EventList` は非減少 offset・`offset<block_frames`・0-frame flush・安定 same-offset order（`sort_events_by_frame_offset` は O(n log n) allocation-free merge sort、scratch 不足時は挿入ソート fallback）。
- `ParamEvent{param_id:ParamId(u32), value:ParamValue(f64 normalized [0,1])}`。`TimedParamEvent{position:MediaTime, event:ParamEvent}` は非RT build-side 型。
- `PortFormat::TimedEvent`/`SignalDomain::TimedEvent` は M1 でマーカーのみ（typed payload は M3+ 未定義、本 Issue の領域）。
- **不足**: (a) `MidiEvent`/`MidiKind`（NoteOn/NoteOff/CC/PitchBend/PolyPressure/ChannelPressure/ProgramChange + System/SysEx 拡張余地）。(b) `MidiChannel`/`MidiGroup`/`NoteKey` の validated 型（0..=15/0..=15/0..=127）。(c) 値の canonical 表現（velocity/pressure/bend の MSB-justified 整数 — MIDI1 7bit/14bit と UMP 16bit/32bit の両方を無損失保持）。(d) `EventKind::Midi` variant。(e) adapter 側の kind 別 capability 照会。

### E3. Timestamp / Clock Domain（論点 3）

- 既存: `TimestampProvenance{Carried,Derived,Estimated}`・`TickTimestamp{ticks,provenance}`・`MediaTimestamp{time,provenance}`（media-core `observe`、media reader 用だが provenance 語彙として再利用可能）。`EngineFrame{epoch:ClockEpochId, frame:u64}` が frame 権威、`checked_offset`/`advance`/`same_epoch` のみ。`media_time_ticks` は `convert_exact` ベースの position→tick 単一権威。
- `BeatPosition{num:i64,den:u32}`（有理数正本）・`TempoMap`（非RT compile・immutable・O(log n) クエリ・`beat_to_media` は単一 NearestEven 丸め・`media_to_beat` は exact）・`Ppqn`（可変、#189）。`MusicalTimeInfo` は per-call ABI snapshot。
- `DEVICE_CLOCK_TIMEBASE`=1/1e9。`OutputTimestamp`/`OutputCallbackInfo` は device clock を callback に供給（DAC playback 時刻は `Option`、未報告 backend は `None`）。
- **不足**: (a) `ClockDomainId`/clock domain 語彙（sample clock・backend monotonic・unknown）。(b) 入力 stamp 型（`ticks`+`base`+`domain`+`provenance`）。(c) `ClockAnchor`（domain stamp ↔ EngineFrame の pair、calibration provenance 付き）。(d) stamp→frame 写像の単一権威関数（anchor delta → frame_base 変換、負 delta/epoch 前/未写像 domain の defined handling）。(e) 写像信頼度語彙（未知・推定・検証済みの区別）。

### E4. Event Pipeline（論点 4）

- 非RT 準備: `TransportPlanBuilder::new(frame_base, generation)`→`event(position, source_sequence, payload)`/`bound_event`/`build()`（`(position_ticks, source_sequence)` 安定ソート・plan_generation 刻印）。`install_plan`/`request`/`preflight`/`device_restart` 非RT。`PluginNodeControl` が bounded SPSC（`CTRL_DRAIN_MAX=32`/callback、`QueueFull` defined rejection）経由で RT 側へ伝達。
- RT 配送: `TransportScheduler::run_callback(start, frames, sink)`→span 分割→`build_call_event_list(span, events, call_start, len, scratch, kind)`→`EventList`→`PluginEndpoint::process`。`event_scratch` は `budget.max_events_per_span` で事前確保、不足は `EventScratchOverflow` report + 空 list で継続（defined drop）。
- **不足**: (a) live input の入口（bounded SPSC または同等の ring、producer=callback側 / consumer=RT側）。(b) 入力 drain → stamp 写像 → frame offset への正規化段。(c) plan events と mapped live events の安定 merge（same-frame ordering 規約: 両 source で causal order 保持、cross-source tie は deterministic 規則の明文化）。(d) 容量超過検出（input queue full・mapped hold overflow・merge scratch overflow）の fault/counter。(e) 0-frame flush・block 境界規約の live 入力への適用。

### E5. Scheduling & Replay（論点 5）

- 既存: `ScheduledEvent<P>{position, source_sequence, plan_generation, transport_generation:Option<..>, payload}`。`bound_event` は `Some(tg)` bound（live inject/one-shot 用途を #190 が #188 領域として予約）。loop 内 events は occurrence ごとに 1 回配送。`Play/Pause/Stop/Seek` は `tg++`、`SetLoop/ClearLoop`/loop wrap は `tg` 据置。`device_restart` で `epoch++`+全 pending `StaleGeneration` 拒否。
- **不足**: (a) `NoteLedger`（channel+key の active note 追跡、bounded capacity、overflow → force-release+fault）。(b) transport 不連続（tg 変化・device_restart・Stop）での release/reset events 自動供給。(c) CC64/AllNotesOff/AllSoundOff の chase/reset 語彙。(d) recorded event の engine block 配送は plan 経路で動くが、live input と replay の同一 pipeline 統合（merge 規約の統一）の設計。

### E6. Plugin adapter（論点 6）

| 項目 | L0 (in-process) | L1 (out-of-process) | CLAP adapter | VST3 adapter |
| --- | --- | --- | --- | --- |
| Event 配送 | `EventList` Verified | `EventList` Verified（`WIRE_MAX_EVENTS=1024`、kind tag `0`=ParamValue、未知 tag は `Malformed`） | `push_events`→`ParamValueEvent`（Declared）。`MAX_EVENTS_PER_BLOCK=256` 超過は silent truncation。non-ParamValue は `let-else` で暗黙 skip | `fill_param_queues`→per-param `IParamValueQueue`（Declared）。queue 1024 上限。non-ParamValue・foreign id・範囲外は暗黙 drop |
| Plugin 側受信 | — | — | `translate_events`: ParamValue のみ。`header.time`→`frame_offset`、unsorted は安定再ソート、>256 は `CLAP_PROCESS_ERROR` | `collect_param_events`: `IParamValueQueue` のみ。`MAX_BLOCK_EVENTS=1024`+`MAX_PARAM_SCAN_OPERATIONS=8192`。`IEventList`/`inputEvents`/`outputEvents` 常に null |
| Note/MIDI 語彙 | `EventKind` に語彙なし | wire に tag なし | clack 0.2.0 は `NoteOn/NoteOff/NoteChoke/NoteEnd/NoteExpression/MidiEvent(3byte)/Midi2Event` を提供 — 未使用 | `IEventList`/`NoteOnEvent`/`LegacyMIDICCOutEvent`/note-expression は未接続 |
| plugin→host | 経路なし | 経路なし | output events `OutputEvents::void()` 破棄 | `outputParameterChanges` 常に null、`IComponentHandler` ack 後破棄 |

- **不足**: (a) `EventKind::Midi` の各 adapter での translation（CLAP `NoteOnEvent`/`NoteOffEvent`/`MidiEvent`、VST3 `IEventList`/`LegacyMIDICCOutEvent`）。(b) 未対応 kind/精度の defined rejection（現在は暗黙 skip）。(c) `cap.rs` の note/channel/per-note 能力フィールド。(d) 精度差分（velocity f64/16bit→7bit narrowing、bend 14bit、Pckn port/channel/key/note_id 写像、VST3 note_id 不在）の正直な宣言。

### E7. RT contract（論点 7）

- 既存: `ruviform_rt::spsc::channel<T:Send>`（`try_push`/`try_pop`/`pop_latest`、capacity bounded、no alloc/no block）。`SnapshotWriter`/`Reader`（plan snapshot 交換、retire channel）。`NodeStatus`/`Counter`（AtomicU64 counters、report overflow bump）。`PlanVault`（RT 側で destructor 不実行）。
- no-alloc: `run_callback`/`build_call_event_list`/`call_transport_info`/`event_engine_frame`/`sort_events_by_frame_offset` は allocation-free。`PluginNode::process` 全体が alloc なし設計。
- `rt_` テスト: `ruviform-rt` に `rt_*_no_alloc` 測定先例（alloc_counter）。plugin-core/media-core/bridge に `rt_` 契約テスト（`mise run test-rt` が `cargo test --workspace rt_` で選択）。
- **不足**: (a) 入力 drain・stamp 写像・merge・ledger update の no-alloc/bounded 証明。(b) 非RT 準備（anchor 校正・port open/close・input channel 構築）と RT 側の分離。(c) `INPUT_DRAIN_MAX` 等の budget 定数。

### E8. 互換性（論点 8）

- **互換維持が必要な契約**: `EventList` wire 形（`frame_offset<u32`・非減少・`offset<block_frames`・0-frame flush）、`EventKind::ParamValue` 意味・wire tag `0`、`ParamValue` NaN/clamp ポリシー、`PluginEndpoint::process`/`PluginProcessor::process` シグネチャ、`TransportInfo`/`MusicalTimeInfo` snapshot、`ScheduledEvent`/`TransportPlan`/`TransportScheduler` 型、`NodeControl`/`NodeReport` 語彙、`PortFormat`/`SignalDomain`、adapter `capability_for` 行列、wire codec の未知 tag `Malformed` defined rejection。
- **破壊的変更の可能性**: `Event` の `reserved:[u64;2]` は MidiEvent に使わず `EventKind` payload 側に乗せる方針で wire 形を維持できる（`MidiEvent{group,channel,kind}` は ParamEvent と同程度サイズ）。`AdapterCapability` への field 追加は構造体リテラル構築の破壊（publish=false workspace-internal のため許容、IR で記録）。
- **機能不足一覧（本 Issue で扱う範囲）**:
  1. MIDI 語彙（`MidiEvent`/`MidiKind`/`MidiChannel`/`MidiGroup`/`NoteKey`、canonical MSB-justified value、SysEx/UMP 拡張余地）
  2. `EventKind::Midi` + wire tag `1` + `merge_call_events`（plan×live 安定 merge）
  3. Clock Domain/stamp/provenance/anchor + stamp→frame 写像権威
  4. live input bounded handoff + per-callback drain + normalized delivery
  5. `NoteLedger` + transport 不連続時の release/reset + overflow 可視化
  6. audio-core MIDI input port trait（列挙/open/close/外部・仮想区別/backend 能力）
  7. adapter cap 拡張 + non-ParamValue defined drop
  8. 実 backend 選定の比較記録（midir vs OS API、実装は分割 Issue）

### E9. 消費側（Synvelis #12）が Ruviform に要求する最小契約

- Native timestamp（ticks+timebase+domain+provenance）を破壊せず消費者へ引き渡せる `TimedInputEvent` 形。
- frame offset 精度での Note On/Off/CC/PitchBend の型・順序維持配送。
- seek/loop/stop/overflow での stuck-note fail-safe（release/reset）と欠落の可観測性。
- adapter 別の未対応精度/fallback/拒否の明示（誇大な互換保証なし）。
- realtime/offline で同一 event order・source-relative time の再現性。

### E10. 後方互換と機能不足一覧（Issue AC「既存実装と不足一覧」）

- **互換維持**: E8 一覧の全契約（`EventKind` variant 追加は `#[non_exhaustive]` で non-breaking、wire tag `1` は additive、旧 codec 側は `Malformed` で定義済み拒否）。
- **consumer 影響分類**: synvelis = impacted（新 API 利用予定・#12 本筋）。audio-lens = no impact（既存 wire/挙動不変・追加のみ）。
- **分割 Issue 候補（本 Issue では PoC + 設計 + 分割案のみ）**: (a) 実 MIDI backend adapter（midir 等の比較→実装）、(b) `PortFormat::TimedEvent` の typed payload 化と graph 配線、(c) VST3 `IEventList`/note-expression 実装、(d) L1 wire の MIDI 対応（version negotiation 含む）、(e) engine 層の録音 tap/consumer 向け stamp 投影 API、(f) NoteLedger の controller chase 拡張（CC64/全CC reset ポリシー）。
