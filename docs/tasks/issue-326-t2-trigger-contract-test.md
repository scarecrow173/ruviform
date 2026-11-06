---
id: "TASK-kjitc7DOTjE43h2VO4qJgh"
type: "task"
status: "done"
title: "T2 trigger contract test（ci-yml-contract.test.mjs）+ dev-env 同期（Issue #326）"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "docs/plans/issue-326-ci-manual-only.md"
  changes:
    added:
      - type: "file"
        path: "ci/daily/tests/ci-yml-contract.test.mjs"
    modified:
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-326-ci-manual-only.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-326-ci-manual-only.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-326-t1-ci-yml-dispatch-only.md"
    - "docs/test-specs/issue-326-ci-manual-only.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-326-ci-manual-only.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T2 trigger contract test + dev-env 同期（Issue #326）

## Work

`ci/daily/tests/ci-yml-contract.test.mjs` を新設し、`.github/workflows/*.yml` を Node stdlib で scan する contract test を実装（test-spec C0-C9 — `on:` block 抽出・直接子限定の trigger 識別・全合法形 fixture self-check・コメント/入れ子 key 誤検出排除）。(a) 全ファイルに `pull_request`/`pull_request_target`/`push` trigger 0 件、(b) `ci.yml` の trigger 集合が厳密に `{workflow_dispatch}`（allowlist）+`expected_sha:`、(c) `ci-issue-tracker.yml` に `workflow_run:`。RED 実証（撤去前 state で失敗）→ GREEN を記録。`docs/development-environment.md` の `ci.yml` trigger・concurrency・checkstatus 記述を実態に同期（ADR-SYd9kRt5mEssMKoPG8hnSG 参照追加）。`mise run test-ci-daily`（`node --test ci/daily/`）で自動包含 — 新 mise task は作らない。

## Classification

- `normal-plan-task`

## Acceptance

- [x] contract test が `test-ci-daily` 経由で pass（RED→GREEN 記録済み）。
- [x] dev-env doc が dispatch 専用・disabled checkstatus・簡素 concurrency を記述。
- [x] `test-ci-daily`/`test-ci-tracker`/`test-ci-summary`/`actionlint`/`prek` 全 pass。
