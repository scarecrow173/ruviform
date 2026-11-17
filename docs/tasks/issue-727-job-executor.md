---
id: "TASK-034Z0uGBWlESDKh8X21tPy"
type: "task"
status: "done"
title: "Issue #727 bounded 非 RT job executor — 責務判断決裁・API 設計・検証"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["devin-worker"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/727"
    - "https://github.com/scarecrow173/synvelis/issues/518"
  changes:
    added:
      - type: "file"
        path: "evidence/rf727/"
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
    modified:
      - type: "file"
        path: "docs/designs/shared-foundations-and-modularization-policy.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-727-job-executor.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-727-job-executor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-721-post-seal-temp.md"
    - "docs/tasks/issue-716-runtime-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-727-job-executor.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-727-job-executor.md"
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

# Issue #727 bounded 非 RT job executor — 責務判断決裁・API 設計・検証

## Work

`scarecrow173/ruviform#727` の責務判断 slice。bounded 非 RT
job executor primitive を Ruviform `ruviform-exec` module
追加で決裁し、API 設計・stale 表記の最小修正・検証証拠を
確定する docs-only 作業。

1. `crates/ruviform-exec` 既存 primitive（`OfflineJob`・
   `OfflineDriver`・`CancellationToken`・`Progress`・
   `JobScope`/`JobState`/`JobEvent`/`JobOutcome`/`JobError`）
   の契約を read-only で確認。
2. consumer 要件を read-only で照合（synvelis
   `orchestrator.rs` の `RenderExecutor` port・同期
   `run()`/`drive()`・`ruviform_exec::CancellationToken` 使用、
   audio-lens `al-job` の smoke test 利用）。
3. 責務判断を決裁: Ruviform `ruviform-exec` module 追加
   （synvelis-driven 実行境界 ADR Decision 6・shared-
   foundations 分割判断基準・module-first 原則に基づく）。
4. `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
   に ADR を作成（`relations.source` に sv#518 URL を記載）。
5. `docs/designs/issue-727-bounded-nonrt-job-executor.md` に
   API 契約（bounded 性・cancel/preemption・結果 join・RT
   分離）を記述。
6. `docs/designs/shared-foundations-and-modularization-policy.md`
   L282 の stale 行番号参照を最小修正 + ADR 参照追加。
7. spec→design→plan→task→test-spec→impl/ir・exp の文書
   チェーンと `evidence/rf727/` を作成し、比例検証を実測。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-727-job-executor.md`（approved）
- `docs/designs/issue-727-bounded-nonrt-job-executor.md`
  （approved）
- `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md`
  （accepted）
- `docs/plans/issue-727-job-executor.md`（approved）
- `docs/test-specs/issue-727-job-executor.md`（approved）
- `POLICIES.md`

## Dependencies

- synvelis-driven 実行境界 ADR Decision 6（汎用実行契約 =
  Ruviform / Job Registry = Synvelis）— 確定済み。
- baseline `ruviform-exec` 行（Primitives 層・非 RT 専用・
  許可依存制約）— 確定済み。
- consumer 要件（synvelis `orchestrator.rs`・audio-lens
  `al-job`）は read-only 参照で照合済み。
- `docs/designs/shared-foundations-and-modularization-policy.md`
  は本 slice で L282 のみ最小修正（他行は変更しない）。

## Affected Files

- `docs/specs/issue-727-job-executor.md` — 新規
- `docs/designs/issue-727-bounded-nonrt-job-executor.md` — 新規
- `docs/adr/issue-727-bounded-nonrt-job-executor-placement.md` —
  新規
- `docs/plans/issue-727-job-executor.md` — 新規
- `docs/tasks/issue-727-job-executor.md` — 本文書
- `docs/test-specs/issue-727-job-executor.md` — 新規
- `docs/impl/ir/issue-727-job-executor.md` — 新規
- `docs/impl/exp/issue-727-job-executor.md` — 新規
- `docs/impl/exp/issue-727-job-executor-exp.jsonl` — 新規
- `docs/designs/shared-foundations-and-modularization-policy.md`
  — L282 の 1 行のみ最小修正
- `evidence/rf727/` — baseline/事後の検証証拠
- `docs/*/README.md` — index 未同期のまま（Coordinator が
  統合時に再生成・本 slice では触れない）

## Done When

- [x] 責務判断が「Ruviform `ruviform-exec` module 追加」で
      結論づけられ、根拠が ADR に記録される
- [x] API 設計 doc が作成され、bounded 性・preemption・
      cancel・結果 join・RT 分離の契約が定義される
- [x] sv#518 から stable URL で参照可能
      （ADR `relations.source` の sv#518 URL）
- [x] shared-foundations L282 の stale 表記の扱いが ADR に
      記録され、該当行が最小修正される
- [x] `doc_status` lint 前後差分で新規文書由来の finding が
      ないことを確認
- [x] `route_graph` が baseline と同等以下（blocked は既存
      baseline finding のみ）
- [x] `mise run fmt-check` が exit 0
- [x] `prek run --files` が変更ファイルで Pass
- [x] review-protocol self-review で blocker なし

## Verification

- [x] `node .agents/skills/doc-status/scripts/doc_status.js lint
      --dir docs/adr`（+ designs/specs/plans/tasks/test-specs/
      discovery/impl）— Pass（baseline 採取済み・新規文書由来
      の finding なし）
- [x] `node .agents/skills/doc-driven-dev-graph/scripts/
      route_graph.js --json` — baseline blocked（既存
      broken-relation）を記録・新規文書追加後も同等
- [x] `node .agents/skills/adr-doc/scripts/review_adr.js
      --dir docs/adr`・`audit_adr.js`・`check_code_links.js`
      — 実行・結果記録
- [x] `mise run fmt-check` — Pass（exit 0・docs-only）
- [x] `mise exec -- prek run --files <変更ファイル>` — Pass
- [ ] `*-msrv`・`*-full`・3 OS 横断・cargo 系 lane — 本 slice
      の明示対象外（docs-only・POLICIES.md §5・NotRun）
