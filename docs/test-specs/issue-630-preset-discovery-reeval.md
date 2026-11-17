---
id: "TSPEC-630presetDiscoveryReeval06"
type: "test-spec"
status: "approved"
title: "Issue #630 preset-discovery 外部検証再評価 slice — test specification"
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
        path: "docs/test-specs/issue-630-preset-discovery-reeval.md"
  implements:
    - "docs/specs/issue-630-preset-discovery-reeval.md"
    - "docs/designs/issue-630-preset-discovery-reeval.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-abi-validator-rerun.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 preset-discovery 外部検証再評価 slice — test specification

## Rule: preset-discovery は除外されず実行されなければならない

fixture CLAP target は library-level exclusion を持たず、canonical
leg の command に `--exclude preset-discovery-(crawl|load)` を含ま
ない。raw `clap-*.txt` には `preset-discovery-crawl` /
`preset-discovery-load` / `preset-discovery-descriptor-consistency`
の結果が残り、crawl/load が success または正直な failed として観測
される（timeout・JSON 無しは Unsupported/Failed）。

- Example: `clap-test_gain.txt` に preset-discovery 3 case があり、
  crawl/load が `success`。
- Example: run.json の test_gain leg に `exclusions` が無い。

## Rule: 除外合成は target/plugin の宣言のみを反映する

`clapLegExclusions` は target.excludes と plugin.exclude を合成する
が、preset-discovery を含む既定除外を持たない。fixture target
（`excludes: []`）では gain に除外 0、fault に `param-(set|fuzz)`
のみ、informational fault run でも plugin 除外を解除した結果が
target 除外だけになる。

- Example: `clapLegExclusions({ excludes: [] }, test_gain)` は `[]`。
- Example: `clapLegExclusions({ excludes: [] }, test_fault,
  { includePlugin:false })` は `[]`。

## Rule: fail-closed verdict を維持する

preset-discovery を実走しても、他の failed/crashed/NotRun/
Unsupported canonical leg があれば verdict は Passed にならない。
informational leg は verdict に含めない。
