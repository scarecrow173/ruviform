---
id: "TASK-034LNYvBEBtmjJIUa6xJMi"
type: "task"
status: "todo"
title: "Issue #76 プラグイン L1 分離 closeout"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/tasks/plugin-l1-final-verification.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-l1-wire-codec-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #76 プラグイン L1 分離 closeout

## Work

canonical closeout（plan action 19）: 実績 IR（`docs/impl/ir/`）・coverage 記述・task/plan status 更新（実検証 + review 後のみ・全 task 完了の実 closeout 後に plan を `"completed"` へ遷移）。`verified-by` の 3 test-spec（`plugin-l1-wire-codec-tests`・`plugin-l1-proxy-contract-tests`・`plugin-l1-runner-e2e-tests`）を実結果で confirmed に更新。GitHub merge は lead の authoritative `gh` のみ（文書チェックリストで代行しない・PR merged 主張なし・issue closure は merge 実績確認後）

## Verification

- [ ] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` — exit 0・0 findings（全 doc 編集後に実実行）
- [ ] `rg -q '^status: "completed"$' docs/plans/implement-issue-76-plugin-l1-out-of-process.md` — exit 0（全 7 task 完了の実 closeout 後に plan status を `"completed"` へ遷移済み）
- **expected**: IR/coverage/status は immutable な実 log のレビューから記述 — 新規メトリクス再計算なし・検証未実施のまま status を進めない。GitHub merge は lead の authoritative `gh` のみ
