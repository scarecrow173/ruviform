---
id: "TASK-034Smunkp1KgV6Pfg3HUvX"
type: "task"
status: "done"
title: "T1 ci/summary.mjs fail-closed 評価器 + contract tests（Issue #366 Stage 1）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/366"
  changes:
    added:
      - type: "directory"
        path: "ci/summary"
    modified:
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-366-full-ci-stage1.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-366-full-ci-stage1.md"
  blocks:
    - "docs/tasks/issue-366-t2-ci-yml-full-ci.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-366-t2-ci-yml-full-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-366-full-ci-stage1.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T1 ci/summary.mjs fail-closed 評価器 + contract tests（Issue #366 Stage 1）

## Work

`ci/summary/summary.mjs`（stdlib-only）と `ci/summary/tests/summary.test.mjs`（`node --test`）を新設し、mise task `test-ci-summary` を追加する。旧 `ci/impact/gate.mjs` と同じ入力規約（`NEEDS_JSON` env または `--needs <file>`）を踏襲するが、分類期待集合の概念は持たない — 全 needs job が `success` であることだけを判定する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-366-full-ci-stage1.md`
- `docs/designs/issue-366-full-ci-stage1.md`

## Dependencies

- なし（先行タスクなし。T2 が本タスクの成果物に依存する）

## Affected Files

- `ci/summary/summary.mjs`（新設）
- `ci/summary/tests/summary.test.mjs`（新設）
- `mise.toml`（`test-ci-summary` 追加）

## Done When

- `summary.mjs` が以下を判定する: 全 needs job `result === 'success'` → pass・`failure`/`cancelled`/`skipped`/未知値/`needs` key 欠落 → fail・宣言 14 leg の `checkout-sha-<leg>-attempt-<n>` 実測証跡の欠落/不正/命名規約外/attempt 間乖離 → fail・`EXPECTED_SHA` 非空かつ実測共通値と不一致/実測不能 → fail・NEEDS_JSON parse 失敗 → fail。leg の実測値は最大 run attempt を採用（`needs` 結果の最新 attempt 基準と整合）。`HEAD_SHA` は summary job 自身の `git rev-parse HEAD` 実測値（`github.sha` は `EVENT_SHA` として別途裏付け記録）。
- Job Summary（`GITHUB_STEP_SUMMARY`）へ event・head sha・expected sha・run URL・job 別結果表・verdict を書く。`--summary <file>` でファイル出力も可能。
- `ci-summary-report.json` を `--report <file>` で出力（`kind: "ci-summary"`・event・headSha・expectedSha・runUrl・verdict・reasons・jobs[]）。
- contract tests が verdict 行列・expected_sha 3 態・report 形状をカバーし、`mise run test-ci-summary` が exit 0。
