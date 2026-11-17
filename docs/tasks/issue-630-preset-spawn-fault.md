---
id: "TASK-630presetSpawnFault05"
type: "task"
status: "done"
title: "Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — task"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-630-preset-spawn-fault.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-630-preset-spawn-fault.md"
  implements:
    - "docs/plans/issue-630-preset-spawn-fault.md"
  implemented-by:
    - "docs/impl/ir/issue-630-preset-spawn-fault.md"
  depends-on:
    - "docs/plans/issue-630-preset-spawn-fault.md"
    - "docs/tasks/issue-630-test-stateful.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — task

- [x] VST3 preset surface（IUnitInfo/IUnitHandler/IUnitHandler2 +
  IProgramListData/IUnitData）+ in-process ABI test
- [x] CLAP preset surface（preset-discovery factory + preset-load
  extension）+ in-process ABI test
- [x] 両 l1-runner bin の `--ruviform-test-fault=MODE`（exit/abort/
  hang/stderr/bad-fixture）
- [x] `test_spawn_fail`（VST3 class / CLAP descriptor、scan 通過・
  create 拒否）
- [x] L1 spawn-fault e2e（16 tests per ABI）+ host-path 期待更新
- [x] `mise run test-l1-real-plugin` 全緑・fmt/clippy/test-rt/prek
- [x] evidence 記録（`evidence/rf630/test-preset-spawn-fault-verification.md`）
- [x] 逸脱根拠（runner argv 採用）を design doc / PR 本文に記録
- [x] doc chain 完成・commit・push・PR（Refs #630）
