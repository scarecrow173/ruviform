---
id: "TASK-630abiValidator05"
type: "task"
status: "done"
title: "Issue #630 外部 ABI validator slice — task"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-630-abi-validator.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-abi-validator.md"
  implements:
    - "docs/plans/issue-630-abi-validator.md"
  implemented-by:
    - "docs/impl/ir/issue-630-abi-validator.md"
  depends-on:
    - "docs/plans/issue-630-abi-validator.md"
    - "docs/tasks/issue-630-test-stateful.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator slice — task

- [x] clap-validator 0.4.1 provisioning（pin version・license・sha256 記録）
- [x] VST3 SDK validator 導入経路確定（source build・commit `3cdf9ca`・license 確認）
- [x] CLAP fixture 修正（module path / fractional rate / MAX_EVENTS / instrument / stateful rescan）
- [x] VST3 fixture 修正（terminate 非終端 / setupProcessing / bus info）
- [x] `ci/plugin-abi-validator/run.mjs` + `tests/run.test.mjs`（fail-closed）
- [x] canonical 実走（`node ci/plugin-abi-validator/run.mjs` exit 0）
- [x] evidence 記録（`evidence/rf630/abi-validator/` + verification.md）
- [x] 正規検証コマンド一式の実行と exit code 記録
- [x] doc chain 完成・commit・push・PR（Refs #630）
