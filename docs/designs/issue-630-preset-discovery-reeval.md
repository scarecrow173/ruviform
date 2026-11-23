---
id: "DESIGN-630presetDiscoveryReeval03"
type: "design"
status: "approved"
title: "Issue #630 preset-discovery 外部検証再評価 slice — 設計"
created: "2026-11-17"
updated: "2026-11-23"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-preset-discovery-reeval.md"
    - "docs/discovery/issue-630-preset-discovery-reeval.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-preset-discovery-reeval.md"
  implements:
    - "docs/specs/issue-630-preset-discovery-reeval.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-preset-discovery-reeval.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 preset-discovery 外部検証再評価 slice — 設計

## 方針

runner の fail-closed leg 集約と schema `rf630-abi-validator-run/2`
を維持し、tool 側 workaround だった library-level exclusion だけを
除去する。validator pin は upstream commit build とし、実行証拠は
従来どおり run.json + raw txt に残す。

## 変更点

- `CLAP_FIXTURE_LIBRARY_EXCLUDES` を削除し、fixture target の
  `excludes: []` を明示。production target も従来どおり空。
- `clapLegExclusions` は target/plugin の宣言的除外を合成する汎用
  機構として残す（現行 fixture では test_fault の `param-(set|fuzz)`
  のみ）。
- `spawnCapture` の file-backed stdio は維持する。0.4.1 deadlock は
  解消したが、fault-injection child が pipe を保持する問題は残り得る
  ため、完了 report を timeout で隠さない境界として必要。
- contract test は「preset-discovery を含む library exclusion が無い」
  ことを固定し、generic merge 契約は synthetic target で検証する。

## 失敗時の倒れ方

preset-discovery case が failed/crashed なら通常 leg verdict で
Failed。validator が JSON 無しで落ちた場合は Unsupported/Failed の
既存経路。informational fault leg は引き続き verdict 対象外。
