---
id: "TSPEC-034QkFscAwaBuZWB5aEeBy"
type: "test-spec"
status: "approved"
title: "Issue #186 Parameter Control Events — test spec"
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
    - "docs/plans/issue-186-param-control-events.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-186-param-control-events-detailed.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-186-param-control-events.md"
    - "docs/designs/issue-186-param-control-events-detailed.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #186 Parameter Control Events — test spec

## Purpose

Issue #186 の受入条件（AC-3/4/5/6/7）を検証するテストの規約。パラメータ制御イベント契約が誇大な互換性保証・silent 誤適用・RT 違反をしないことを固定する。契約が変更されるまで維持。

## Feature

Parameter control events — per-param capability, TargetId binding, timed delivery via TransportPlan<EventKind>, defined rejection, and scheduled/dispatched/reported observability.

## Rules

- Rule: `check_param_write` は design §1 行列に厳密一致し、`READONLY`→`ReadOnly`、非`AUTOMATABLE` への `TimedEvent`→`NotAutomatable`、`Immediate`/`PerNote`→`UnsupportedMeans` を返す。
- Rule: 未解決/Defunct/旧 generation の Target への配送要求は defined rejection で、silent fallback・誤 target 適用をしない。
- Rule: Timed `ParamEvent` は `frame_offset` が `event_engine_frame - call_start` に一致し、イベントは非減少 offset・安定 source 順で `EventList` に載る。同一 offset 同一 param は last-wins。
- Rule: `ParamValue` は normalized [0,1]、NaN/inf 拒否・有限範囲外 clamp の既存規約を変えない。
- Rule: `BeatPosition`→frame 変換は `TempoMap::beat_to_media`（単一 NearestEven 丸め）に従い、Loop occurrence ごとにイベントが再発火し、tempo 変更は `TempoMapRevision` で identity 分離される。
- Rule: 容量超過は defined fault（`EventBudgetExceeded`/`EventScratchOverflow`/`VaultExceeded`/`QueueFull`）。opt-in coalescing は同一 param 連続値のみ latest-wins。
- Rule: RT 配送経路は no-alloc/no-block/no OS I/O で、旧 plan は vault keep-alive・retire は非RT。
- Rule: `ParamDispatchLedger` は `Scheduled`→`Dispatched`（deterministic projection）→`Dropped`/`Reported`/`Unavailable` を区別し、評価位置・revision を保持する。`Reported` は plugin 報告値で内部真値を保証しない。
- Rule: 規格差分は `cap.rs` が正直に宣言し、`Declared`/`Unknown`/`Unavailable`/`Unsupported` を supported 扱いしない。

## Examples

- Given: `ParamInfo{flags: READONLY}` When: `check_param_write(info, TimedEvent)` Then: `Err(ReadOnly)`。
- Given: AUTOMATABLE param 2個（id=1,7）と plan に3イベント（(pos=64,id=1,v=.5),(pos=64,id=7,v=.9),(pos=128,id=1,v=.2)）When: callback で 128-frame call 開始=0 Then: EventList=[{id1@64,.5},{id7@64,.9}]、2nd call=[{id1@0,.2}]。
- Given: 同 offset 同 param 2イベント（v=.3,seq=0;v=.8,seq=1）When: 配送 Then: 順序保持、processor 適用は .8（last-wins）。
- Given: 未 bind の TargetId When: `resolve` Then: `Unresolved`（配送は行われない）。
- Given: loop range [0,192) と event pos=64 When: 2回目の loop occurrence Then: event は occurrence ごとに1回発火。
- Given: budget.max_events_per_span=2 に3イベント When: dispatch Then: defined fault（`EventBudgetExceeded`）、silent drop なし。
- Given: ledger に pos=64 の Scheduled entry When: `project(span_start=0,frames=128)` Then: `Dispatched{call_start=0,frame_offset=64}`。When: `EventScratchOverflow` report Then: `Dropped`。
- Given: backend=Clap When: `capability_for` Then: `param_event_precision=Declared`,`per_note_param=Unsupported`,`param_reporting=Unavailable`。

## Guarantees

- 複数時刻・複数対象イベントの順序・位置・値が実 API 契約通りに保証される。
- 非対応 precision/type/state が全て defined rejection/fault を返し silent 不適用がない。
- 時間変換・Loop・tempo・block 境界・非同期 install が再現可能に検証される。
- dispatch 経路が `assert_no_alloc` で allocation-free、vault が publish/retire を安全に行う。
- 観測 stage の区別（送信予定/送信済み/報告値/不能）が照会可能。

## Non-goals

- plugin→host 報告の実配線（`Reported` の配線は別 Issue — API 形と `Unavailable` 既定のみ検証）。
- per-note/MIDI 系イベント、Control Graph 評価 DAG、Synvelis 固有 Write モード。
- CLAP/VST3 adapter 内部の規格固有挙動（中立契約の宣言のみ）。

## Risk

- 規約を落とすと: silent 誤適用（誤 param 値・誤タイミング）、RT 違反（alloc/block）、誇大 capability 宣言による consumer の誤判断、観測 stage の混同（dispatched を plugin 確認済みと誤読）。
