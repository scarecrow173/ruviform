---
id: "SPEC-034RToefGzxcXIANSfMf7U"
type: "spec"
status: "approved"
title: "Issue #279 OBS-04 Plugin Host既存4 CounterのSource登録・統合"
created: "2026-11-04"
updated: "2026-11-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/279"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/279"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-277-observability-core-source-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-121-l1-diagnostics.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-277-observability-core-source-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-279-plugin-host-counter-source-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/291"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/281"
  deferred-by: []
---

# Issue #279 OBS-04 Plugin Host既存4 CounterのSource登録・統合

## What Should Be Built

`ruviform-plugin-host` の L1 データプレーンが Issue #121 で持つ既存 4 診断
Counter（`dropped_submits` / `passthrough_submits` / `results_failed_observed` /
`results_dropped`）を、OBS-02（Issue #277・着地済み）の
`ObservationRegistrar`/`RegistrationLease` 契約を通じて Observation Source として
登録する。Plugin Host は OBS 統合設計（Issue #275）の最初の実消費者となり、
Domain 側が Descriptor・ReportingHint・SourceSemantics を宣言し、
Consumer（Collector/Exporter/Diagnostics）が変換・Delta・Severity を担う境界を
実装する。

## Why Now

- OBS-01（#276 logging）・OBS-02（#277 source 契約）は着地済み。Source/Lease
  契約の最初の実使用経路を固定し、OBS-07 Collector（#289）・OBS-03
  tracing_export（#278）が実装時にそのまま接続できる接続面を確立する。
- 4 Counter は既に `ruviform_observability::counter()` の実消費者であり、
  増分位置・意味・Golden は確定済み。登録経路を Domain 宣言として型化しないと、
  後続の Collector/Exporter 実装が意味付けを推測で補うことになる。

## Users and Value

- **OBS-07（#289）実装者**: 実 Consumer 経路での Source 登録/退避/Generation
  区切りの使用例を得る。Collector は SourceIdentity で同名異 Instance の
  Series を区別する前提を検証できる。
- **OBS-03（#278）/ OBS-09（#291）実装者**: Domain 宣言の
  Descriptor/ReportingHint/SourceSemantics を変換・Severity 判定の入力として
  利用できる。
- **Consumer（AudioLens/Synvelis）**: Counter→Log mapping や Delta/Severity
  計算を自前で持たず、Logging/Tracing OFF でも既存 Query と Metrics
  Snapshot が利用できる。

## Scope

### In Scope

- `PluginHost` への Observation Registrar 注入面（`Option<Arc<dyn ObservationRegistrar>>`、
  未接続 = 未登録で動作する既存互換）。
- 各 audio Instance mint（dedicated / bundled / supervisor restart の全経路）での
  4 Counter Source 登録と `RegistrationLease` の Instance facade 保存。
- Domain 宣言の 4 `MetricDescriptor`（key・unit・reporting・aggregation・semantics）
  — Issue #121 の絶対維持意味表に忠実。
- Instance destroy・failed-create rollback・re-mint（新 mint = 新登録）を
  Source 寿命イベントとして Lease に反映。writer quiescence（既存 destroy 経路の
  quiescence 点）→ `retire` の順序。
- 登録失敗（`RegistrationError` 各型）を typed な degrad として扱い、plugin
  processing を継続する経路。
- 登録・寿命・degraded 扱いの決定的試験（in-crate test registrar 使用）。

### Out of Scope

- OBS-07 Collector（#289）の実 Registry/Poll/Baseline/Rate/History/Snapshot
  実装・その試験 — 本 Issue は `ObservationRegistrar` ポートの Domain 側消費のみ。
- OBS-03 `tracing_export`（#278）の tracing Event 変換・event_code/schema
  一致検証 — 変換層未実装。
- OBS-05 RT Record Bridge（#280）、OBS-09 Diagnostics Health/Report（#291）、
  OBS-11 integration runner（#293）。
- child stderr 全文の自動 Trace Export、Consumer 独自 Delta/Severity 計算。
- AudioLens/Synvelis 実機経路での OBS-06（#281）証跡 — cross-repo gate は別 Issue。
- 新規 crate・RT leg の構造・増分位置・意味・既存 Query・ABI の変更。

## Requirements

- 絶対維持する L1 診断 Counter（Issue #121・Issue #279 本文の表）:
  - `dropped_submits`: slot busy/channel full 等の Submit 不成立。block あたり
    最大 1。XRUN 等の別事象と混同しない。
  - `passthrough_submits`: **実際に代替出力が成立した block 数**。warmup を
    含むため無条件 WARN は禁止。
  - `results_failed_observed`: RT が `RESULT_FAILED` 付き result を初めて pop
    して**観測した数**。全 Failure 発生件数ではない。
  - `results_dropped`: Reader 側の result enqueue 失敗による破棄数。すべての
    結果欠落数ではない。
- 既存 Query（`PluginHost::audio_counters`/`AudioCounterSnapshot`）、bounded
  child stderr 診断、FailureCause/DiagRegistry、Counter 増分点、Reset/再 mint、
  Plugin ABI、stdout/IPC、Audio/Video 独立境界を回帰させない。
- RT leg には事前準備済み Counter Writer のみ渡し、RT path 上に Registry
  Lock/Subscriber/動的確保/Callback を追加しない。登録・退避・Lease Drop は
  全て非 RT でのみ実行する。
- Source の epoch は Counter cell の epoch と一致させる: destroy（cell 所有
  entry 除去）→ writer quiescence → `retire`；re-mint（新 cell）→ 新規登録
  （新 `SourceIdentity`）。cell が継続する leg swap / endpoint detach / Failed
  convergence では再登録しない（同一 cell の再登録は `SourceReused` 競合かつ
  誤 Series 合成の原因になる）。
- 同名 key の別 Instance 登録・登録容量超過・Registrar 停止は型付き
  `RegistrationError` で返り、host は degrad（未登録 slot）として記録しつつ
  plugin processing を継続する。誤った Series を合成する fallback は作らない。
- `Collector`/`ObservationRegistrar` 未接続時（`None`）は登録を一切試みず、
  従来どおり動作する。

## Acceptance Criteria

- [ ] AC-1: 4 Counter の意味・増分位置・既存 RT API の同値 Golden を維持
  （`mise run test`/`test-rt`・既存 counter 試験が無改変で pass）。
- [ ] AC-2: Registrar 接続時、audio Instance mint ごとに 4 Source が Instance
  scope で登録され、SourceId/SourceGeneration が Instance 間で区別される
  （同名 key・異 Instance・re-mint で誤 Series を合成しない）。
- [ ] AC-3: `RegistrationError` 各型（stopped/capacity/conflict/reused 等）で
  plugin processing が継続し、degraded slot が型付きで観測できる。
- [ ] AC-4: destroy/failed-create で writer quiescence → `retire` の順序が
  守られ、Lease 未退避（drop）でも `abandoned` として可視化される（黙って
  消えない）。
- [ ] AC-5: Registrar 未接続（`None`）・登録失敗・Collector 側 drain 未実行でも
  既存 Query と AudioCounterSnapshot が同値で動作する。
- [ ] AC-6: leg swap / endpoint detach / Failed convergence で cell が継続する
  限り Source が継続し（再登録・重複 Series なし）、re-mint で新
  SourceIdentity が発行される。
- [ ] AC-7: RT path（`process`/leg writers）に Registry Lock・alloc・IO・
  Subscriber・Callback が追加されない（`mise run test-rt` レーン pass・
  `mise run lint` -D warnings 通過）。

## Deferred Design Concerns

- Collector Snapshot と tracing_export の event_code/schema/Severity 一致検証
  | OBS-07（#289）・OBS-03（#278）未実装 | 両 Issue 着地時に OBS-06 gate で検証 |
  先行して推測で変換層を作ると Domain/Consumer 責務境界が曖昧になる。
- warmup 期間の `Duration` 化 | warmup は `lookahead_blocks` 個の block であり
  mint 時点で sample rate 未定のため `Duration` 未確定 | Activate 後に
  semantics を供給する仕組みが必要になった時点 | Descriptor に warmup 構造を
  押し込むと mint 時の不変性が壊れる。
- AudioLens/Synvelis 実機経路の OBS-06 証跡 | cross-repo gate（#281）の責務 |
  両 Consumer の Ruviform 取り込み時 | 本 repo 内では再現不能。
- `passthrough_submits` の post-warmup 異常増加の ReportingHint 細分化 |
  warmup と異常を Descriptor だけでは時間軸で区別できない | Diagnostics
  （OBS-09）が warmup 条件を解釈する設計確定後 | 現段階で OnIncrease を
  付けると warmup 偽 WARN の温床。

## Sources and Relations

- Issue #279（本 spec の正本・完了条件）、親設計 Issue #275（単一クレート
  構成・責務分離の合意版）、絶対維持 Counter の Issue #121 診断契約。
- `docs/designs/issue-277-observability-core-source-contract.md`（Source/
  Registrar/Lease 契約の設計正本）、`docs/specs/issue-277-observability-core-source-contract.md`。
- `docs/adr/realtime-contract-and-verification-policy.md`（API 単位の RT 保証
  ラベル）。
