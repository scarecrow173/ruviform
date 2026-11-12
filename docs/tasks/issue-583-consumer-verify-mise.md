---
id: "TASK-034Wv7JmMeNPkMsHD0V8Cf"
type: "task"
status: "in-progress"
title: "Issue #583 T1 — 原因確定・mise 検証・consumer-verify dispatch・doc-chain"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/583"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-583-consumer-verify-mise.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-583-consumer-verify-mise-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-583-consumer-verify-mise-plan.md"
    - "docs/specs/issue-583-consumer-verify-mise.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-583-consumer-verify-mise.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-583-consumer-verify-mise-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-583-consumer-verify-mise.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #583 T1 — 原因確定・mise 検証・consumer-verify dispatch・doc-chain

## Objective

consumer-verify preflight の `mr_boxington` 失敗について、直接原因を証拠つきで確定し、現行 main で `mise install --locked` 系コマンドと consumer-verify の preflight が回復していることを検証・記録する。設定変更は行わない。

## Checklist

- [x] 失敗 run(43140257916)・ci run(43140254535 / 43270011255)のログからエラー箇所と lane 差を確定
- [x] 旧設定(@4f31460e)を mise 2026.11.7 + clean global config で再現し同一エラーを確認
- [x] `mise install --locked node` が exit 0(mise 2026.11.7)
- [x] `MISE_ENV=ci mise install --locked rust git-lfs node` が exit 0(同)
- [x] `MISE_ENV=ci mise install --locked`(全 7 tool)と fresh `MISE_DATA_DIR` での node install が exit 0
- [x] doc-chain(spec/design/plan/task/test-spec)作成
- [x] `git merge origin/main` → 触った docs dir の管理 index を doc-maintenance で再生成
- [ ] commit → push → PR(`Refs #583`)
- [ ] `gh workflow run consumer-verify.yml --ref <branch>` で preflight success を確認し PR 本文に URL 記載
