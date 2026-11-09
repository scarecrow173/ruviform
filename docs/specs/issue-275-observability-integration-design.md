---
id: "SPEC-8EJ4gM4rl1pb5GPij5Nm0l"
type: "spec"
status: "approved"
title: "Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/103"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/plans/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/tasks/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-275-observability-integration-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-276-generic-logging-adapter.md"
    - "docs/designs/issue-276-generic-logging-adapter.md"
    - "docs/specs/issue-277-observability-core-source-contract.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
    - "docs/specs/issue-278-observability-tracing-export.md"
    - "docs/designs/issue-278-observability-tracing-export.md"
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
    - "docs/designs/issue-279-plugin-host-counter-source-registration.md"
    - "docs/specs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/designs/issue-280-rt-record-bridge-clock-correlation.md"
    - "docs/specs/issue-289-observability-collector-metrics.md"
    - "docs/designs/issue-289-observability-collector-metrics.md"
    - "docs/specs/issue-290-observability-allocation-probe.md"
    - "docs/designs/issue-290-observability-allocation-probe.md"
    - "docs/specs/issue-290-observability-rt-timing.md"
    - "docs/designs/issue-290-observability-rt-timing.md"
    - "docs/specs/issue-291-observability-diagnostics.md"
    - "docs/designs/issue-291-observability-diagnostics.md"
    - "docs/specs/issue-292-observability-debug-inspection.md"
    - "docs/designs/issue-292-observability-debug-inspection.md"
    - "docs/specs/issue-293-observability-integration-runner.md"
    - "docs/designs/issue-293-observability-integration-runner.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/adr/obs-05-observability-leaf-primitive-deps.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/103"
  derived-by: []
  verifies: []
  verified-by: []
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

## What Should Be Built

Issue #275「Observability統合設計正本 — 単一クレート・
OBS-01〜11・両Consumer」（2026-11-04 合意版）の **正本設計
文書** を `docs/` チェーン（spec → design → test-spec →
plan → task）として作成する。具体的には
`docs/designs/issue-275-observability-integration-design.md`
を新設し、Issue 本文の合意事項を次の構成要素として
文書化する。

1. **単一クレート構成と論理依存 DAG**: 既存
   `ruviform-observability` クレート内への統合（
   `ruviform-observability-collector`/`-tracing`/`-logging`
   等の独立クレート新設禁止）、`core`/`source` →
   `collector` → `diagnostics`/`debug`/`tracing_export`、
   `logging` の collector 非依存、`integration` による
   組立てという論理依存と、実装時にコンパイル可能な DAG
   で確定する配置方針。
2. **横断合意事項 1〜10**: Core/RT 互換、Collector/
   Metrics、Tracing Export、Logging Adapter、Profiling、
   Diagnostics、Debug、統合運用、安全・所有権、両
   Consumer の各契約を、per-issue 正本への参照を伴う
   横断索引として記述する。
3. **Cargo Feature 方針と実装済み matrix の突合**:
   合意された feature 名（`collector`/`diagnostics`/
   `debug-tools` 等を含む構想）と、実装時のコンパイル
   検証を経て確定した実際の feature 構成
   （`logging`/`tracing-export`/`allocation-probe`/
   `integration` + 無条件モジュール）の差異を記録し、
   どちらを正とするかを明確化する。
4. **実装 Issue 索引と責務分界**: OBS-01（#276）〜
   OBS-11（#293）と Consumer Issue（AudioLens #138・
   Synvelis #81）、Gate Issue（#281 OBS-06・#219・
   #294）の Owner/Scope/依存関係の索引。
5. **両 Consumer 利用面**: AudioLens の Logging parity
   要件（既定 OFF・`AL_LOG`・stderr/JSONL・`inspect`
   stdout 保護・daily 7 世代 retention・機密・f64/samply
   維持）と Synvelis の Rust Host 統合面（Multiwindow・
   Playback・Plugin・Gateway・Subscriber 所有・RT 安全・
   認可/Audit 分離）。
6. **受入 Gate との証拠項目対応**: Issue #275 の受入
   Gate 9 項目を、既存の per-issue test-spec・実装
   記録・#281（OBS-06）の G0〜G4 gate・#219/#294 の
   cross-repo 検証機構へ対応付ける索引。
7. **#101/#103 前提の更新**: 旧「Logging 保留」判定と
   「クレート抽出前提」の設計書を合意版の内容へ更新し、
   影響変更を記録する。

## Why Now

- per-module の設計正本（#276〜#293）と実装は main に
  揃いつつある: `crates/ruviform-observability` に
  `core`/`source`/`collector`/`metrics`/`diagnostics`/
  `debug`/`record`/`rt_bridge`/`rt_timing`/`tracing_export`/
  `logging`/`profiling`/`integration` の各モジュールが
  存在し、OBS-01/02/04/05 は Issue CLOSED、OBS-08 の
  RT Timing slice も merge 済み（PR #519）。しかし
  OBS-01〜11 を貫く統合設計の正本は未作成で、横断契約
  （単一クレート・依存 DAG・RT 境界・Consumer 利用面・
  証拠項目対応）が 10 件の per-issue 文書に分散している。
- OBS-06（#281）は「OBS-01〜11 の全モジュールと両
  Consumer の互換性・性能・精度検証の追跡 Owner」で
  あり、その検証対象の設計正本として #275 が求められて
  いる。#219（cross-repo CI gate）・#294（candidate
  gate）は本書の証拠項目対応を消費する。
- 受入 Gate 最終項目は #101/#103 の旧前提（Logging 保留・
  クレート抽出候補）の設計書更新を要求しており、本
  slice がその更新を担う。

## Users and Value

- **Ruviform maintainer / Observability Owner**: OBS-01〜
  11 の責務境界・依存順・RT/non-RT 分離・feature 構成の
  正本を得て、後続の変更・Consumer 統合・gate 判定の
  基準を固定できる。
- **AudioLens / Synvelis Owner**: 両 Consumer が利用する
  logging/collector/diagnostics/debug/tracing 面と、
  schema/version・認可/Audit 等の Consumer 所有責務の
  分界を確認できる。
- **#281（OBS-06 gate）Owner**: G0〜G4 の各検証が参照する
  設計正本と証拠項目の対応を辿れる。
- **後続の実装・保守担当**: モジュール詳細は per-issue
  設計・test-spec を正本として参照し、本書では横断契約
  と索引のみを確認できる。

## Scope

### In Scope

- `docs/` 配下の文書チェーン（spec/design/test-spec/
  plan/task）と生成 index、および #101/#103 前提を持つ
  既存設計書 2 件への合意版反映注記のみ。**コード変更
  なし**。
- Issue #275 本文（2026-11-04 合意版）の要求事項の
  文書化。
- OBS-01〜11 の per-issue 設計・test-spec・実装記録、
  #281 gate 設計、#219/#294 cross-repo 検証機構、
  AudioLens #138・Synvelis #81 の main 上の設計・実装
  成果を消費した、単一クレート構成・横断契約・Consumer
  利用面・証拠項目対応の記述。

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `.github/` の変更（本 slice は docs 設計専用）。
- per-module の詳細 API・内部実装・test 規則の再定義
  （#276〜#293 の設計・test-spec が正本。本書は横断契約
  と索引のみを所有する）。
- AudioLens/Synvelis repo の変更・実測（cross-repo
  gate、#219/#294 consumer-verify・各 Consumer Issue が
  Owner）。
- 新規 CI Workflow・証拠 YAML/JSON スキーマ・Dashboard
  の作成。
- 別の汎用 Logger/Span フレームワーク、専用
  Observability 製品/サーバー、必須 OTLP/Tracy/
  Prometheus、無制限 PCM Capture、完全 Replay、通常
  Logging による Authorization/Audit の置換（Issue
  非対象）。

## Requirements

- 単一クレート構成: `ruviform-observability` への統合と
  独立サブクレート新設禁止、論理依存方向
  （core/source → collector → diagnostics/debug/
  tracing_export、logging 独立、integration 組立）を
  明文化する。
- 横断合意事項: Issue 本文の 10 項目（Core/RT・
  Collector/Metrics・Tracing・Logging・Profiling・
  Diagnostics・Debug・統合運用・安全/所有権・両
  Consumer）を per-issue 正本への参照付きで記述する。
- Feature/DAG: 合意 feature 方針と実装済み feature
  matrix の差異を記録し、`--no-default-features`/各
  feature/full consumer でのコンパイル検証が gate 証拠
  項目であることを明記する。
- Consumer 利用面: AudioLens の Logging parity・
  Metrics/Profiling 維持要件と Synvelis の Rust Host
  統合面・認可/Audit 分離を記述する。
- 証拠項目対応: 受入 Gate 9 項目を既存 test-spec・
  実装記録・#281 G0〜G4・#219/#294 機構へ対応付ける
  索引を設計文書に含める。
- 「設計の合意 ≠ テスト完了・PoC 完了・実機 RT 保証・
  Cross-Consumer 移行完了」を明記し、実装済み部分と
  未検証・Consumer 側未実施部分を分離して記録する。
- #101/#103 前提の設計書更新: `foundation-
  observability-minimal-api.md` と `shared-foundations-
  and-modularization-policy.md` に合意版への改訂注記を
  入れ、影響変更を記録する。

## Acceptance Criteria

- [ ] 単一 crate/Feature DAG・横断合意事項・Consumer
      利用面・証拠項目対応を含む正本設計文書が
      `docs/designs/` に存在する。
- [ ] `ruviform-observability` 単一クレート構成と独立
      サブクレート新設禁止、論理依存 DAG、標準
      `tracing` 利用（独自 Logger/Span API なし）が
      明文化されている。
- [ ] 合意 feature 方針と実装済み feature matrix の
      差異が記録され、`--no-default-features`/各
      feature/full consumer のコンパイル検証が未実施
      の gate 証拠として `NotRun` 相当で記録されて
      いる。
- [ ] RT call path の no-alloc/no-lock/no-IO/no-
      Subscriber 契約と「feature 分離だけで RT 安全を
      保証しない」原則が per-issue 正本への参照付きで
      記述されている。
- [ ] AudioLens（既定 OFF・`AL_LOG`・stderr/JSONL・
      `inspect` stdout・daily 7 世代・機密・f64/samply）
      と Synvelis（Rust Host・Multiwindow・認可/Audit
      分離）の利用面と責務分界が明記されている。
- [ ] Issue #275 の受入 Gate 9 項目が既存 per-issue
      test-spec・実装記録・#281 G0〜G4・#219/#294 機構
      へ対応付けられている。
- [ ] #101/#103 前提の設計書 2 件に合意版への改訂注記が
      あり、影響変更が記録されている。
- [ ] 「設計合意と実装/検証・PoC・実機 RT・Consumer
      受入の分離」が明記され、未実施項目は `not run`/
      `NotRun`/`Unsupported` と記録する方針が示されて
      いる。
- [ ] 変更が `docs/` のみであり、`crates/`・
      `Cargo.toml`・`Cargo.lock`・`mise.toml`・
      `.github/` に差分がない。

## Deferred Design Concerns

- Consumer 側の実測・移行受入（AL Logging parity の
  Golden 比較、SV Host 統合の Multiwindow/RT/Plugin
  検証）| cross-repo gate（#219/#294 consumer-verify）
  と各 Consumer Issue が Owner であり本 slice の対象外
  | 設計どおりの互換性が実測で裏付けられないまま採用
  されるリスク。
- 実機 RT 計測（OBS-05 の RT Record Bridge 実機 Gate・
  CPU/Platform/Buffer Size 負荷時の latency 計測）|
  PoC 値を hard RT 保証と主張しない原則により段階的
  Gate として #281 で管理 | 実機環境・負荷条件の確保が
  前提 | PoC 結果を RT 保証と誤認するリスク。
- macOS/Linux 実機・MSRV 検証の全組合せ | 本 slice の
  作業環境は Windows のみ・docs のみ | CI 既存 OS lane
  または実機環境での gate 実行時 | OS/ツールチェーン差異
  が未検証のまま残る。
- per-module の残実装・残受入（#278/#289〜#293 が OPEN）
  | 各 Issue の実装・受入段階 | 本書は設計正本であり
  実装完了を意味しない | 設計正本と実装状態の乖離。

## Sources and Relations

- 正本 Issue: [#275](https://github.com/scarecrow173/
  ruviform/issues/275)（Observability統合設計正本
  2026-11-04 合意版 — Status: Design approved /
  implementation NOT STARTED（登録時点の状態。per-module
  実装の main 上の現状は設計書の snapshot 節で別途記録））。
- 前提設計: [#101](https://github.com/scarecrow173/
  ruviform/issues/101)（共通基盤・Observability 設計方針
  — 「内部モジュール先行・必要な時のみクレート抽出」）、
  [#103](https://github.com/scarecrow173/ruviform/
  issues/103)（Foundation/Observability 候補の最小設計 —
  本書で前提更新の対象）。
- 実装 Issue 索引: OBS-01 #276・OBS-02 #277・OBS-03
  #278・OBS-04 #279・OBS-05 #280・OBS-07 #289・OBS-08
  #290・OBS-09 #291・OBS-10 #292・OBS-11 #293。Gate:
  OBS-06 #281、cross-repo #219、candidate gate #294。
  関連: RT Observability PoC #104、Probe #185、初消費者
  #121。
- Consumer Issue: AudioLens
  [#138](https://github.com/scarecrow173/audio-lens/
  issues/138)（CLOSED — 2026-11-09 snapshot）、Synvelis
  [#81](https://github.com/scarecrow173/synvelis/issues/81)
  （CLOSED — 同 snapshot）。
- 参照設計文書: `docs/designs/issue-276-…`〜
  `issue-293-…`（per-module 正本）、`docs/designs/
  foundation-observability-minimal-api.md`・`docs/designs/
  shared-foundations-and-modularization-policy.md`（#101/
  #103 前提 — 本 slice で改訂注記）、`docs/adr/
  obs-05-observability-leaf-primitive-deps.md`（leaf
  依存の ADR）。
