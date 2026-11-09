---
id: "TSPEC-sFT08b6Qp306TbonlKfL28"
type: "test-spec"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-275-observability-integration-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-275-observability-integration-design.md"
    - "docs/designs/issue-275-observability-integration-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計

## Purpose

Issue #275 の設計正本（[design](../designs/issue-275-observability-integration-design.md)）
が「単一クレート `ruviform-observability` への統合・
論理依存 DAG・横断合意事項 1〜10・feature 方針と実装済み
matrix の突合・RT/non-RT 境界・Source/Generation/Clock/
Loss の非偽装・両 Consumer 利用面・受入 Gate 9 項目との
証拠項目対応・#101/#103 前提更新を、実在する per-issue
正本への参照で構成し、設計合意と実装/検証・PoC・実機
RT・Cross-Consumer 受入の完了を混同していない」ことを
検証する。対象 spec は
[spec](../specs/issue-275-observability-integration-design.md)、
実施記録は [task](../tasks/issue-275-observability-integration-design.md) /
[plan](../plans/issue-275-observability-integration-design.md) を参照。
コード変更を伴わない docs 成果物のため、検証は文書検査
（link/relation 解決・参照先の実在確認・内容の突合・
lint）で行う。本 test-spec は横断設計文書の正しさを
対象とし、per-module の実走テスト合否・Consumer 実測・
実機 RT 計測は対象外（実装/受入段階の要件）。

## Feature

Observability 統合設計正本（単一クレート・OBS-01〜11・
両 Consumer）横断設計文書

## Rules

- **R1（単一クレート構成の完全性）**: 設計文書は
  `ruviform-observability` への統合・`collector`/`tracing`/
  `logging` 独立クレート新設禁止・#101「内部モジュール
  先行」方針への言及・論理依存（core/source → collector
  → diagnostics/debug/tracing_export、logging は
  collector 非依存、integration が組立）・標準 `tracing`
  利用で独自 Logger/Span API を作らないこと・外部依存が
  ADR `obs-05` の一方向 leaf 依存（`ruviform-rt`・
  `ruviform-media-core`）のみであることを明記する。
- **R2（横断合意事項の網羅）**: Issue 本文の合意事項
  1〜10（Core/RT・Collector/Metrics・Tracing・Logging・
  Profiling・Diagnostics・Debug・統合運用・安全/所有権・
  両 Consumer）の全てが節として存在し、各項目が対応する
  per-issue 正本（`issue-276`〜`issue-293` の設計文書・
  ADR obs-05）への参照を持つ。
- **R3（参照の実在性）**: 設計文書・spec・plan・task・
  本 test-spec が参照する Issue 番号（#101/#103/#104/#121/
  #185/#219/#275〜#281/#289〜#294・AL #138・SV #81）・
  docs パス（per-issue spec/design/test-spec・#101/#103
  前提 2 文書・ADR obs-05・#294・#219 系文書）・crate
  名（`ruviform-observability`）・モジュール名
  （`core`/`source`/`collector`/`metrics`/`diagnostics`/
  `debug`/`record`/`rt_bridge`/`rt_timing`/`profiling`/
  `tracing_export`/`logging`/`integration`）・Cargo
  feature 名（`logging`/`tracing-export`/`allocation-
  probe`/`integration`）が作成時点で実在する。存在しない
  Issue/文書/モジュール/feature を根拠として記載しない。
- **R4（Feature/DAG 突合の正確性）**: 合意 feature 構想
  （`collector`/`diagnostics`/`debug-tools` を含む）と
  `crates/ruviform-observability/Cargo.toml` の実際の
  feature 集合（`default=[]`・`logging`・`tracing-
  export`・`allocation-probe`・`integration`）および
  `src/lib.rs` の `cfg(feature)` 配置との対応表が存在し、
  「feature として未採用 → 無条件モジュール」の対応が
  `collector`/`metrics`/`diagnostics`/`debug` の 4 領域
  について正確に記述される。compile matrix 未実施は
  `NotRun` と記録される。
- **R5（RT/non-RT 境界）**: RT writer の許可操作
  （事前準備・有界・no heap allocation/lock/IO/
  Subscriber）、「Cargo Feature 分離だけで RT 安全を
  保証しない」原則、RT から Span を開かない・Poll 時刻を
  実事象時刻にしない規則、`allocation-probe` の Global
  Allocator 宣言が Host/Test Binary 責務であることが
  記述される。
- **R6（Source/Generation/Clock/Loss 非偽装）**: Source
  identity の mint/re-mint 境界・Generation/Clock
  Domain/epoch 不一致の報告・loss の bridge counter/
  `dropped_before` 可視化・clock anomaly の計数・
  staleness/validity の Snapshot 明示・「未観測≠ゼロ」
  「Trace ID/Correlation Token は認可 capability でない」
  が per-issue 正本への参照付きで記述される。
- **R7（Consumer 利用面）**: AudioLens の維持要件
  （既定 OFF・`AL_LOG`/flag・stderr text/file JSONL・
  `inspect` stdout 保護・daily 7 世代 retention・失敗時
  degrade・JSON schema/error exit code Golden・機密非出力・
  f64/samply 数値精度）と Synvelis の境界（Host のみが
  Subscriber 設置・Multiwindow double initialization 禁止・
  Playback/Plugin/Gateway の RT 安全・Gateway
  Grant/Approval/Audit/Media Provenance の独立性・
  Project/Snapshot/Schema/Version/Migration の Consumer
  所有）が記述され、AL #138・SV #81 の Issue 番号が
  含まれる。
- **R8（証拠項目対応）**: Issue #275 の受入 Gate 9 項目
  全てが、実在する per-issue test-spec・crate contract
  test・#281 G0〜G4・#219/#294 機構・Consumer Issue へ
  対応付けられている。
- **R9（設計/実装分離）**: 「Design approved」は PR・
  PoC・実機 RT 保証・Cross-Consumer 移行完了を意味し
  ない旨、Issue 状態 snapshot（OPEN/CLOSED）とコード
  存在・受入証拠の区別、未実施項目を `not run`/`NotRun`/
  `Unsupported` と記録する方針が明記される。
- **R10（#101/#103 前提更新）**: `foundation-
  observability-minimal-api.md` と `shared-foundations-
  and-modularization-policy.md` に合意版への改訂注記が
  存在し、歴史的分析本文を改変せず影響変更（front
  matter・注記）を記録している。
- **R11（変更範囲）**: 差分が `docs/` の新規
  `issue-275-*` 文書・2 件の前提文書注記・生成 index
  のみであり、`crates/`・`Cargo.toml`・`Cargo.lock`・
  `mise.toml`・`.github/` に変更がない。

## Examples

- Given Issue #275 本文の「`ruviform-observability-
  collector`/`-tracing`/`-logging` など独立クレートを
  新設しない」、When 単一クレート構成節を検査する、
  Then 同旨の固定と #101「内部モジュール先行」への言及・
  論理依存 DAG・leaf 依存 ADR への参照を持つ（R1）。
- Given `Cargo.toml` に `collector`/`diagnostics`/
  `debug-tools` feature が存在しない実装、When feature
  突合表を検査する、Then 3 feature が「未採用→無条件
  モジュール」として `collector`/`metrics`/`diagnostics`/
  `debug` に対応付けられ、compile matrix が `NotRun` と
  記録される（R4）。
- Given consumer-verify の AL/SV lane が `not run`、
  When Consumer 受入を判定する、Then 「両 Consumer 受入
  済み」とは記録されず `NotRun` のまま残る — 本書は
  Consumer 独立判定と設計/実装分離を明記している（R7/R9）。
- Given 本 PR の diff、When `git status`/`git diff --stat`
  を確認する、Then 変更は `docs/` の `issue-275-*` 新規
  文書・前提 2 文書の注記・生成 index のみ（R11）。

## Guarantees

- 横断契約の記述は per-module 正本（#276〜#293 の設計・
  test-spec・ADR obs-05）への参照で構成され、詳細の複写
  による drift を避ける構造。
- feature matrix・module 存在・Issue 状態は
  `Cargo.toml`/`src/lib.rs`/`gh issue view` の実測に基づく
  snapshot であり、独自の未検証な主張を追加しない。
- 証拠項目対応表の各行は実在する正本への参照のみを持ち、
  存在しない Issue/文書/command を根拠にしない。
- Consumer の schema/version/canonical 意味論・認可/
  Audit/Provenance の所有と、Ruviform が提供する観測機構
  の責務分界が #281・#219 と同一原則で記述される。
- 「Design approved ≠ 実装/検証/PoC/実機 RT/Consumer 受入
  完了」の分離が spec・design・plan・task の全てで一貫
  している。

## Non-goals

- per-module の実走テスト合否（per-issue test-spec と
  crate `tests/` が正本）。
- Consumer repo での parity/byte 互換/RT latency の実測
  （#219/#294 consumer-verify・AL #138/SV #81 が Owner）。
- 実機 RT 計測・負荷試験・PoC 完了判定（#281 の段階
  Gate）。
- macOS/Linux 実機・MSRV 全組合せの compile matrix 実行。
- `crates/`・`Cargo.toml`・`mise.toml`・`.github/` の変更。

## Risk

- 本 test-spec が検証するのは横断設計文書の正しさで
  あり、Issue #275 の受入 Gate 実行（G-1〜G-8 の証拠
  収集）は #281/#219/#294 の gate 段階で別途検証される。
  設計文書が完璧でも gate 未実施なら Issue #275 の受入は
  未完である — この分離を崩すと「設計合意＝受入完了」の
  誤認が起きる。
- 参照先 Issue の状態（OPEN/CLOSED）は変動する。本書は
  2026-11-09 時点の snapshot（#276/#277/#279/#280 CLOSED、
  #278/#281/#289〜#294 OPEN、AL #138/SV #81 CLOSED）を
  記録しており、将来の状態変化で接続表の「役割」記述が
  古くなる可能性がある — snapshot 日付の明記で緩和する。
