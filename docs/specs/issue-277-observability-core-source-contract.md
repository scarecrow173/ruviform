---
id: "SPEC-034Rp1Lwq5BGjy5ucoWJ4s"
type: "spec"
status: "approved"
title: "Issue #277 OBS-02 軽量Primitive・Observation Source型・登録/寿命契約"
created: "2026-11-03"
updated: "2026-11-03"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/277"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-277-observability-core-source-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/104"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/289"
  deferred-by: []
---

# Issue #277 OBS-02 軽量Primitive・Observation Source型・登録/寿命契約

## What Should Be Built

既存 `ruviform-observability` クレート内に、観測の最小語彙である **core 軽量 Primitive**
（既存 `Counter`/`CounterReader` に加え `Gauge`/`Histogram`）と、観測 Source の
**登録・寿命契約**（Descriptor / Scope / SourceId+Generation / `ObservationRegistrar`
ポート / `RegistrationLease` / `SourceLifecycle` / 型付き Error）を定義・実装する。
実際の Registry・Poll・Metrics 集約・History は OBS-07（Issue #289）が所有し、
本 Issue はその**実装可能なコンパイル済み契約**だけを提供する。

## Why Now

- 親設計 Issue #275（Observability 統合設計正本・approved）が、単一クレート内に
  `core`/`source` 層を置く構造を合意済み。OBS-07 Collector（#289）は OBS-02 の
  Source/Registrar/Lease 契約を前提とするため、先に型を固定する必要がある。
- 現状は `counter()` ペアのみで、Source の一意識別・登録・寿命・失効時 Error の
  語彙が存在しない。後続 Issue（#289・#278・#279）が各自で型を発明すると
  責務境界（Domain 宣言 vs Consumer 変換）が曖昧になる。

## Users and Value

- **OBS-07（#289）実装者**: `ObservationRegistrar`/`RegistrationLease`/寿命状態機械を
  そのまま実装・保持できるコンパイル済みポートを得る。
- **Domain 側（plugin host 等の instrumented コード）**: 事象の意味・温度感・正常
  warmup・閾値・情報源・Confidentiality を登録時に型で宣言できる。
- **Consumer（AudioLens/Synvelis）**: Counter→Log mapping や Severity 推論を自前で
  持たずに済む境界が確定する。

## Scope

### In Scope

- core Primitive: 既存 `counter()`/`Counter`/`CounterReader`（回帰禁止）、
  `gauge()`/`Gauge`/`GaugeReader`（単一 `Arc<AtomicU64>`）、
  `histogram()`/`Histogram`/`HistogramReader`（固定 Bucket 境界・Bucket 別 `AtomicU64`）。
- source 契約型: `CounterDescriptor`・`MetricDescriptor`/`MetricKind`・
  `ObservationUnit`・`ReportingHint`/`ReportingImpact`・`AggregationPolicy`・
  `ObservationScope`・`SourceSemantics`（意味/温度/warmup/閾値/情報源/Confidentiality）・
  `SourceId`/`SourceGeneration`/`SourceIdentity`・`CounterSource`/`MetricSource`/
  `MetricReader`・`SourceLifecycle`・`SourceState`・`RegistrationLease`/
  `RetiredSource`・`ObservationRegistrar`・`RegistrationError`/`LifecycleError`。
- operation ごとの RT 適性記述と、`rt_` レーン（`assert_no_alloc`）での no-alloc 実証。
- 寿命状態機械（Active→Retiring→FinalDrain→Removed）と失効時 Error の決定的試験。
- Domain/Consumer の責務分離を示す crate README 例。

### Out of Scope

- 実 Registry・`Collector::poll`・Metrics 集約・History・Snapshot（OBS-07 #289）。
- `tracing`/`tracing-subscriber` 変換（OBS-03 #278）、固定長 RT Record Bridge
  （OBS-05 #280）、Logging（OBS-01 #276）、Diagnostics/Debug/Integration。
- 新規クレート作成、外部 IO/GUI/Subscriber 依存の追加。
- 浮動小数点・複数値 Gauge の transactional consistency、Histogram 分位点の厳密値。
- Counter 値減少/wrap 疑い時の再 Baseline（OBS-07 の Baseline 責務）。

## Requirements

- 既存 `counter()` 公開 API・意味（単一 Counter 非減少観測・複数 Counter 非同時
  Snapshot 非保証）・RT 計装コスト・Clone/Drop 境界を一切回帰させない。
- Gauge は単一 Atomic の安全性のみを型・ドキュメントで限定する。浮動小数点や
  複数値の整合性は約束しない。
- Histogram の Bucket RT 書込みは実証前に RT-safe と宣言しない
  （no-alloc は `rt_` レーンで実証するが、正式な RT-safe 宣言は実機証跡待ち）。
- 登録/解除/変更は非 RT のみ。RT Writer は Registry Lock を持たず、必要な
  Reader/metadata は RT 稼働前に準備する。最後の Drop を RT で実行しない。
- `RegistrationLease` の単純 Drop が未出力 Delta を黙って消さない
  （Reader は Registry 側所有。Lease Drop は `abandoned` として観測可能にし、
  明示 `retire` → `RetiredSource` 経路を用意する）。
- `SourceId` は観測登録を一意識別し、Domain Instance ID・Graph Revision・
  Source Generation/再 mint と混同しない。`&'static str` key・enum Scope で
  Source 数と Scope/Label の Cardinality を静的に制限する。
- 全公開 enum は `#[non_exhaustive]`。依存は std のみを維持する
  （`assert_no_alloc` は dev-dependency）。

## Acceptance Criteria

- [ ] AC-1: 既存 Counter 公開 API/試験と RT 性能/Clone/Drop 境界が保持される
  （`mise run test`・`mise run test-rt`・既存 consumer ビルドが回帰なし）。
- [ ] AC-2: Core/Source 型が `tracing`・`tracing-subscriber`・外部 IO/GUI 依存なしで
  ビルドされる（crate の `[dependencies]` が空のまま）。
- [ ] AC-3: Descriptor/Scope/SourceId/Generation/RegistrationLease/Reader 所有権が
  型化され、`ObservationRegistrar` が object-safe かつ `Send + Sync` であり、
  Lease/状態共有に循環参照がないことをコンパイル・試験で検証する。
- [ ] AC-4: Gauge/Histogram 軽量 Primitive の RT 適性が operation ごとに
  Rustdoc で記述され、`rt_` レーン試験が存在する。
- [ ] AC-5: Active/Retiring/FinalDrain/Removed の Source 寿命契約と
  Source 失効時 Error（不正遷移・終端到達後の遷移）が決定的に試験される。
- [ ] AC-6: OBS-07 が実装可能なコンパイル済み登録ポートと no-cycle 依存 DAG
  （observability は他 ruviform クレートへ依存しない）が維持される。
- [ ] AC-7: Domain と Consumer が変換/Severity/Grant/Audit 責務を混同しない
  README 例が存在する。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- 実 Registry・Poll・Baseline 確立・不連続/再基準化・History・Snapshot | OBS-07 の責務 | Issue #289 着手時 | 契約だけ先に確定しないと OBS-07 が型を仮決めするリスク。
- Histogram `observe` の正式 RT-safe 宣言 | assert_no_alloc では alloc のみ検出で
  blocking/実 latency は未検証 | OBS-08 計測・実機証跡 | 宣言を急ぐと過剰保証。
- f64・複数値 Gauge | transactional consistency を約束しない方針 | 実需要の出現 | 安易な追加は未定義の整合性期待を生む。

## Sources and Relations

- Issue #277（本 spec の正本・完了条件）、親設計 Issue #275（単一クレート構成・
  Feature 方針・責務分離の合意版）、実装先 Issue #289（OBS-07 Collector）。
- `docs/designs/foundation-observability-minimal-api.md`（Counter 既存契約の原点）、
  `docs/adr/realtime-contract-and-verification-policy.md`（API 単位の RT 保証ラベル）。
