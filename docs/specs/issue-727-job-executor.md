---
id: "SPEC-034Z0tP43L1BILteVZhtDI"
type: "spec"
status: "approved"
title: "Issue #727 bounded 非 RT job executor primitive の責務判断と配置"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/designs/issue-727-bounded-nonrt-job-executor.md"
      - type: "doc"
        path: "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
      - type: "doc"
        path: "docs/plans/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/tasks/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/test-specs/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/impl/ir/issue-727-job-executor.md"
      - type: "doc"
        path: "docs/impl/exp/issue-727-job-executor.md"
      - type: "file"
        path: "docs/impl/exp/issue-727-job-executor-exp.jsonl"
      - type: "file"
        path: "evidence/rf727/"
    modified:
      - type: "file"
        path: "docs/designs/shared-foundations-and-modularization-policy.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/plans/issue-727-job-executor.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/designs/offline-execution-driver.md"
    - "docs/designs/issue-155-headless-cli-foundation.md"
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
    - "docs/test-specs/issue-727-job-executor.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-727-job-executor.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
    - "https://github.com/scarecrow173/ruviform/issues/155"
    - "https://github.com/scarecrow173/ruviform/issues/24"
    - "https://github.com/scarecrow173/ruviform/issues/101"
    - "https://github.com/scarecrow173/ruviform/issues/316"
  defers: []
  deferred-by: []
metadata:
  issue: 727
---

# Issue #727 bounded 非 RT job executor primitive の責務判断と配置

## What Should Be Built

synvelis#518（production job system 配線 — async scheduler・
preemption・結果 join）が必要とする bounded な非 RT job
executor primitive の責務判断を確定する。判断は二者択一:

- **Ruviform 側に置く** — `ruviform-exec` の primitive として
  bounded worker pool + bounded queue + 協調 cancel を提供し、
  consumer が job 定義・スケジューリング方針・永続化を
  注入する形にする。
- **consumer 側に置く** — Synvelis の job system 層
  （synvelis-automation / synvelis-app）が自前の executor を
  持ち、Ruviform は提供しない。

Ruviform 側採用の場合は配置先（`ruviform-exec` 内モジュール
追加か新規 crate か）も併せて決め、設計 doc または ADR で
bounded 性・preemption・cancel の契約を定義する。consumer
側採用の場合は synvelis 側の受け入れ先を記録する。

本 slice は判断と設計までを成果とし、executor の実装コードは
作成しない（Issue #727「対象外」明記 — 採用時は別 Issue）。
あわせて、shared-foundations policy が指摘する
`ruviform-architecture-design-baseline.md` の「スレッドプール／
ワーカープール」stale 表記との整合を記録し、該当箇所の扱い
（baseline 本文は範囲外のため改訂要否の結論のみ）を残す。

## Why Now

- `ruviform-exec` は `OfflineJob`（1 ワーカーの薄い接着層）と
  `OfflineDriver`（`run_parallel` ごとの scoped worker）を持つ
  が、複数 job を bounded に受け付ける executor / scheduler
  primitive を意図的に提供していない（`lib.rs` 冒頭の
  責務記述で明記）。
- synvelis#518 は residual (a)「非同期ジョブスケジューリング・
  preemption」から依存解除されておらず、production job system
  の配線に bounded executor が必要とされる。本 Issue はその
  共通基盤候補の配置判断を求められている。
- Issue #155（headless CLI foundation）で executor / スレッド
  プールの共有化は「実需が出た時点で追加可能」として後送り
  されており、sv#518 がその最初の実需に相当する。
- 判断が遅れると Synvelis 側が product-specific な executor を
  先行実装するリスクがあり、後から共通化すると境界移行コストが
  増える。

## Users and Value

- **Synvelis（sv#518 consumer）**: bounded な job 実行基盤を
  Ruviform primitive として受け取り、Job Registry・スケ
  ジューリング方針・永続化・UI 配線に集中できる。cancel /
  preemption / 結果 join の契約が既存 `CancellationToken`・
  `Progress` と整合する。
- **Ruviform**: 実需が確認された段階で generic execution
  contract（synvelis-driven 実行境界 ADR Decision 6）の
  primitive 面を拡張し、module-first 原則に沿って crate 分割
  判断を行える。
- **AudioLens（`al-job`）**: 現状は smoke test での
  `ruviform-exec` 利用のみだが、将来の job runner 需要に対し
  同じ primitive を再利用できる潜在 consumer として記録する。

## Scope

### In Scope

- bounded 非 RT job executor primitive の責務判断の決裁
  （Ruviform 側 / consumer 側の二者択一 + 根拠記録）
- Ruviform 側採用の場合の配置先決定（`ruviform-exec` 内
  モジュール vs 新規 crate）と API 設計（bounded 性・
  preemption・cancel・結果 join の契約）
- consumer 側採用の場合の synvelis 側受け入れ先の記録
- 既存 `OfflineJob` / `OfflineDriver` との関係整理
- shared-foundations policy が指摘する baseline「スレッド
  プール／ワーカープール」stale 表記との整合記録と該当箇所の
  最小修正要否判断
- synvelis#518 から stable URL で参照可能な ADR の作成

### Out of Scope

- executor の実装コード（採用時は別 Issue）
- RT オーディオスレッド向け scheduler（rf#316 の領域 —
  本 Issue は非 RT 限定）
- Job Registry・job 定義・永続化・retry/resume・manifest・
  exit code 等の product/job-system 意味論（synvelis 側責務）
- docs index（`docs/**/README.md`）の再生成
- baseline 本文（`ruviform-architecture-design-baseline.md`）の
  改訂
- consumer リポジトリ（synvelis / audio-lens）の変更
- GitHub Issue コメント・依存登録

## Requirements

- **R1（責務判断の決裁）**: Ruviform 側採用か consumer 側採用
  かを一意に結論づけ、採用・不採用の両選択肢の根拠を ADR に
  記録する。
- **R2（配置決定）**: Ruviform 側採用の場合、`ruviform-exec`
  内モジュール追加か新規 crate かを、baseline の依存方向制約
  と module-first 原則に照らして決定する。
- **R3（API 契約）**: Ruviform 側採用の場合、bounded 性
  （worker 数・キュー容量・job 予算）・cancel/preemption
  意味論・結果 join/observe・RT 分離を設計 doc または ADR で
  定義し、sv#518 が要求する最小 API（`RenderExecutor` port・
  非同期投入経路・`CancellationToken` との関係）に対応付ける。
- **R4（責務境界の維持）**: Job Registry・job ID・永続化・
  retry/resume・storage quota・user-facing scheduling・製品
  feedback state を Ruviform primitive に取り込まないことを
  synvelis-driven 実行境界 ADR Decision 6 と整合させる。
- **R5（preemption の正直な定義）**: 「preemption」を任意
  Rust コードの強制中断と解釈せず、協調 cancel / budget
  checkpoint / worker-level scheduling のいずれかとして明示的
  に定義する。
- **R6（sv#518 参照可能性）**: ADR front matter の
  `relations.source` に
  `https://github.com/scarecrow173/synvelis/issues/518` を
  含め、stable URL で相互参照を成立させる。
- **R7（stale 表記の扱い）**: baseline の「スレッドプール／
  ワーカープール」表記が Issue #24 実装より前の記述である
  ことを ADR で指摘し、改訂要否の結論を記録する。
- **R8（docs-only）**: 本 slice では `crates/**`・ルート
  `Cargo.toml`・`Cargo.lock`・CI 設定等のコード・設定を
  一切変更しない。

## Acceptance Criteria

- [x] ruviform 側に置くか consumer 側に置くかの責務判断が
  結論づけられ、根拠が記録される（→ ADR）
- [x] ruviform 側に置く判断の場合、API 設計（設計 doc または
  ADR）が作成され、bounded 性・preemption・cancel の契約が
  定義される（→ `docs/designs/issue-727-bounded-nonrt-job-executor.md`）
- [x] consumer 側に置く判断の場合、その旨と synvelis 側の
  受け入れ先が記録される（→ N/A、ruviform 側採用のため）
- [x] 結論が synvelis#518 側の参照（設計判断の根拠）として
  記録できる形になる（→ ADR `relations.source` の sv#518 URL）

## Deferred Design Concerns

- **executor の実装**: 採用時は別 Issue。本設計は API 契約と
  bounded 保証の骨格までを定義し、内部実装（queue 構造・
  worker loop・panic 伝播の詳細）は実装 Issue で確定する。
- **preemption の将来拡張**: 現行設計は協調 cancel + budget
  checkpoint に限定する。強制中断が必要になった場合は別途
  ADR で検討する（unsafe 強制割り込みは現行方針で非対象）。
- **複数 executor instance / 優先度**: 現行は単一 bounded
  pool を想定。優先度付き scheduling や複数 pool 分離は
  consumer 要求が出た時点で再検討する（YAGNI）。
- **baseline 本文改訂**: 「スレッドプール／ワーカープール」
  表記の改訂は baseline 改訂 Issue の範囲。本 slice では
  改訂要否の結論のみを ADR に記録する。
- **al-job の実利用**: AudioLens 側の job runner が
  `ruviform-exec` executor を利用するかは潜在需要として
  記録するに留め、al-job 側の変更は行わない。

## Sources and Relations

- `docs/designs/ruviform-architecture-design-baseline.md` —
  クレート表（`ruviform-exec` 行）・スレッド構成・依存方向
  制約の正本。
- `docs/designs/shared-foundations-and-modularization-policy.md` —
  module-first 原則・分割判断基準（L299-312）・汎用利用者の
  実在トリガ（L336-337）・baseline stale 表記の指摘（L282）。
- `docs/adr/synvelis-driven-generic-execution-boundaries.md` —
  汎用実行契約 = Ruviform / Job Registry = Synvelis の責務
  分離（Decision 6）。
- `docs/designs/offline-execution-driver.md`・
  `docs/designs/issue-155-headless-cli-foundation.md` —
  `OfflineDriver`/`OfflineJob` の設計経緯と executor 後送り
  の記録。
- `crates/ruviform-exec/src/lib.rs`・`src/job.rs`・
  `src/offline.rs`・`crates/ruviform-exec/README.md` —
  既存 primitive の契約と「executor API 非提供」の明記。
- `synvelis/crates/synvelis-render/src/render_job/orchestrator.rs`
  （read-only 参照） — `RenderExecutor` port・同期
  `run()`/`drive()`・`ruviform_exec::CancellationToken` 使用。
- `audio-lens/crates/al-job/`（read-only 参照） — JobDefinition
  / registry / runner を持つが `ruviform-exec` 利用は smoke
  test のみ。
