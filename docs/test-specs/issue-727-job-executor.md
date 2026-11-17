---
id: "TSPEC-034Z0uy4qJcLIsiQTm3jM5"
type: "test-spec"
status: "approved"
title: "Issue #727 bounded 非 RT job executor 責務判断・API 契約 テスト仕様"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
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
    - "docs/test-specs/issue-721-post-seal-temp.md"
    - "docs/test-specs/issue-716-runtime-backend.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/issue-727-job-executor.md"
    - "docs/designs/issue-727-bounded-nonrt-job-executor.md"
    - "docs/adr/issue-727-bounded-nonrt-job-executor-placement.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
    - "https://github.com/scarecrow173/ruviform/issues/155"
    - "https://github.com/scarecrow173/ruviform/issues/316"
  defers: []
  deferred-by: []
metadata:
  issue: 727
---

# Issue #727 bounded 非 RT job executor 責務判断・API 契約 テスト仕様

## Purpose

bounded 非 RT job executor primitive の責務判断
（Ruviform `ruviform-exec` module 追加）が ADR・設計 doc・
spec 間で一貫して記録され、sv#518 が参照可能な形で確定した
ことを検証可能な形で残す。本 slice は docs-only であり、
executor の実装・動作検証は存在しない — 本 test-spec が
守る意図は「判断と API 契約の文書間整合」であり、実装の
正しさは実装 Issue の test-spec に委ねる。

## Feature

bounded 非 RT job executor primitive の配置決裁 — Ruviform
`ruviform-exec` への module 追加が、spec の Requirements・
設計 doc の API 契約・ADR の Decision Outcome で矛盾なく
記録され、consumer（sv#518）が必要とする最小 API に対応
していることを保証する。

## Rules

- **R1（責務判断の一意性）**: ADR の Decision Outcome が
  二者択一（Ruviform 側 / consumer 側）で一意に結論づけられ、
  採用・不採用の両選択肢の根拠が記録される。spec の R1 を
  満たす。
- **R2（配置決定の整合）**: Ruviform 側採用の場合、配置先
  （`ruviform-exec` 内 module vs 新規 crate）が baseline の
  依存方向制約と module-first 原則に照らして決定され、
  spec の R2 を満たす。
- **R3（API 契約の完全性）**: bounded 性（worker 数・queue
  容量・in-flight 上限）・cancel/preemption 意味論・結果
  join/observe・RT 分離が設計 doc で定義され、sv#518 の
  最小 API（`RenderExecutor` port・非同期投入・
  `CancellationToken`）に対応付けられる。spec の R3 を満たす。
- **R4（責務境界の維持）**: Job Registry・job ID・永続化・
  retry/resume・storage quota・user-facing scheduling・
  製品 feedback state が Ruviform primitive に取り込まれない
  ことが synvelis-driven 実行境界 ADR Decision 6 と整合する。
  spec の R4 を満たす。
- **R5（preemption の正直な定義）**: 「preemption」が任意
  Rust コードの強制中断と解釈されず、協調 cancel / budget
  checkpoint / worker-level scheduling のいずれかとして明示的
  に定義される。spec の R5 を満たす。
- **R6（sv#518 参照可能性）**: ADR front matter の
  `relations.source` に
  `https://github.com/scarecrow173/synvelis/issues/518` が
  含まれる。spec の R6 を満たす。
- **R7（stale 表記の扱い）**: baseline の「スレッドプール／
  ワーカープール」表記が Issue #24 実装より前の記述である
  ことが ADR で指摘され、改訂要否の結論が記録される。
  spec の R7 を満たす。
- **R8（docs-only）**: 本 slice で `crates/**`・ルート
  `Cargo.toml`・`Cargo.lock`・CI 設定等のコード・設定に
  差分がない。spec の R8 を満たす。
- **R9（文書間整合）**: spec・design・ADR・plan・task の
  間で、配置判断・API 契約・責務境界・stale 表記の扱いが
  矛盾しない。front matter の relations（implements/
  depends-on/derives-from/verifies/related/references）が
  双方向に整合する。

## Examples

- **E1（責務判断の一意性）**: Given ADR
  `issue-727-bounded-nonrt-job-executor-placement.md`、When
  Decision Outcome を読む、Then「Ruviform `ruviform-exec`
  module 追加」が一意に選択され、Option 1–4 の pros/cons が
  記録される。
- **E2（配置決定の整合）**: Given ADR と design doc、When
  配置先を照合、Then 両者が「`ruviform-exec` 内 module 追加」
  で一致し、新規 crate を作らないことが明記される。
- **E3（API 契約の完全性）**: Given design doc、When
  `ExecutorConfig`/`JobExecutor`/`QueuedJob`/`SubmitError`/
  `ExecutorError` の契約を読む、Then bounded 性（worker 数・
  queue 容量・in-flight 上限 = workers + queue_capacity）・
  cancel/preemption（協調 cancel・強制中断なし）・結果 join
  （`wait`/`try_wait`/`detach`）・RT 分離（non-RT contract
  付与・RT audio path 非関与）が定義される。
- **E4（責務境界の維持）**: Given design doc の Out of Scope
  と ADR の Decision Drivers、When synvelis-driven 実行境界
  ADR Decision 6 と照合、Then Job Registry・job ID・永続化・
  retry/resume・storage quota・user-facing scheduling が
  consumer 責務のまま残ることが確認できる。
- **E5（preemption の正直な定義）**: Given design doc の
  cancel/preemption 節、When 「preemption」の定義を読む、
  Then 協調 cancel + budget checkpoint に限定され、強制
  中断が提供されないことが明記される。
- **E6（sv#518 参照可能性）**: Given ADR front matter、When
  `relations.source` を読む、Then
  `https://github.com/scarecrow173/synvelis/issues/518` が
  含まれる。
- **E7（stale 表記の扱い）**: Given ADR の「baseline stale
  表記の扱い」節、When shared-foundations L282 の指摘と
  baseline の実際の記述を照合、Then stale 表記が Issue #24
  実装より前の記述であること・改訂要否の結論（改訂必要・
  baseline 改訂 Issue に委ねる）が記録される。
- **E8（docs-only）**: Given `git diff origin/main`、When
  変更ファイルを列挙、Then `crates/**`・`Cargo.toml`・
  `Cargo.lock`・`mise.toml`・CI 設定に差分がなく、docs と
  `evidence/rf727/` のみが変更される。

## Guarantees

- 責務判断が「Ruviform `ruviform-exec` module 追加」で一意に
  結論づけられ、根拠が ADR に記録される。
- API 契約が設計 doc で定義され、bounded 性・preemption・
  cancel・結果 join・RT 分離が明示される。
- sv#518 から stable URL で参照可能（ADR `relations.source`）。
- shared-foundations L282 の stale 表記の扱いが ADR に記録
  され、該当行が最小修正される。
- 文書チェーン（spec→design→ADR→plan→task→test-spec→
  impl/ir・exp）が front matter relations で双方向に整合する。

## Non-goals

- executor の実装・動作検証（実装 Issue の範囲）。
- `OfflineJob`/`OfflineDriver` の既存契約の変更検証。
- cargo 系 lane（`check`/`lint`/`test`/`build`）— docs-only
  slice のため NotRun。
- baseline 本文の改訂検証（baseline 改訂 Issue の範囲）。
- consumer リポジトリ（synvelis/audio-lens）の変更検証。

## Risk

- 責務判断が文書間で drift すると sv#518 が誤った前提で
  実装を進める — R9 の文書間整合ルールで緩和。
- API 契約が `OfflineJob` の既存 surface と矛盾すると実装
  時に再設計が必要になる — design doc の「Consumer 最小 API
  対応」節と `OfflineJob` surface との照合で緩和。
- `JobState` queued 表現の未確定事項が contract 上問題に
  なる — design doc で「区別可能であること」のみを要求し、
  具体表現は実装 Issue に委ねる形で緩和。
- route_graph baseline blocked が新規文書の問題と誤認される
  — baseline を `evidence/rf727/` に記録し、前後差分で
  新規 finding がないことを確認する形で緩和。

## Sources and Relations

- 検証対象: `docs/specs/issue-727-job-executor.md`・
  `docs/designs/issue-727-bounded-nonrt-job-executor.md`・
  `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
  （本 spec の verifies）。
- 上流: `docs/plans/issue-727-job-executor.md`・
  `docs/tasks/issue-727-job-executor.md`。
- 関連 test-spec: `docs/test-specs/issue-721-post-seal-temp.md`・
  `docs/test-specs/issue-716-runtime-backend.md`。
- 実測証拠: `evidence/rf727/`。
- 関連 Issue: `scarecrow173/ruviform#727`・
  `scarecrow173/synvelis#518`・`scarecrow173/ruviform#155`・
  `scarecrow173/ruviform#316`（RT 領域・非対象）。
