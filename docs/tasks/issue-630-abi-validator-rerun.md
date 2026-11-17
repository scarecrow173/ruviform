---
id: "TASK-630abiValidatorRerun05"
type: "task"
status: "done"
title: "Issue #630 外部 ABI validator 再実走 slice — task"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-630-abi-validator-rerun.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-abi-validator-rerun.md"
  implements:
    - "docs/plans/issue-630-abi-validator-rerun.md"
  implemented-by:
    - "docs/impl/ir/issue-630-abi-validator-rerun.md"
  depends-on:
    - "docs/plans/issue-630-abi-validator-rerun.md"
    - "docs/tasks/issue-630-abi-validator.md"
    - "docs/tasks/issue-630-preset-spawn-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator 再実走 slice — task

- [x] origin/main から `feature/630-abi-validator-rerun` 作成・
  validator pin binary 確認
- [x] post-#735 surface の実走プローブ（spawn_fail 署名・
  preset-discovery deadlock・VST3 setup 失敗行を確認）
- [x] `run.mjs`: CLAP `expectCreateDecline` +
  `CLAP_FIXTURE_LIBRARY_EXCLUDES`/`clapLegExclusions` +
  `VST3_DECLARED_UNCREATABLE`/`applyVst3DeclaredUncreatable` +
  schema `rf630-abi-validator-run/2`
- [x] `tests/run.test.mjs`: declared create-decline・unexpected
  decline/crash・preset-discovery exclusion・VST3 uncreatable の
  contract test 追加（32 test 全 pass）
- [x] `mise run test-plugin-abi-validator` 再実走 → verdict Passed
  exit 0、run.json /2 + raw txt 更新
- [x] 非回帰: fmt-check / check・lint・test（両 fixture crate）/
  test-l1-real-plugin / test-rt / prek 全て実 exit code 記録
- [x] evidence 検証記録
  （`evidence/rf630/abi-validator-rerun-verification.md`）
- [x] doc chain 完成・review-protocol レビュー・commit・push・
  PR（Refs #630）
