---
id: "PLAN-630presetDiscoveryReeval04"
type: "plan"
status: "approved"
title: "Issue #630 preset-discovery 外部検証再評価 slice — 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-preset-discovery-reeval.md"
    - "docs/designs/issue-630-preset-discovery-reeval.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-preset-discovery-reeval.md"
  implements:
    - "docs/specs/issue-630-preset-discovery-reeval.md"
    - "docs/designs/issue-630-preset-discovery-reeval.md"
  implemented-by:
    - "docs/tasks/issue-630-preset-discovery-reeval.md"
  depends-on:
    - "docs/specs/issue-630-preset-discovery-reeval.md"
    - "docs/designs/issue-630-preset-discovery-reeval.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 preset-discovery 外部検証再評価 slice — 実装計画

1. pin 確認: upstream `47e2424` build を `%LOCALAPPDATA%` 配下へ配置し
   version/SHA-256/source HEAD を記録。
2. runner 更新: `CLAP_FIXTURE_LIBRARY_EXCLUDES` 削除、fixture
   `excludes: []`、stale comment 修正、missing-validator message を
   新 spec へ誘導。
3. contract test 更新: import/除外合成 test を新契約へ合わせる。
4. canonical 実走: `CLAP_VALIDATOR`/`VST3_VALIDATOR` を pin して
   `mise run test-plugin-abi-validator`、run.json/raw txt を更新。
5. 証拠・文書: preset-discovery 成功の確認、検証 md、doc chain、
   impl ir/exp を作成。
6. 非回帰: node test、fmt/check/lint/test、L1 lane、test-rt、prek。
7. review-protocol → commit → push → PR（Refs #630）。

`crates/`・`Cargo.lock`・`mise.toml`・`.github/**`・docs index は
変更しない。
