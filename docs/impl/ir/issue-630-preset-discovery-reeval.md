---
id: "IMPL-630presetDiscoveryReeval07"
type: "impl"
status: "completed"
title: "Issue #630 preset-discovery 外部検証再評価 slice — 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-630-preset-discovery-reeval.md"
    - "docs/plans/issue-630-preset-discovery-reeval.md"
    - "docs/specs/issue-630-preset-discovery-reeval.md"
    - "docs/designs/issue-630-preset-discovery-reeval.md"
  changes:
    added:
      - type: "file"
        path: "evidence/rf630/preset-discovery-reeval-verification.md"
      - type: "file"
        path: "docs/discovery/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/specs/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/designs/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/plans/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/tasks/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/test-specs/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-discovery-reeval.jsonl"
    modified:
      - type: "file"
        path: "ci/plugin-abi-validator/run.mjs"
      - type: "file"
        path: "ci/plugin-abi-validator/tests/run.test.mjs"
      - type: "file"
        path: "evidence/rf630/abi-validator/run.json"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_gain.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_fault.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_stateful.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_instrument.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_editor.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_spawn_fail.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_fault-unfiltered.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/vst3-validator.txt"
  implements:
    - "docs/tasks/issue-630-preset-discovery-reeval.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-preset-discovery-reeval.md"
  blocks: []
  supersedes: []
  superseded-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-preset-discovery-reeval.jsonl"
    rejected: []
---

# Issue #630 preset-discovery 外部検証再評価 slice — 実装記録

## 実装したもの

- `ci/plugin-abi-validator/run.mjs`: `CLAP_FIXTURE_LIBRARY_EXCLUDES`
  を削除し fixture target を `excludes: []` に変更。missing-validator
  message を upstream-commit build 前提へ更新し、stale comment を修正。
  `spawnCapture` の file-backed stdio は維持。
- `ci/plugin-abi-validator/tests/run.test.mjs`: 除外 import を除去し、
  preset-discovery を含まないことを固定する contract test へ更新。
- `evidence/rf630/abi-validator/`: pinned upstream-commit
  clap-validator で canonical lane を再実走し run.json/raw txt を更新。

## 実行結果

- `node --test ci/plugin-abi-validator/tests/`: 32 pass / 0 fail。
- `CLAP_VALIDATOR=.../pin-47e2424/clap-validator.exe
  VST3_VALIDATOR=.../vst3sdk-build/bin/Release/validator.exe
  mise run test-plugin-abi-validator`: verdict **Passed**、exit 0。
  canonical 7 leg 全 Passed。preset-discovery crawl/load は raw
  `clap-*.txt` で success として観測され、run.json の fixture CLAP
  leg に preset-discovery exclusion は無い（test_fault の
  `param-(set|fuzz)` のみ残存）。informational fault leg は従来どおり
  Unsupported（validator exit 101、JSON 無し）で verdict 対象外。
