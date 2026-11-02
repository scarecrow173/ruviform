---
id: "DESIGN-034QkB2r7d3Qh9nZ3zJ3rP"
type: "design"
status: "approved"
title: "Issue #186 Parameter Control Events — detailed contract design"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/param_write.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/target.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/dispatch.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/cap.rs"
      - type: "file"
        path: "crates/ruviform-audio-plugin-bridge/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-189-rational-beat-variable-ppqn-tempomap.md"
    - "docs/designs/issue-190-transport-span-scheduler.md"
    - "docs/designs/issue-196-typed-graph-plugin-adapter-multi-bus-lifecycle-process-offset.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md"
    - "docs/designs/issue-186-param-control-events.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-186-param-control-events.md"
    - "docs/adr/parameter-control-events-contract-placement-and-deferral-of-generic-control-grap.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/discovery/issue-186-param-control-events-inventory.md"
  defers: []
  deferred-by: []
metadata:
  gate: "M3"
  issue: 186
---

# Issue #186 Parameter Control Events — detailed contract design

## Context

Issue #186 の8検討範囲を実 API 上の契約へ落とす。ADR Option A（plugin-core 中立型 + bridge consumer 機構、新クレートなし、`TransportPlan<EventKind>` 経路正本）に従う。本書は Capability Matrix・正規化値・Time Domain・同一 offset・defined rejection・観測 stage・規格差分を実コード根拠で規定する（AC-2）。

## Scope

### In Scope

- `ruviform_plugin_core::param_write` モジュールの API 形と意味規則。
- `ruviform_audio_plugin_bridge::{target, dispatch}` の API 形と stage 遷移。
- `cap.rs` のパラメータ精度拡張と全 backend の現行値。
- 同一 offset 規約・coalescing・時間変換規約の中立仕様。

### Out of Scope

- plugin→host 報告配線、per-note 語彙、Control Graph クレート、`NodeEvent` 拡張、L1 verb、Synvelis 固有型。

## Components and Boundaries

### 1. Capability Matrix（パラメータ単位）

中立 write 手段:

```rust
#[non_exhaustive]
pub enum ParamWriteMeans {
    TimedEvent,      // EventList の frame_offset 付き配送（sample-accurate 経路）
    BlockBoundary,   // ブロック先頭のみ有効な block-rate 更新（offset 常時 0 に劣化）
    Immediate,       // 非時刻付き即時 write — 中立契約に動詞なし
    PerNote,         // per-note/per-voice ターゲティング — EventKind に語彙なし
}
```

`check_param_write(info: &ParamInfo, means: ParamWriteMeans) -> Result<(), ParamWriteRejection>`:

| means | AUTOMATABLE | READONLY | 結果 |
| --- | --- | --- | --- |
| `TimedEvent` | set | clear | Ok |
| `TimedEvent` | set | set | `Err(ReadOnly)` |
| `TimedEvent` | clear | clear | `Err(NotAutomatable)` |
| `BlockBoundary` | any | set | `Err(ReadOnly)` |
| `BlockBoundary` | clear | clear | Ok（block-rate 直接 write 相当。automation 保証なし） |
| `Immediate` | any | any | `Err(UnsupportedMeans)` — 中立 write 動詞が存在しない |
| `PerNote` | any | any | `Err(UnsupportedMeans)` — `EventKind` に語彙なし |

`ParamWriteRejection`（`#[non_exhaustive]`）: `ReadOnly`, `NotAutomatable`, `UnsupportedMeans`, `UnsupportedPrecision`, `UnknownParam`, `NonFiniteValue`。`ParamValue::new` の NaN/inf 拒否は既存 `Error::InvalidValue("param_value")` と整合（check 内で `NonFiniteValue` に翻訳する経路を提供しない — 値検証は `ParamValue::new` が単一権威。`check_param_write` は手段×flags のみ判定）。

`STEPPED` の意味: 値の量子化規則（step 数・許容値）は `ParamInfo` が保持しないため中立基盤は量子化を適用しない。consumer が規則を知る場合のみ write 前に量子化し、量子化不能な連続要求は consumer 側で rejection とする（VST3 stepCount/CLAP enum は現行 adapter で消失、deferred）。

`MODULATABLE` の意味: 規格側 per-note/per-voice modulation 能力の宣言で、グローバル timed write の可否とは無関係。中立語彙に per-note 配送がないため modulation 要求は `PerNote`→`UnsupportedMeans`。

adapter 行列拡張（`cap.rs` の `AdapterCapability` に追加、`Backend` 別現行値）:

| フィールド | L0 | L1 | CLAP | VST3 |
| --- | --- | --- | --- | --- |
| `param_event_precision` | Verified | Verified（shm `WIRE_MAX_EVENTS=1024`） | Declared（`MAX_EVENTS_PER_BLOCK=256`、超過は defined reject または flush 経路打切り） | Declared（queue 1024、scan 8192 上限） |
| `per_note_param` | Unsupported | Unsupported | Unsupported（`pckn` 消失・global 化） | Unsupported |
| `param_reporting` | Unavailable | Unavailable | Unavailable（output events 破棄） | Unavailable（outputParameterChanges null） |

照会経路: consumer は `param_capability(backend, info)` 相当の合成（adapter 行列 ∩ `check_param_write`）で「要求 means が Verified か」を判定。`Declared`/`Unknown` は supported 扱いしない（capability honesty）。

### 2. Parameter ID 境界

```rust
pub struct TargetId(pub u64);            // app-stable。値の生成規則は consumer 所有
pub struct BindingGeneration(pub u64);   // rebind ごとに単調増加
pub struct ParamBinding {
    pub target: TargetId,
    pub node: NodeId,
    pub param: ParamId,
    pub generation: BindingGeneration,
}
#[non_exhaustive]
pub enum TargetResolution {
    Resolved(ParamBinding),
    Unresolved,   // table に target 無し
    Defunct,      // binding 記録済みだが node/param が現行 enumerate で検証失敗
}
```

- `ParamTargetTable`（非RT・HashMap<Vec ベース小容量で可、PoC は `BTreeMap`）: `bind`/`rebind`（generation 増加）/`unbind`/`resolve`。
- 検証: `bind` 時に `param_info(node, param)` が存在すれば Resolved、`ParamId` 不明 → 即時 `UnknownParam` reject。node 不在・plugin detach → `Defunct`（silent でなく明示）。
- state 復元: consumer が復元シリアライズした (TargetId→ParamId) 候補を `rebind` へ。`ParamInfo` 列挙と不一致なら `Defunct` を返して UI/consumer に未解決を返す — 誤 target への silent 適用禁止（Issue 制約・synvelis#20 の「gesture 開始時に binding 固定」は consumer が generation を記録して実現）。

### 3. Control/Event Pipeline と同一 offset 規約

- **非RT**: capability 照会 → `check_param_write` → `TempoMap::beat_to_media` → `TransportPlanBuilder` 構築 → `install_plan`（`PlanVault` keep-alive、capacity 超過は `VaultExceeded` defined rejection）。
- **RT**: `run_callback` が span を `BlockContract::partition` で物理 call に分割 → `build_call_event_list` で process-relative `EventList`（`frame_offset<block_frames`、非減少、0-frame ブロックは offset 0 flush）→ `endpoint.process`。
- **同一 offset 規約（中立・明文化）**: `EventList` は非減少 `frame_offset` のみ要求し、安定ソートが同 offset の source 順を保持する。同一 `ParamId` に同 offset で複数 `ParamValue` がある場合、processor 適用意味論は **source 順 last-wins**（実装は VST3 gain と一致、補間なし）。`source_sequence` が大きい方が勝つ。異なる `ParamId` 同士の順序は意味を持たない（独立 param）。
- **0-frame flush**: 全ブロック長 0 の call は offset 0 のイベントのみ合法で即時 flush を意味する（既存 `EventList` 規約そのまま）。
- **coalescing（opt-in）**: `coalesce_param_events_latest(&mut Vec<(source_seq, pos, ParamEvent)>)` 相当のヘルパを plan-build 側に提供。同一 `ParamId` 連続 `ParamValue` のみを対象に、同一 position 内で最新値のみ残す。別 param を挟むと不可・`ParamValue` 以外の `EventKind` 混在時は不適用。既定は圧縮なし（overflow → `EventBudgetExceeded`/`EventScratchOverflow` defined fault）。

### 4. Time Domain

- 音楽位置: `BeatPosition{num,den}`（有理数、負 pre-roll 可）。→ `TempoMap::beat_to_media(beat, TimeBase)` で `MediaTime` へ（単一 `NearestEven` 丸め、誤差 ≤0.5 tick）。`MusicalTimeInfo` 経由の `ppqn` は可変・固定前提なし。
- 媒体位置: `ScheduledEvent.position` は `MediaTime`（tick）。`span_event_window` + `event_engine_frame` が span の `context.position` から `EngineFrame` へ変換。call 内 offset = `event_frame - call_start`。
- テンポ変更: `TempoMap` は `install_plan` とは独立に参照される（`call_transport_info` が毎 call tempo_map から `TransportInfo` 構成）。**イベントの frame 位置は plan 構築時の変換で固定** — tempo map 差替えは `TempoMapRevision` として identity 分離（#190 ADR）。plan 内イベントを別 tempo に追従させたい場合は新 plan を install（旧 plan は generation 照合で stale）。
- Loop: `transport_generation` は loop wrap/SetLoop で据置。`span_event_window` が half-open span と `LoopOccurrence` ごとにイベントを 1 回ずつ展開 — 同一 position のイベントは occurrence ごとに再発火。
- レイテンシ: `TransportInfo`/イベント offset は plugin processing 座標。adapter 側の lookahead（L1 `latency_frames`）は delivery の frame を遅らせるが offset 計算座標は変わらない — consumer は `latency` を device/backend 照会で別途取得（neutral `Plugin` 面に accessor なし、VST3 `getLatencySamples` 未接続、declared gap）。
- offline/live 再現性: 同一 `TransportPlan` + 同一 cursor/contract なら `run_callback` は決定的 → offline render と live で同一 dispatch 投影。

### 5. Host-resolved 観測（scheduled/dispatched/reported）

```rust
#[non_exhaustive]
pub enum DispatchStage {
    Scheduled,                 // plan に含まれる・まだ call 未通過
    Dispatched { call_start: u64, frame_offset: u32 }, // 物理 call への投影確定
    Dropped { reason: DropReason }, // budget/scratch/stale/undeliverable
    Reported,                  // plugin 報告値あり（payload は ParamValue）
    Unavailable,               // 報告経路なし・stage 未定義
}
pub struct ParamValueObservation {
    pub target: TargetId,
    pub stage: DispatchStage,
    pub value: Option<ParamValue>,   // Scheduled/Dispatched=host-resolved 値, Reported=plugin 値
    pub revision: u64,               // ledger 内 revision（install ごとに増分）
    pub position: Option<MediaTime>,
}
```

- `ParamDispatchLedger`（非RT）: `record_plan(node, plan, target_map)` → 各 `ScheduledEvent<EventKind::ParamValue>` を `Scheduled` で記録 → `project(node, contract, span_start, frames)` が `event_engine_frame`+`partition` を再計算して `Dispatched{call_start,frame_offset}` へ進める → `apply_report(NodeReport)` が `EventScratchOverflow`/`UndeliverableSpan`/`EndpointError`/fault を `Dropped{reason}` へ写像 → `record_reported(target, value)`（将来の plugin 報告経路用・現行は `Unavailable`）。
- **誠実さ**: `Dispatched` は wire ack でなく plan+contract からの deterministic projection — plugin 側到達確認ではない。`Reported` は「plugin が主張した値」であり内部真値の保証ではない。`Unavailable` は経路不在を表す（silent な None ではなく明示 stage）。
- 区別の確認: Scheduled=送信予定、Dispatched=送信確定（call/offset）、Reported=報告値、Unavailable=照会不能 — Issue の「送信予定/送信済み/Plugin 報告値の区別」を stage で表現。

### 6. 規格差分（実コード実測・誇大禁止）

| 項目 | L0 | L1 | CLAP | VST3 |
| --- | --- | --- | --- | --- |
| ParamEvent 受付 | EventList Verified | EventList Verified | Declared（256/block） | Declared（1024/queue, scan 8192） |
| sample-accurate | あり | あり（wire 表現範囲内） | offset 保持・plugin 依存 | offset 保持・queue point 単位 |
| per-note modulation | 語彙なし | 語彙なし | `pckn`→global 潰れ | 未写像 |
| block-boundary のみ更新 | 可能（offset 0） | 可能 | 可能 | 可能 |
| plugin→host 報告 | 経路なし | verb なし | void discard | null/ack-discard |
| latency | 0 | lookahead | 未照会 | 未照会 |
| 非対応状態 | `Err(Lifecycle)`/`Err(EndpointUnavailable)` | `Err(Ipc)` 等 | `kResultFalse` | `kResultFalse`/`kInvalidArgument` |

## Data and Control Flow

```
consumer (non-RT)
  │ resolve(TargetId) → ParamBinding | Unresolved | Defunct
  │ check_param_write(info, TimedEvent) → Ok | ParamWriteRejection
  │ beat_to_media(BeatPosition) → MediaTime
  │ TransportPlanBuilder.event(pos, seq, ParamEvent) → build() → install_plan
  ▼
RT callback (PluginNode::process, no-alloc)
  │ drain control (install_plan, ≤CTRL_DRAIN_MAX)
  │ run_callback(span) → partition → build_call_event_list → EventList
  │ endpoint.process(inputs, outputs, events, transport)
  │ overflow/stale → NodeReport (bounded)
  ▼
non-RT observability
  │ pop_report → ParamDispatchLedger.apply_report → stage 遷移
  │ ledger.query(target) → ParamValueObservation{stage,value,revision,position}
```

## Non-Functional Constraints

- RT: no-alloc/no-block/no OS I/O。全 bounded（`CTRL_DRAIN_MAX`, `event_scratch`, `PlanVault` cap, budget）。超過は defined fault/report。
- API: 公開 enum `#[non_exhaustive]`、doctest 付き、`unsafe` 禁止。
- compat: `EventList` wire・`ParamValue` 規約・`Plugin::params()`・`PluginEndpoint::process`・`TransportPlan` は不変（追加のみ）。
- consumer 影響分類: synvelis = impacted（新契約の第一 consumer）、audio-lens = no impact（既存 API 不変・追加のみ）。検証不能な部分は unverified 明記。

## Risks and Trade-offs

- `Dispatched` が投影である点を誤読するリスク → 型名・doc で projection と明記、`Reported`/`Unavailable` と区別。
- `check_param_write` と `ParamValue::new` の責務分担（means×flags vs 値検証） → 値検証は `ParamValue::new` 単一権威のまま。check は値を見ない。
- `TargetId` を u64 で受けるが生成規則は consumer — Ruviform は uniqueness を保証せず binding table が last-write（generation 増分）で追跡。
- plan 差替え頻度: 高頻度連続変化は plan install が非RT コスト → PoC では automation-block 単位の batch install を推奨する設計注記（連続更新最適化は将来）。

## Deferred Design Concerns

- plugin→host 報告経路（CLAP output events/rescan、VST3 outputParameterChanges/handler）— `record_reported` の実配線。trigger: READONLY 実質化・synvelis#22 の実装。
- `NodeEvent` param 拡張（即時 write）— trigger: gesture-follow 実需。前提: 表現力（ParamId/f64/タイミング）不足の解消。
- `PortFormat::Control` typed payload — trigger: node 間 control 伝送の実需。
- 汎用 Control 評価基盤 — trigger: 2 consumer 目 or Synvelis PoC 実需、ADR で分割提案。
- `STEPPED` 量子化粒度の中立表現 — trigger: consumer が step 数を必要とした時。
- L1 wire capability/param verb — trigger: L1 での照会実需。

## Verification Notes

- [ ] `check_param_write` 全組合せテスト（means×flags×reject 種別）
- [ ] binding: resolved/unresolved/defunct/generation 増分・旧 generation reject
- [ ] E2E: multi-time multi-target ParamEvent が期待 offset/順序/値で L0 plugin 着弾
- [ ] 同一 offset last-wins 順序保持・coalescing opt-in 挙動
- [ ] beat→frame/loop/tempo 変更/block 境界/非同期 install の再現テスト
- [ ] rt_: dispatch 経路 no-alloc、vault keep-alive/retire、bounded fault
- [ ] ledger stage 遷移（Scheduled→Dispatched→Dropped/Reported/Unavailable）
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` green
