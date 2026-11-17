---
id: "EXP-630presetSpawnFault08"
type: "impl"
status: "completed"
title: "Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 実験記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/impl/ir/issue-630-preset-spawn-fault.md"
    - "docs/impl/exp/issue-630-preset-spawn-fault-exp.jsonl"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-spawn-fault.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-spawn-fault-exp.jsonl"
  implements:
    - "docs/impl/ir/issue-630-preset-spawn-fault.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-630-preset-spawn-fault.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 test_stateful preset surface + spawn/pre-Hello fault slice — 実験記録

詳細な時系列イベントは companion JSONL
`docs/impl/exp/issue-630-preset-spawn-fault-exp.jsonl`
（`experiment_event.v1` schema）を参照。

## 要約

1. VST3 preset interface の component/controller 帰属を vendored
   binding で確定（controller: IUnitInfo/IUnitHandler系、
   component: IProgramListData/IUnitData）。
2. CLAP `preset-discovery` feature を Coordinator 承認のもと
   有効化。`state-context` は preset load が既存 state blob を
   再利用するため不要と判断し不採用。
3. spawn-fault は専用 bin 案から既存 l1-runner argv 案へ変更
   （no-argv 契約不変・fail-closed・pre-Hello 到達性同等）。
   ADR `issue-630-test-spawn-fault-runner-argv` に記録。
4. `InstanceLimit` 到達 probe が host pooling（Bundled 分散 /
   Dedicated 専用子）により到達不能と判明し test を削除。
5. pre-Hello 高速死亡の `FailureCause` 分類 race を観測し、
   `PRE_HELLO_DEATH` 集合 oracle に整理。
