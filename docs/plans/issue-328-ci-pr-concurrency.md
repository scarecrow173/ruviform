---
id: "PLAN-SIZd1oW7Pd1d5Vi3BzCMFc"
type: "plan"
status: "approved"
title: "Issue #328 実施計画 — CI PR concurrency(cancel stale PR runs)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/328"
    - "docs/specs/issue-328-ci-pr-concurrency.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".github/workflows/consumer-impact-check.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "docs/specs/issue-328-ci-pr-concurrency.md"
  implemented-by:
    - "docs/tasks/issue-328-t1-ci-pr-concurrency.md"
  depends-on:
    - "docs/specs/issue-328-ci-pr-concurrency.md"
    - "docs/designs/issue-328-ci-pr-concurrency.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-328-ci-pr-concurrency.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-328-ci-pr-concurrency.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-328-ci-pr-concurrency.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #328 実施計画 — CI PR concurrency(cancel stale PR runs)

## Goal

`ci.yml` と `consumer-impact-check.yml` に Issue #328 推奨形の workflow-level `concurrency` block を追加し、同一 PR の stale run だけを cancel する。12 required checks・job 名・trigger は不変。

## Scope

1 plan・1 task・2 workflow 変更 + AGENTS.md 記述同期 + doc chain(spec/ADR/design/plan/test-spec/task/IR)。Rust コード・Cargo 系は無関係。

## Upstream Documents

- `docs/specs/issue-328-ci-pr-concurrency.md`(approved; AC1–AC6)
- `docs/adr/issue-328-ci-concurrency-grouping.md`(accepted; Option A)
- `docs/designs/issue-328-ci-pr-concurrency.md`(approved; group 解決表・edge case 検証)

## File Map

| file | 変更 |
| --- | --- |
| `.github/workflows/ci.yml` | `permissions:` 直後に `concurrency:` block 追加(設計書の snippet そのまま) |
| `.github/workflows/consumer-impact-check.yml` | `permissions:` 直後に同 block 追加 |
| `AGENTS.md` | ci.yml 記述 bullet に「同一 PR の旧 run は concurrency で cancel」旨を一文追加 |
| `docs/specs|adr|designs|plans|test-specs|tasks/issue-328-*.md` | doc chain 新規作成 |
| `docs/*/README.md` | doc_maintenance による managed index 再生成 |
| `docs/impl/ir/issue-328-*.md` | 実装記録(最終検証後) |

## Task Granularity

- **T1 — concurrency 適用 + 静的検証 + docs**: 両 workflow へ block 追加 → `actionlint` → `git diff` で job 構成非変更確認 → AGENTS.md 同期 → doc index 再生成 → doc audit で新規 blocker なし確認 → IR 作成。

## Dependency Graph

```text
spec + ADR → design → plan → test-spec → T1 (workflow + AGENTS.md + IR)
```

## Risk Register

| risk | 評価・緩和 |
| --- | --- |
| 式の typo で group が常に同一値になり全 run が単一 group を共有 | actionlint + 式の机上評価表(design)で排除。`github.event_name`・`github.ref`・`github.run_id` は concurrency で許可された `github` context の property |
| `cancel-in-progress` の式評価失敗 | `github.event_name == 'pull_request'` は boolean を返す単純式。公式ドキュメントにも `cancel-in-progress` の条件式例あり |
| 古い run の手動 re-run が最新 run を cancel | 既知 edge(ADR Consequences・design edge 表に記録)。fail-safe 方向・復旧手順あり |
| ruleset(#326)との相互作用 | job 名・workflow 名・check 名を変更しないため required check 定義への影響なし。cancelled check run は合格とみなされない点を test-spec INV で固定 |
| consumer-impact-check への追加が意図しない cancel を起こす | 同じ per-PR group 意味論。非必須 check のため required 欠落リスクなし。draft PR では job 自体が `if` で skip(従来通り) |

## Rollback

差分は `concurrency:` block の追加のみ。`git revert` で従来の無制限並列へ即座に戻せる。永続データ形式・feature flag・依存の変更なし。Acceptance の「Rollback で従来の全実行に戻せる」を満たす。

## Verification Matrix

| Spec AC | 検証 | 実施者 |
| --- | --- | --- |
| AC1/AC2 concurrency block | `actionlint` + diff レビュー + design group 解決表 | worker |
| AC3 同一 PR cancel 実測 | 本 PR の CI で 2 push 実験・Run URL 記録 | coordinator(worker は IR で NotRun/記録引継ぎ) |
| AC4 非 PR・別 PR 非 cancel | 式の静的評価(design group 解決表) | worker |
| AC5 偽装なし | check run 意味論の文書化(test-spec INV-4) | worker |
| AC6 actionlint green・job 非変更 | `actionlint` + `git diff` | worker |
