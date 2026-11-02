---
id: "DISC-034QkuJnI4etlHHuplmcbe"
type: "discovery"
status: "resolved"
title: "Issue #186 Parameter Control Events — existing API inventory and gap analysis"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/186"
    - "https://github.com/scarecrow173/synvelis/issues/6"
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
    - "docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/discovery/issue-185-generic-probe-observation-inventory.md"
    - "docs/specs/issue-190-transport-span-scheduler.md"
    - "docs/specs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/189"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/196"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/91"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "https://github.com/scarecrow173/synvelis/issues/20"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  defers: []
  deferred-by: []
metadata:
  gate: "M3"
  issue: 186
---

# Issue #186 Parameter Control Events — existing API inventory and gap analysis

## Summary

Issue #186 が要求する「制御結果を対応する処理対象へ安全かつ検証可能な精度で伝達する最低限の API/Capability 契約」について、既存コードを全数棚卸しした。結論: **タイムドイベントの配送骨格（TransportPlan/Scheduler/EventList/PluginNode）は #190+#196 で実装済みであり再利用可能**だが、パラメータ Capability 照会、ParamId↔TargetId 写像、ホスト解決値の追跡（scheduled/dispatched/reported 区別）、容量超過時の coalescing 方針、パラメータ単位の defined rejection、plugin→host パラメータ報告の不在が実ギャップである。信頼度: 高（全対象コードを実読・確認済み）。

## Exploration Goal

- Issue #186 の検討範囲 1〜8 を既存 API に対してマッピングし、実現可能範囲・不足・規格差分を確定する。
- DAW 要件固有部分（Synvelis 所有の Automation/Modulation セマンティクス）と汎用基盤部分を分離する。
- 汎用 Control 評価基盤（Source→Transform→Target DAG）の共通化が実需かを判断する材料を集める。

## Key Issues and Assumptions

- `EventKind::ParamValue` が唯一のイベント種別であり、per-note/per-voice や MIDI 系のイベント語彙は存在しない（#188 側の責務）。
- `Plugin::params() -> &[ParamInfo]` は列挙のみ。音声パスに get/set 動詞も plugin→host 報告チャネルも存在しない。
- `ParamFlags::READONLY` は「plugin が変更してよい」と規定するが、変更を観測する手段がない。`MODULATABLE` は規格側の per-note 能力を示し得るが、中立モデルは per-note 配送を持たない。
- `ParamId(pub u32)` は plugin ローカル。アプリ内安定 ID（TargetId）や構成変更時の未解決状態を表す型は存在しない。
- `AdapterCapability`（bridge `cap.rs`）はアダプタ/層単位であり、パラメータ単位の配送手段・精度・保証を照会する API がない。
- 同一 offset で同一 param に競合するイベントの意味論（安定順・last-wins）は暗黙の実装挙動であり、中立契約として明文化されていない。
- 連続パラメータの coalescing（容量超過時）は #190 spec で本 Issue へ明示的に延期されている。
- Plugin の processing 外状態（非 Processing/Failed/準備未了）で配送されたイベントの扱いは「破棄または silence 代替 + エラー返却」だが、呼び出し側への再送ポリシー指針がない。

## Alternatives and Comparison

制御値の配送・評価経路として、既存資産をどう組み合わせるかの比較。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. `TransportPlan<EventKind>` + `install_plan`（#190/#196 経路）に乗せる | 既存の generation/stale/budget/receipt/span 分割を全部再利用。sample-accurate。offline/live 同一評価が可能 | Plan は immutable で install 単位。高頻度更新は plan 差し替えコスト。pre-allocated vault 容量管理が必要 | **yes（timed 配送の正本）** |
| B. `NodeEvent`（typed runtime の generation 付き SPSC）で即時配送 | 即時・軽量・generation チェック済み | タイミング情報なし（callback head 適用）。`opcode+u32/f32` のみで ParamId/値域/精度を表現できない。#196 設計で plan/command 輸送用途を明示拒否済み | maybe（即時 Fast Control 用途のみ、拡張は別Issue） |
| C. 新規 Control Graph クレート（Source→Transform→Target DAG + RT 評価）を先行実装 | Synvelis #6 の Modulation 評価を製品非依存で再利用可能 | 実需が現状 1 consumer（Synvelis 未実装）のみ。評価順・合成規則・型は製品方針と不可分で過早抽象化リスク大 | **no（本 Issue では PoC 評価のみ、実装は分割 Issue 候補）** |
| D. 既存 `cap.rs` をパラメータ単位へ拡張 + plugin-core に capability/rejection 型追加 | 層ルール適合（中立契約は plugin-core/media-core、consumer 機構は bridge）。最小差分 | 小規模でも API 追加は後方互換ポリシーが必要 | **yes** |

## Tentative Conclusions and Hypotheses

- **[confirmed]** Timed ParamEvent の配送は Option A を正本とする。#190 の `ScheduledEvent<EventKind>`/`TransportPlan`/`install_plan` と #196 の `build_call_event_list` で「複数時刻・複数対象の ParamEvent が順序・位置・値通りに届く」ことが実 API で構成可能。
- **[confirmed]** 不足は配送骨格ではなく周辺契約: (a) パラメータ Capability Matrix の照会型、(b) ParamId↔TargetId 写像と未解決状態、(c) host-resolved value の scheduled/dispatched/reported 区別と照会、(d) 容量超過時の defined rejection/coalescing、(e) plugin→host パラメータ報告、(f) 規格差分の正直な宣言。
- **[confirmed]** `PortFormat::Control`/`SignalDomain::Control` は M1 でマーカーのみ。typed payload は M3（本 Issue）で定義してよい領域だが、今回の実需は EventList 経路で充足するため新ポート型の先行実装は不要。
- **[confirmed]** 非RT 準備/RT 有界評価/旧 Plan 維持/retire は #190/#196 で実装・検証済み（TransportPlanBuilder、PlanVault、pop_receipt/pop_fault、rt_ テスト）。本 Issue は「パラメータ用途での E2E 実証」を追加する。
- **[hypothesis]** 汎用 Control 評価基盤の共通化は現時点で不要。最小利用者は Synvelis のみで、合成ポリシー（既定加算）は製品所有。Ruviform は「Source/寄与/Target/評価結果」の語彙と「非RT 構築・検証 + RT 有界評価」の境界契約のみを将来の分割 Issue に残す判断が妥当。
- **[confirmed]** Synvelis 固有の Reference Value 合成・Automation Lane・Touch/Latch/Undo/プリセットは境界 ADR により Ruviform へ持ち込まない。

## Open Questions

- Plugin 側からのパラメータ報告（CLAP output events/gesture/rescan、VST3 outputParameterChanges/IComponentHandler）を中立契約へ追加するか。本 Issue では「観測可能性の区別を API で表現する」までとし、報告経路の実装は別 Issue 分割が妥当か → design で扱う。
- L1 ワイヤへ capability 照会 verb を拡張するか（bus enumeration が Unavailable なのと同様、param capability も L1 では Declared 止まりか）→ design で扱う。
- Coalescing を「どのイベント種・条件で許すか」の最終方針（連続 ParamValue 同一 param のみか、last-wins 圧縮か）→ design + PoC で決定。

## Promotion Candidates

- [x] spec-doc needed: Capability Matrix 照会・TargetId 写像・defined rejection・観測区別・配送検証の受入条件
- [x] adr-doc needed: 契約の配置（plugin-core 中立型 + bridge consumer 機構）と「汎用 Control Graph クレート不採用（現時点）」の決定

## Sources and Relations

- Issue: ruviform#186（本棚卸しの要求元）、synvelis#6（consumer 側 Parameter Control Model）、synvelis#18/#20/#22（Timed Control と generation 照合の設計要求）、ruviform#185/#188/#189/#190/#193/#196。
- 実コード（main@1305538 時点）: `crates/ruviform-plugin-core/src/{param,event,transport,transport_bridge,plugin,processor,lifecycle,boundary,plain_param}.rs`、`crates/ruviform-media-core/src/{transport_schedule,music}.rs`、`crates/ruviform-plugin-host/src/{endpoint,host}.rs`、`crates/ruviform-audio-plugin-bridge/src/{node,control,cap,attach,vault,status}.rs`、`crates/ruviform-plugin-clap/src/{adapt,plugin,host}.rs`、`crates/ruviform-plugin-vst3/src/{block,component,host}.rs`、`crates/ruviform-audio/src/typed/{runtime,probe}.rs`、`crates/ruviform-graph/src/port.rs`。

## Evidence

以下は main@1305538 の実コード読取結果。各項は Issue #186 検討範囲 1〜8 に対応する。

### E1. Capability Matrix（検討範囲 1）

- 既存: `ruviform_audio_plugin_bridge::cap` が `AdapterCapability{bus_enumeration, multi_bus, aux_sidechain, events, in_block_transport, min_frames_enforced, state_save_restore, crash_isolation, notes}` を `capability_for(Backend::{L0,L1,Clap,Vst3})` で返す。`CapabilityLevel::{Verified,Declared,Unknown,Unavailable,Unsupported}` で Verified のみ supported 扱い。行列はコードが正本。
- パラメータ記述: `ParamInfo{id:ParamId(u32), name:&'static str, unit:&'static str, min/max/default: f64(plain), flags:ParamFlags}`。`ParamFlags` は `AUTOMATABLE|MODULATABLE|STEPPED|HIDDEN|READONLY|BYPASS` の 6bit のみ。`from_bits` は未定義 bit を拒否。
- 値域: `ParamValue` は normalized f64 [0,1]。NaN/±inf は `ParamValue::new`/`ParamInfo::normalize` で `Err(Error::InvalidValue("param_value"))`、有限範囲外は clamp。`denormalize` は線形。`STEPPED` の量子化粒度（VST3 stepCount、CLAP IS_ENUM/choices）は中立型に載らず消失。
- **不足**: (a) param 単位で「どの更新手段を受け付けるか」（timed EventList / 即時 / block-boundary のみ / per-note）を表す型がない。(b) `READONLY`/`STEPPED`/`AUTOMATABLE`/`MODULATABLE` の更新可否への意味付けが照会 API として存在しない。(c) 「配送保証・未対応・fallback」を返すクエリがない。(d) adapter 行列は `events` の有無だけで sample-accurate/per-note/block-boundary の精度区別を持たない。

### E2. Parameter ID 境界（検討範囲 2）

- `ParamId(pub u32)` は plugin ローカル番号。CLAP は `clap_param_info.id` をそのまま `ParamId(u32)` へ写像（cookie は破棄）。VST3 も `ParameterInfo.id` をそのまま使用。`PluginHandle{slot,generation}` が instance 識別。`NodeId` は graph 内 node 識別。
- **不足**: plugin 構成変更（params() 内容の更新）や不在/無効 plugin、state 復元時の「TargetId→ParamId 未解決」を表す型・規約がない。アプリ内安定 Target ID は Synvelis の `ParameterTarget` 所有と設計境界で確定しているが、Ruviform 側の「binding」（TargetId→(PluginHandle,ParamId) + binding generation）の語彙が未定義。

### E3. Control/Event Pipeline（検討範囲 3）

- 非RT 準備: `TransportPlanBuilder::new(frame_base, generation)` → `event(MediaTime, source_sequence, payload)` / `bound_event` / `build()`（(position_ticks, source_sequence) でソート、plan_generation 全 event に刻印）。`install_plan`/`request`/`preflight`/`device_restart` は非 RT。`PluginNodeControl::{request, install_plan, set_clock, device_restart}` が bounded SPSC 経由で RT 側へ伝達（`CTRL_DRAIN_MAX=32`/callback、`QueueFull` defined rejection）。
- RT 配送: `TransportScheduler::run_callback(start, frames, sink)` → span 分割（min/max 適合 `BlockContract::partition`）→ `build_call_event_list(span, events, call_start, len, scratch, kind)` で `EventList`（process-start-relative、非減少、`offset<block_frames`、0-frame flush）→ `PluginEndpoint::process(inputs, outputs, events, Some(transport))`。
- 同一 offset: `EventList` は非減少順のみ要求。安定ソートで同一 offset の入力順を保持。「同一 param に同一 offset で複数値」は VST3 gain 実装が last-wins を採用（block 全体に最終値、補間なし）— 中立契約として未明文化。
- 競合・容量: `event_scratch` は `budget.max_events_per_span` で事前確保。`build_call_event_list` は scratch 不足で `None` → bridge は `EventScratchOverflow` report + 空リストで継続（defined drop）。`max_events_per_span` 超過は `EventBudgetExceeded` fault。
- **不足**: 0-frame flush・block 境界・同一 offset 競合の中立仕様記述は EventList モジュールにあるが、param 配送利用者向けの「どう使うか」の契約がない。即時（非 timed）更新経路は `NodeEvent` のみで ParamEvent を運べない。

### E4. タイムスタンプ・精度（検討範囲 4）

- `BeatPosition{num:i64,den:u32}`（有理数・全順序・負 pre-roll 可）。`TempoMap::beat_to_media(pos, TimeBase)` が単一 `NearestEven` 丸め（≤0.5 tick）で変換。`time_info(beat, ppqn)` が `MusicalTimeInfo{position_ticks,bar_start_tick,bpm_milli,ppqn,sig_num,sig_den}` を生成。固定 PPQN は無し。
- `call_transport_info(ctx, offset, tempo_map, ppqn)` で per-call `TransportInfo` を構成（loop_range・musical 任意）。`span_event_window`/`event_engine_frame` が loop 内イベントを occurrence ごとに 1 回配送。
- `EngineFrame{epoch,frame}` が frame 位置の唯一権威。`ClockEpochId` は device_restart で更新。`TransportGeneration` は Play/Pause/Stop/Seek で更新、loop wrap と SetLoop では据置。`PlanGeneration` は plan 適用で更新。
- offline/live: 同一 `TransportPlan` + 同一 cursor なら `run_callback` の span/event は決定的 → offline render との再現性は構造上確保。
- **不足**: tempo 変更中の「評価時 beat→frame 変換結果の安定性」「非同期（RT 外）の control 更新がどの frame に着地するか」の利用者向け規約。L1 の lookahead latency（`latency_frames`）とイベント時刻の関係の明文化。

### E5. 評価基盤の再利用可能性（検討範囲 5）

- 現状、Source→Transform→Target の評価 DAG、寄与合成、型/範囲チェック、cycle rejection の実装は Ruviform に存在しない。`PortFormat::Control` はマーカー（typed payload は M3+ 未定義）。
- 再利用可能な部品: generation タグ付き SPSC（RuntimeCommand/NodeEvent の stale 切断）、`SharedStatus`/`RuntimeFault` の fault bit パターン、`ProbeDeck`/`ProbeBroker` の tap 解決、`PlanVault` の publish/retire、`SnapshotWriter/Reader` の plan スナップショット交換。
- **判断**: 評価基盤の共通化は「実需が Synvelis 1 件のみ・合成規則が製品方針依存」のため本 Issue では実装しない（ADR で記録）。代わりに「Target/寄与/Host-resolved」の語彙と「非RT 構築・検証 + RT 有界評価 + 旧構成維持 + retire」の境界契約だけを設計する。

### E6. ホスト解決値の可観測性（検討範囲 6）

- 現状の観測面: `AppliedReceipt{status: Requested/Pending/Applied/Rejected, requested_frame, applied_frame, generations, degradation, reason}`、`ScheduleFault{kind,at,detail}`、`NodeReport{Receipt,Fault,UndeliverableSpan,EventScratchOverflow,EndpointError,VaultExceeded}`、`NodeStatusSnapshot` counters、`SharedStatus`/`RuntimeFault` flags、`audio_counters`（L1 block telemetry）。
- 値の追跡: 「いつ・どの値を送る予定か」「実際にどの call に載ったか」「plugin が何を報告したか」を区別する API がない。CLAP adapter は `ParamMirror`（AtomicU64 f64 bits）で adapter 内部の値鏡像を持つが中立契約ではない。
- `typed::probe`（#185）は信号（audio buffer 等）の tap/capture であり、ホスト解決値・plugin 報告値とは別責務 — 混同しない（Issue 明示）。
- **不足**: scheduled（plan 内 pending）/dispatched（実コールへ載った、frame 確定）/reported（plugin 由来、現状ほぼ Unavailable）の区別と、評価時刻・revision・欠落・非対応の照会 API。Plugin 内部真値は保証しない前提。

### E7. RT 安全と更新の原子性（検討範囲 7）

- no-alloc/no-block: `run_callback`、`build_call_event_list`、`call_transport_info`、`event_engine_frame`、`sort_events_by_frame_offset`（scratch merge sort）、`pop_receipt`/`pop_fault`（bounded remove(0)）は allocation-free。`PluginNode::process` は全体で alloc なし設計。
- 容量超過: `QueueFull`（control channel）、`BudgetExceeded`/`DeferralExceeded`（scheduler stage）、`EventBudgetExceeded`/`SpanBudgetExceeded`/`FaultLedgerOverflow`（RT fault）、`EventScratchOverflow`/`VaultExceeded`/`ReportOverflow`（bridge report/counter）。
- publish/retire: `Arc<TransportPlan>` immutable + `PlanVault`（capacity 64、RT 側で destructor 不実行）+ snapshot channel の retire + `GraphRuntime::collect`（non-RT drop）。
- 旧実行構成: stage 拒否時は last-good 継続（`Rejected` receipt）。`device_restart` で epoch 切替 + pending を StaleGeneration 拒否。
- 過負荷測定: `ScheduleBudget{max_spans_per_callback:64, max_events_per_span:256, max_pending_applies:16, max_deferral:4096}`（Default）。RT 側は全てこの bound で有界。

### E8. 規格 Adapter 差分（検討範囲 8）

| 項目 | L0 (in-process) | L1 (out-of-process) | CLAP adapter | VST3 adapter |
| --- | --- | --- | --- | --- |
| ParamEvent 配送 | EventList Verified | EventList Verified（shm、`WIRE_MAX_EVENTS=1024`。表現不可は `Ipc`→`Prepare(UnsupportedConfig)`） | `push_events`→`ParamValueEvent`（Declared）。`MAX_EVENTS_PER_BLOCK=256` 超過は silent drop。per-note `pckn` 常時 global | `fill_param_queues`→per-param `IParamValueQueue`（Declared）。queue あたり 1024 上限。foreign id/範囲外 drop |
| Plugin 側受信 | — | — | `translate_events`: header.time→frame_offset 保存。>256 は `CLAP_PROCESS_ERROR` defined reject。unsorted は安定再ソート | `collect_param_events`: `IParamValueQueue::getPoint` 走査。`MAX_BLOCK_EVENTS=1024`+`MAX_PARAM_SCAN_OPERATIONS=8192` 上限（超過は収集済み保持で打切り） |
| 同一 offset 競合 | EventList 安定順 | 同左 | ParamValue のみ。processor last-wins | host 順を保持し processor last-wins（補間なし） |
| per-note modulation | EventKind に語彙なし | 同左 | `pckn` を `match_all` に潰す（global 適用）、`IS_MODULATABLE_PER_*` flag 消失 | VST3 note-expression / `kIsProgramChange`/`kIsWrapAround` 未写像、`stepCount`→`STEPPED` 化で粒度消失 |
| plugin→host 報告 | 経路なし | 経路なし（wire verb 無し） | output events を `OutputEvents::void()` 破棄、gesture/rescan 未実装、`ParamMirror` は adapter 内部鏡像 | `outputParameterChanges`/`inputEvents`/`outputEvents` 常に null、`IComponentHandler` begin/perform/end は ack して破棄（`restartComponent` は kNotImplemented） |
| transport | `TransportInfo` per call | 同左 | `process.transport` から adapt（`CLAP_EVENT_TRANSPORT` in-events は skip）。bridge は常に `transport=None`（free-running 境界を正直に宣言） | `processContext` 未接続 |
| latency | `latency_frames=0` | `lookahead_blocks×max_frames`（default 1024） | 未照会 | plugin `getLatencySamples` 未照会（中立 Plugin 面に accessor 無し） |
| 値の精度 | f64 normalized | 同左 | plain f64 ↔ normalized f64、processor 内部 f32 へ defined narrowing、text は 4 桁 | normalized f64 直通、`normalizedParamToPlain(0)/(1)` の端点のみで plain 域を推定（非線形不可視）、PPQ↔tick は f64 経由の丸め |
| 非対応状態 | 非 Processing→`Err(Lifecycle)`、Failed→silence+`Ok(CONTINUE)`、busy→`Err(EndpointUnavailable)` | 同左 + unrepresentable block→`Err(Ipc)` | 非 Processing→`kResultFalse`、非 audio thread→defined rejection（thread-check 拡張照会時） | 非 Processing/再入/形式不正→`kResultFalse`/`kInvalidArgument` |

### E9. 消費側（Synvelis #6）が Ruviform に要求する最小契約

- `ParameterTarget` 相当の stable Target ID と plugin binding（binding generation を gesture 開始時に固定 — Synvelis #20）。
- Reference Value→各 Source 寄与→**Host-resolved Value** の評価語彙と、評価時刻・revision・stale/unavailable の照会。
- 対応しない Parameter/精度（ReadOnly、Stepped 連続 modulation、per-note）への defined rejection/制限返却。
- Timed ParamEvent の配送保証（frame offset、同一 offset order、容量、block/sample-accurate capability）。
- 送信予定/送信済み/plugin 報告値の区別（Plugin 内部真値は無保証）。

### E10. 後方互換と機能不足一覧（Issue AC「既存 Event/Plugin Host/Adapter API の後方互換と機能不足の一覧を更新」）

- **互換維持が必要な契約**: `EventList` wire 形状（`frame_offset<u32`、非減少、`offset<block_frames`、0-frame flush、`EventKind::ParamValue` エンコード `u32+f64`）、`ParamValue` の NaN/clamp ポリシー、`Plugin::params()` の列挙形、`PluginEndpoint::process` シグネチャ、`HostError` 意味、`TransportInfo` スナップショット、`ScheduledEvent`/`TransportPlan` 型、bridge の NodeControl/NodeReport 語彙。
- **機能不足一覧（本 Issue で扱う範囲）**:
  1. パラメータ単位の delivery capability/precision 照会（accept means, precision, guarantee, fallback）
  2. ParamId↔TargetId 写像と未解決状態の型
  3. host-resolved value の scheduled/dispatched/reported 区別と照会
  4. 容量超過時の param coalescing/defined rejection 方針
  5. 同一 offset 同一 param 競合の中立規約（last-wins を明文化するか）
  6. plugin→host パラメータ報告経路（READONLY を実質化）
  7. 規格差分の正直な宣言拡張（per-param precision、per-note、in-block transport、latency）
  8. `EventKind` の将来拡張余地（per-note/MIDI は #188 側・`reserved` フィールドで吸収予定）
