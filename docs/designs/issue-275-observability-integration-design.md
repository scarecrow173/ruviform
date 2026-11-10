---
id: "DESIGN-hXhz1NTWfHfPzPb5P4A5ff"
type: "design"
status: "approved"
title: "Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計"
created: "2026-11-09"
updated: "2026-11-10"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/103"
    - "docs/specs/issue-275-observability-integration-design.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/specs/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/plans/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/tasks/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-275-observability-integration-design.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/designs/foundation-observability-minimal-api.md"
      - type: "doc"
        path: "docs/designs/shared-foundations-and-modularization-policy.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-275-observability-integration-design.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-275-observability-integration-design.md"
    - "docs/designs/issue-276-generic-logging-adapter.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
    - "docs/designs/issue-279-plugin-host-counter-source-registration.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-290-observability-allocation-probe.md"
    - "docs/designs/issue-290-observability-rt-timing.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-292-observability-debug-inspection.md"
    - "docs/designs/issue-293-observability-integration-runner.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-278-observability-tracing-export.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-275-observability-integration-design.md"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-275-observability-integration-design.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/103"
    - "https://github.com/scarecrow173/ruviform/issues/104"
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/276"
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/278"
    - "https://github.com/scarecrow173/ruviform/issues/279"
    - "https://github.com/scarecrow173/ruviform/issues/280"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/290"
    - "https://github.com/scarecrow173/ruviform/issues/291"
    - "https://github.com/scarecrow173/ruviform/issues/292"
    - "https://github.com/scarecrow173/ruviform/issues/293"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  defers: []
  deferred-by: []
---

# Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計

## Context

Issue #275 は OBS-01〜11 の合意事項・責務境界・依存順と
各 Consumer 導入の **正本** である（Issue 本文「
Observability統合設計正本（2026-11-04 合意版）」）。
本文の Status 表記は「Design approved / implementation
NOT STARTED」— これは Issue 登録・設計承認時点の状態
宣言であり、**設計承認は PR 作成・PoC 検証・実機 RT 保証・
Cross-Consumer 移行完了を意味しない**。per-module の実装は
その後 main 上で個別 Issue ごとに進行しており、本書は
その現状を snapshot として記録する（後述「実装の現状」）。

本書が固定するもの / 固定しないもの:

- **固定する**: 単一クレート `ruviform-observability` への
  統合と独立サブクレート新設禁止、論理依存 DAG、横断合意
  事項 1〜10 の索引、feature 方針と実装済み matrix の
  突合、Consumer 利用面、受入 Gate との証拠項目対応、
  #101/#103 前提の更新記録。
- **固定しない**: per-module の詳細 API・内部実装・test
  規則（per-issue 設計・test-spec が正本）、module の
  ファイル配置の最終形（実装時のコンパイル可能な DAG で
  確定）、Consumer 側の schema/version/migration・
  認可/Audit の意味論（各 Consumer が所有）。

## 実装の現状（2026-11-10 snapshot）

`crates/ruviform-observability` は main 上に存在し、
`src/` のモジュール配置は Issue の構想木と概ね一致する。

| モジュール | feature gate | 主担当 Issue | 正本 |
| --- | --- | --- | --- |
| `core`（Counter/CounterReader/Gauge/Histogram） | なし | #277（OBS-02） | `issue-277-…` |
| `source`（Descriptor/SourceId+Generation/Registrar/Lease） | なし | #277（OBS-02） | `issue-277-…` |
| `collector` + `metrics`（Registry/Polling/Metrics/History/Snapshot） | なし | #289（OBS-07） | `issue-289-…` |
| `diagnostics`（Evidence/Finding/Health/Report） | なし | #291（OBS-09） | `issue-291-…` |
| `debug`（Inspection/Invariant/Session） | なし | #292（OBS-10） | `issue-292-…` |
| `record` + `rt_bridge`（RT Record/Clock/欠落/相関） | なし | #280（OBS-05） | `issue-280-…` + ADR obs-05 |
| `rt_timing`（RT Timing/Deadline/Histogram 連携） | なし | #290（OBS-08） | `issue-290-observability-rt-timing.md` |
| `profiling`（CountingAllocator/AllocationScope/Window） | `allocation-probe` | #290（OBS-08） | `issue-290-observability-allocation-probe.md` |
| `tracing_export`（Source/Record→tracing・Correlation） | `tracing-export` | #278（OBS-03） | `issue-278-…` |
| `logging`（Dispatch/Subscriber builder/filter/sink/writer/health） | `logging` | #276（OBS-01） | `issue-276-…` |
| `integration`（Host non-RT Runner/Config/Lifecycle/Health） | `integration` | #293（OBS-11） | `issue-293-…` |

Issue 状態 snapshot（2026-11-10・`gh issue view`）:

| Issue | 状態 | 備考 |
| --- | --- | --- |
| #275（本 Issue） | OPEN | 本書が正本文書。Gate 実行証拠は別途 |
| #276 OBS-01 | CLOSED | logging adapter 実装済み |
| #277 OBS-02 | CLOSED | core/source 契約実装済み |
| #278 OBS-03 | CLOSED | `tracing_export` 実装済み（受入照合 slice #539 含む） |
| #279 OBS-04 | CLOSED | plugin-host source 登録実装済み |
| #280 OBS-05 | CLOSED | record/rt_bridge 実装済み（実機 Gate は別段階） |
| #281 OBS-06 | OPEN | 両 Consumer 互換/RT/精度検証 Gate（継続） |
| #289 OBS-07 | CLOSED | collector/metrics 実装済み |
| #290 OBS-08 | CLOSED | allocation-probe + rt_timing 実装済み |
| #291 OBS-09 | CLOSED | diagnostics 実装済み（受入照合 #540 含む） |
| #292 OBS-10 | CLOSED | debug 実装済み（受入照合 #542 含む） |
| #293 OBS-11 | CLOSED | integration runner 実装済み |
| #294 | OPEN | consumer-verify 横断 Gate 拡張 |
| AL #138 | CLOSED | AudioLens 移行 Issue（Consumer repo 側受入） |
| SV #81 | CLOSED | Synvelis 統合 Issue（Consumer repo 側受入） |

**読み方の注意**: Issue OPEN は「コード非存在」を意味
しない。Issue 状態・コード存在・受入証拠は別レイヤーの
事実であり、本書はそれらを混同しない。#281（OBS-06）
と #294（consumer-verify）が OPEN のまま残るのは、
両 Consumer の cross-repo 互換・RT・精度の実機証拠が
未収集だからであり、in-repo の実装・契約試験の完了を
意味しない。

**2026-11-10 追記（実装照合・契約試験棚卸し slice）**:
OBS-01〜11 の全実装 Issue が CLOSED となった時点で
`crates/ruviform-observability` の module/feature 構成と
本書の合意アーキテクチャを再照合した。headless 検証可能
な契約試験ギャップとして、RT-suitable 操作の `tracing`/
Subscriber 非発行を subscriber dispatch 設置下で固定する
`tests/rt_boundary.rs`（3 test）と、lease abandon ×
`Collector::shutdown` 競合での Partial drain を固定する
collector test 1 件を追加した（記録:
`docs/impl/ir/issue-275-obs-design-reconciliation.md`）。
両 Consumer 実機受入・compile matrix 全組合せ・実機 RT
保証は #281/#219/#294 の gate 段階であり `NotRun` のまま。

## 責務分担 — 本書と per-issue 正本

本書は **横断契約と索引のみ** を所有し、モジュール詳細は
per-issue 正本に委譲する（参照構造で drift を防ぐ）。

| 領域 | 正本（詳細） | 本書の役割 |
| --- | --- | --- |
| OBS-01 Logging | `issue-276-generic-logging-adapter.md`（+ ADR: bounded writer/sink resilience） | 横断契約・利用面・証拠対応の索引 |
| OBS-02 Core/Source | `issue-277-observability-core-source-contract.md` | 同上 |
| OBS-03 Tracing Export | `issue-278-observability-tracing-export.md` | 同上 |
| OBS-04 Plugin Host 登録 | `issue-279-plugin-host-counter-source-registration.md` | 同上 |
| OBS-05 RT Record Bridge | `issue-280-rt-record-bridge-clock-correlation.md` + `docs/adr/obs-05-observability-leaf-primitive-deps.md` | 同上 |
| OBS-07 Collector/Metrics | `issue-289-observability-collector-metrics.md` | 同上 |
| OBS-08 Profiling/RT Timing | `issue-290-observability-allocation-probe.md`・`issue-290-observability-rt-timing.md` | 同上 |
| OBS-09 Diagnostics | `issue-291-observability-diagnostics.md` | 同上 |
| OBS-10 Debug | `issue-292-observability-debug-inspection.md` | 同上 |
| OBS-11 Integration | `issue-293-observability-integration-runner.md` | 同上 |
| OBS-06 Cross-Consumer Gate | Issue #281 本文（G0〜G4・PASS 証跡書式） | 証拠項目対応の索引 |
| Cross-Repo 検証 | `issue-219-cross-repo-ci-gate-credential-runbook.md`（設計）・`docs/specs/issue-219-consumer-compat-contract.md`（proposed）・`issue-294-cross-consumer-candidate-gate.md` | 同上 |
| 前提設計 | `foundation-observability-minimal-api.md`（#103）・`shared-foundations-and-modularization-policy.md`（#101） | 前提更新の記録 |

## 単一クレート構成と論理依存 DAG

Issue 合意（2026-11-04）: **既存 `ruviform-observability`
に統合し、`ruviform-observability-collector`/`-tracing`/
`-logging` など独立クレートを新設しない**。#101 の
「内部モジュール先行・必要な時のみクレート抽出」に従う。

合意された論理依存:

```text
core/source ──▶ collector ──▶ diagnostics
                    │    └──▶ debug
                    └──▶ tracing_export
logging: collector 非依存。標準 tracing-subscriber 系へ接続
integration: 必要な機能を選択的に組み立てる（non-RT Host 支援）
```

- `collector` は `tracing`・Logging に依存しない
  （#289 正本）。
- `logging` は collector 非依存で標準
  `tracing-subscriber`/`tracing-appender` 系に接続する
  （#276 正本）。独自 Logger/Span フレームワークを作ら
  ず、標準 `tracing` 発行は非 RT の Ruviform/Consumer が
  直接行う。
- `integration` は Logging/Tracing Export を任意の組立
  要素とし、有効化だけで両方を強制しない（#293 正本）。
- ファイル/内部モジュールの最終配置は実装時のコンパイル
  可能な DAG で確定する（Issue 明記）。現状の `src/`
  配置は上表のとおりで、`record`/`rt_bridge`/`rt_timing`/
  `metrics` は論理領域をまたぐ補助モジュールとして実装
  側で配置済み。
- workspace 内 `path` 依存は ADR `obs-05-observability-
  leaf-primitive-deps` の一方向 leaf 依存のみ:
  `ruviform-observability → ruviform-rt`（bounded SPSC
  ring）と `→ ruviform-media-core`（clock-domain 語彙）。
  逆方向（`rt`/`media-core` → `observability`）は禁止。
  外部 crate 依存は `logging`/`tracing-export` feature の
  optional `tracing`/`tracing-subscriber`/`tracing-appender`
  のみで、`default=[]` では外部依存を引かない。

## 横断合意事項（Issue 本文 1〜10 の索引）

各項目の詳細契約は右列の per-issue 正本が所有する。本書は
合意の存在と境界を固定する。

1. **Core と RT**: 既存 `counter() -> (Counter,
   CounterReader)` の `Arc<AtomicU64>`/Relaxed/単一
   Counter 単調増加・複数 Atomic 非同時 Snapshot・非 RT
   構築/Clone/Drop を維持。RT writer が実行する操作のみ
   事前準備・有界・no heap allocation/lock/IO/
   Subscriber を検証。**Cargo Feature による機能分離
   だけで RT 安全性を保証したと主張しない**。
   → 正本: `issue-277-…`（core primitive）、`issue-280-…`
   （RT record bridge）。
2. **Collector/Metrics**: Registry/Source 寿命・最終
   Drain・Counter/Gauge/Histogram/Rate/Window・上限付き
   Series/History・Snapshot/Validity/欠落をクレート内で
   所有。`Collector::poll(...)` は観測 Batch を返し、
   `poll_and_emit`/`start_background(dispatch)` の旧責務
   混在 API は置換する。→ 正本: `issue-289-…`。
3. **Tracing**: `tracing_export` が Ruviform 固有
   Counter/RT Record/将来 Telemetry を non-RT で
   `tracing` へ変換し、Domain 定義の Semantics/Severity
   を使用。Consumer に独自 Delta/Exporter を実装させない。
   標準 Span/Instrument・async task 伝播・W3C Trace
   Context optional non-RT Adapter・RT 固定長 Correlation
   Token・Source Generation/Clock Domain の不一致・欠落を
   扱う。**Trace ID は認証/Grant を意味しない。Counter
   の Poll 時刻を実事象時刻として扱わず、RT から Span を
   開かない**。→ 正本: `issue-278-…`、`issue-280-…`。
4. **Logging**: `observability::logging` は標準
   `tracing-subscriber`/`tracing-appender` 系を使う初期
   必須の汎用 Logging Adapter。Sink 毎の stderr/rolling
   file・text/JSONL・filter・rotation/retention・有界
   Queue・WriterGuard/Health・BestEffort/Required を提供。
   **Global Subscriber 設置、Sink 有効化とファイル場所は
   Host のみが決定。stdout 自動出力なし**。AudioLens 既存
   挙動を回帰基準に Synvelis にも初期導入。→ 正本:
   `issue-276-…`。
5. **Profiling**: CPU/Heap 詳細分析は samply 等の既存外部
   Profiler を利用。RT Timing/Deadline Utilization/
   Histogram と開発・テスト用 CountingAllocator（System
   等の `GlobalAlloc` Wrapper）を共通提供。**Global
   Allocator の宣言は Host/Test Binary。TLS 計測 Scope は
   他スレッドを観測できない**。Tracy や外部 Profiler は
   任意機能で RT 必須依存にしない。→ 正本:
   `issue-290-observability-rt-timing.md`・
   `issue-290-observability-allocation-probe.md`。
6. **Diagnostics**: Evidence（Observed/Derived/Reported/
   Inferred）・Finding・Health（Healthy/Degraded/
   Unhealthy/Unknown）・Hysteresis・Snapshot/Report を共通
   提供。事実と原因推定、未観測とゼロ、stale と healthy
   を区別。既存 Domain Error/FailureCause・監査・
   Provenance を置換しない。→ 正本: `issue-291-…`。
7. **Debug**: Inspection/Invariant（Satisfied/Violated/
   Inconclusive）/有界 Debug Session（Prepared→Active→
   Draining）を non-RT で提供。追加の不変条件検査を OFF
   にしても本番安全性検証を消さない。RT State は
   Snapshot/RT Probe 経由。既存 Probe（#185）/RT Bridge
   （#280）を再利用し、二重 Probe/完全 Replay を新規実装
   しない。→ 正本: `issue-292-…`。
8. **統合運用**: Host 支援 `integration` が Collector/
   Tracing/Logging の選択的組立・Collector→Exporter→
   Writer の停止順・Validate→Prepare→Activate→Retire の
   動的設定・自己 Health・GUI 向け non-RT Snapshot/
   Subscription を提供。**Logging OFF/Tracing OFF でも
   Collector/Diagnostics は成立**。Ruviform から React/
   Solid/WebView を強制しない。→ 正本: `issue-293-…`。
9. **安全・所有権**: 内部観測・Debug・Raw PCM・Source
   Path・Prompt/Credential・Plugin stderr の秘匿区分と
   Report Export は Host の認可対象。Synvelis Gateway
   Grant/Approval/Audit/Media Provenance の正本は通常
   best-effort logs と独立。RT 録音の lossless 正本と
   Probe の lossy 観測を混同しない。→ 正本:
   `issue-291-…`（privacy/secret 区分）、`issue-292-…`
   （session 権限）、`issue-293-…`（host 境界）、
   `issue-276-…`（sink 分類）。
10. **両 Consumer**: Ruviform 更新は AudioLens/Synvelis
    へ継続取り込み。変更ごとに Cross-Repo Gate（#219）と
    OBS-06（#281）で機能・精度・RT latency・OS 間回帰の
    独立証跡を要求。SHA 固定による迂回禁止。→ 正本:
    Issue #281・#219・#294。

## Cargo Feature 方針 — 合意名称と実装済み matrix の突合

Issue 合意の feature 構想（「最終名称・依存は実装時に
コンパイル検証」）:

| 合意 feature | 合意上の依存 | 実装済みの対応 |
| --- | --- | --- |
| （なし — core counter 等） | `default=[]` 基本 | `core`/`source`/`collector`/`metrics`/`diagnostics`/`debug`/`record`/`rt_bridge`/`rt_timing` は **feature gate なしで常時コンパイル** |
| `collector` | Registry/Metrics/History/Snapshot | **feature としては未採用** — `collector`/`metrics` は無条件モジュール |
| `diagnostics` | `collector` 依存 | **feature としては未採用** — `diagnostics` は無条件モジュール |
| `allocation-probe` | 開発・テスト用 Allocator Wrapper | **採用** — `profiling` モジュールを gate（依存なし・std のみ） |
| `debug-tools` | Debug Inspection/Session | **feature としては未採用** — `debug` は無条件モジュール |
| `tracing-export` | Collector 依存・標準 tracing 出力 | **採用** — `dep:tracing` のみ有効化（`logging` とは独立） |
| `logging` | tracing-subscriber/appender 任意依存・collector 非依存 | **採用** — `tracing`/`tracing-subscriber`/`tracing-appender` を optional dep として引く |
| `integration` | non-RT Host/Runner・Logging/Tracing を強制しない | **採用** — 依存追加なし・`tracing-export`/`logging` を強制しない |

**突合の結論**: 合意は「機能分離の構想と依存方向」で
あり、feature 名の最終集合は「実装時にコンパイル検証」
すると明記されていた。実装では `collector`/`diagnostics`/
`debug-tools` の 3 feature は採用されず、対応モジュールは
無条件コンパイルとされた（軽量 core を肥大化させない
範囲で std のみ・外部依存なしのモジュールは常時提供、
外部依存を要する `logging`/`tracing-export`/
`allocation-probe`/`integration` のみ feature 化）。この
差異は per-module 設計（#289/#291/#292 の feature 検証
節）で個別に確定・記録済みであり、本書は「合意構想 →
実装確定値」の対応表として記録する。**正本としての
feature matrix は `crates/ruviform-observability/Cargo.toml`
と per-module 設計**であり、本表はその索引。

残る検証要件（Issue 合意の未消化部分）: `default=[]`・
各 feature・全 feature・Cargo unification・MSRV・OS の
compile matrix を `--no-default-features`/各 feature/
full consumer で検証し、循環依存や `ruviform-rt` への
無審査逆依存を禁止する。本 slice では **未実施
（`NotRun`）** — #281 G0 の gate 証拠項目。

## RT / non-RT 境界

- RT writer が実行してよい操作は「事前準備済み・有界・
  no heap allocation/lock/IO/Subscriber」のみ。検証は
  `rt_` レーンの `assert_no_alloc` 等の実測に基づく
  （#277・#280・#290 各正本）。
- `tracing_export`/`logging`/`integration`/`diagnostics`
  の report 生成・`collector::poll` の消費側は non-RT。
  RT callback から tracing span を開かない、poll 時刻を
  実事象時刻にしない。
- `allocation-probe` の `CountingAllocator` は開発・テスト
  用で、Global Allocator 宣言は Host/Test Binary の責務。
  TLS Scope は他スレッドを観測しない（#290 正本）。
- CPU sampling/Tracy/外部 Profiler 連携は任意の non-RT
  機能であり、RT 必須依存にしない。

## Source identity / generation / clock / loss の非偽装

- Source identity は counter-cell mint/re-mint 境界で
  変化し、leg swap では変化しない（#279 正本）。
- Source Generation・Clock Domain・epoch の不一致・欠落は
  観測結果として報告し、隠さない・偽装しない（#277・
  #280・#289 正本）。
- RT Record Bridge の loss は bridge counter と
  `dropped_before` carrier 帰属で可視化し、clock anomaly
  （`end_tick < start_tick`）は負 duration にせず anomaly
  として計数する（#280・#290-rt-timing 正本）。
- Collector の staleness/validity/loss は Snapshot に
  明示し、「未観測」と「ゼロ」を区別する（#289・#291
  正本）。
- Trace ID・Correlation Token は認可/Grant capability
  ではない（#278・#280 正本）。

## Consumer 利用面

### AudioLens（AL-OBS-01 = audio-lens#138、CLOSED）

利用面: `observability::logging` の Host-owned Subscriber
構成要素（sink/filter/writer/health）、Collector
Snapshot、RT Timing、CountingAllocator（開発計測）。

維持すべき既存挙動（#281 G1・AL #138 の回帰基準）:

- Logging 既定 OFF、`AL_LOG`/flag 優先順位。
- stderr text / file JSONL の sink 形態。
- `inspect` コマンドの stdout 保護（観測出力で stdout を
  汚さない — sink/出力先は Host が決定）。
- `run`/`resume` の daily 7 世代 file rotation/retention。
- 失敗時 degrade（sink 障害が Job/RT を止めない —
  BestEffort/Required の分類は #276 正本）。
- JSON schema・error exit code の Golden 比較。
- 機密情報の非出力・f64/`samply` 数値精度の維持
  （StageRecord.duration_ms・Source 数値の同値性）。
- schema/version/migration・Dataset binary chunk 等の
  Consumer 所有意味論は Ruviform に移さない。

### Synvelis（SYN-OBS-01 = synvelis#81、CLOSED）

利用面: Rust Host への logging adapter 初期導入、
Collector/Diagnostics/Debug の `integration` Runner 経由
組立、Multiwindow/Headless/GUI 各形態。

維持すべき境界:

- **Subscriber 所有は Host のみ**。Multiwindow での
  double initialization 禁止（#281 G1/G4）。
- Playback/Plugin/Gateway 経路での RT 安全（RT callback
  からの alloc/lock/IO/Subscriber 呼出しなし）。
- **Gateway Grant/Approval/Audit/Media Provenance の正本は
  Observability と独立**。通常 best-effort logs で認可・
  Audit を置換しない（Issue 非対象）。
- Debug/Report/External Agent の権限と Path/PCM/Prompt/
  Secret 秘匿は Host の認可対象（#291・#292・#293 正本）。
- Project/Snapshot/Schema/Version/Migration の意味論は
  Synvelis 所有。Ruviform は観測プリミティブのみ提供。

### Consumer 所有 vs Ruviform 提供（責務分界）

Observability 契約には Consumer schema/version は存在
しない。Consumer が所有するのは: domain descriptor の
意味論（何を計測するか）、severity の domain 定義、
Logging policy（既定 OFF・環境変数・retention）、認可・
Audit・Provenance、数値精度の受入基準。Ruviform が提供
するのは: primitive・source 契約・collector・変換・
adapter・diagnostics/debug の機構と RT 安全性。

## 証拠項目対応 — 受入 Gate 9 項目の索引

Issue #275 受入条件（= 受入 Gate 全項目）各行の正本
対応。**gate 実行証拠の収集・判定は #281/#219/#294 の
gate 段階**であり、本書はその索引。2026-11-10 の
実装照合 slice は in-repo で検証可能な契約試験の
不足分のみを補完した（`docs/impl/ir/
issue-275-obs-design-reconciliation.md`）。

| # | Gate 項目 | 証拠の正本 |
| --- | --- | --- |
| G-1 | 単一 crate/Feature DAG コンパイル確認・core 使用時に外部 Subscriber/Logging 非強制 | `crates/ruviform-observability/Cargo.toml` feature matrix + #276/#293 の feature 組合せ検証 + #281 G0。compile matrix 実行は `NotRun` |
| G-2 | RT call path に alloc/blocking lock/OS IO/`tracing`/Subscriber/無制限 queue/制御されない Drop がない | `rt_` レーン `assert_no_alloc` 系 test（#277/#280/#290 test-spec）+ `tests/rt_boundary.rs`（dispatch 設置下での `tracing` 非発行、2026-11-10 追加）+ #281 G3 + 実機負荷 Gate（別段階） |
| G-3 | Counter/Gauge/Histogram・Collector Snapshot・Diagnostics・Debug・CountingAllocator・Tracing Export・Logging の契約試験 | per-issue test-spec（`issue-276-generic-logging-adapter.md`・`issue-277-observability-core-source-tests.md`・`issue-278-…-tests`・`issue-279-…-tests`・`issue-280-rt-record-bridge-tests.md`・`issue-289-…-tests`・`issue-290-observability-allocation-probe-tests.md`・`issue-290-observability-rt-timing-tests.md`・`issue-291-…-tests`・`issue-292-…-tests`・`issue-293-…-tests`）+ crate `tests/` contract tests（`integration`/`logging`/`rt_boundary`）+ `src/*/tests.rs` unit test + `mise run test-rt` 系 lane |
| G-4 | Source quiescence→final drain→unregister の競合安全・loss/stale/clock/epoch 非偽装 | `issue-277-…`/`issue-289-…`/`issue-280-…` の lifecycle・drain・validity 規則 + 各 test-spec + collector lifecycle test（abandon×shutdown 競合を含む、2026-11-10 追加） |
| G-5 | AudioLens 旧 Logging 既定 OFF/`AL_LOG`/stderr/json/inspect stdout/daily 7 世代/機密・f64/samply 維持 | AL #138（CLOSED）+ `issue-276-…` derived requirements + #281 G1 Golden 比較（consumer-verify 実走は `NotRun`） |
| G-6 | Synvelis Rust Host/Multiwindow/Playback/Plugin/Gateway で Subscriber/RT 安全/認可・Audit 分離 | SV #81（CLOSED）+ `issue-293-…` host 境界 + `issue-292-…` session 権限 + #281 G4 |
| G-7 | 外部 Provider/Exporter 故障で RT/Job 正本・Source のメトリクス/Query を損なわない | `issue-276-…` SinkFailureMode/Health・`issue-293-…` drain/error isolation・`issue-291-…` sink panic isolation + #281 G4 |
| G-8 | MSRV・Linux/macOS/Windows・Cross-Repo #219/#281 に Evidence（未実行は NOT RUN） | #219 consumer-verify runbook + #294 candidate gate + #281 PASS 証跡書式。`test-observability-msrv`（MSRV 1.89・all features）は 2026-11-10 slice で実行済み。Linux/macOS・cross-repo evidence は `NotRun` |
| G-9 | #101/#103 の古い Logging 保留・クレート抽出前提を合意版として設計書更新し影響変更を記録 | 本 slice: `foundation-observability-minimal-api.md`・`shared-foundations-and-modularization-policy.md` への改訂注記（後述） |

## #101/#103 前提の更新記録

Issue 合意 G-9 に対応し、本 slice で以下 2 文書へ合意版
への改訂注記を追加した（本文の歴史的分析は保持し、
「候補/保留」判定が後続の合意で解決された旨を明記）:

- `docs/designs/foundation-observability-
  minimal-api.md`（#103）: `observability::{logging,
  profiling, metrics, diagnostics, debug, rt_bridge,
  export}` の「保留」判定と `ruviform-observability`
  新設トリガは、#121 初消費者以降の OBS-01〜11 実装で
  解決済み。正本は本書。
- `docs/designs/shared-foundations-and-
  modularization-policy.md`（#101）: `ruviform-
  observability` の構成候補名・内部モジュール先行方針は
  #275 合意で確定（単一クレート・上記モジュール構成）。
  他候補（foundation/debug/platform 等）の保留判定は
  不変。

影響変更: 上記 2 文書の front matter（`updated`・
`derived-by`/`references` への本チェーン追加）と本文中の
改訂注記のみ。判定表・スケッチ本文は歴史的記録として
改変しない。

## 設計の合意と実装/検証の分離

- 本書の approved は **設計正本の合意** であり、PR 作成・
  PoC 検証・実機 RT 保証・Cross-Consumer 移行完了を
  意味しない（Issue Status 明記を継承）。
- per-module の実装状態は「実装の現状」snapshot のとおり
  個別に進行しており、本書はその索引であって完了宣言
  ではない。OPEN Issue の受入残件・#281/#219/#294 の
  gate 未実施分は `NotRun`/`not run`/`Unsupported` と
  して記録し、実施済みと偽らない。
- Consumer 受入（AL parity Golden・SV Host 統合の実測）
  は Consumer repo 側 Issue と cross-repo gate が Owner。

## Risks

- **正本の二重化**: 本書と per-issue 正本が同じ契約を
  異なる形で記述すると drift する。本書は索引・境界・
  対応表に限定し、詳細語彙（型名・variant・閾値）は
  per-issue 正本への参照で構成する。
- **snapshot 陳腐化**: Issue 状態・module 存在は
  2026-11-10 時点。将来の merge/close で表が古くなる
  可能性は snapshot 日付の明記で緩和する。
- **feature matrix の誤読**: 合意構想の feature 名と実
  装の差異を「未実装の不備」と誤読しないよう、突合表と
  「正本は Cargo.toml + per-module 設計」の明記を置いた。
- **gate 証拠の未収集**: compile matrix・実機 RT・
  consumer-verify は本 slice 対象外。#281 G0〜G4 の
  未実施を本書の approved で充足したとみなさない。

## Deferred Design Concerns

- Compile matrix（`--no-default-features`・各 feature・
  full consumer・MSRV・OS）の実行と記録 | #281 G0・CI
  lane 整備時 | feature 組合せ起因の build 破壊が検出
  されないまま残る。
- `collector`/`diagnostics`/`debug-tools` feature の
  再検討 | core 使用時のコンパイル時間・依存最小化の
  実需が出た時点 | 現状は無条件モジュールで問題ないが、
  Consumer が feature 分離を要求した場合に再設計が必要。
- `poll_and_emit`/`start_background` 旧 API の完全撤去
  確認 | #289 受入段階 | 旧責務混在 API の残存が
  Consumer 移行を複雑にする。

## Verification Notes

- 本 slice の検証は文書検査（link/relation 解決・参照先
  実在確認・内容突合・lint）のみ。test-spec
  `issue-275-observability-integration-design.md` が
  本書の正しさを検証する。
- `crates/` の実装・test・cargo 検証・Consumer 実測は
  対象外（`NotRun`）。
