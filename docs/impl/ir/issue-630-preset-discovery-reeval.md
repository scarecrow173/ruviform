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
        path: "docs/adr/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-discovery-reeval.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-preset-discovery-reeval-exp.jsonl"
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
  related:
    - "docs/adr/issue-630-preset-discovery-reeval.md"
    - "evidence/rf630/preset-discovery-reeval-verification.md"
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-630-preset-discovery-reeval-exp.jsonl"
    rejected: []
---

# Issue #630 preset-discovery 外部検証再評価 slice — 実装記録

## Summary

upstream fix（PR #41 / commit `47e2424`）を含む clap-validator build で
CLAP fixture の preset-discovery crawl/load を実走し、前 slice の宣言的
除外を解除した。canonical lane `mise run test-plugin-abi-validator` は
verdict Passed / exit 0。併せて validator pin/provisioning 判断を ADR に
記録し、検証記録へ `--version`・license・build/provision 手順を追記、
experiment JSONL を lease 命名 `-exp.jsonl` へ rename した。

## Context

- Task: `docs/tasks/issue-630-preset-discovery-reeval.md`
- Plan: `docs/plans/issue-630-preset-discovery-reeval.md`
- Spec: `docs/specs/issue-630-preset-discovery-reeval.md`
- Design: `docs/designs/issue-630-preset-discovery-reeval.md`
- ADR: [docs/adr/issue-630-preset-discovery-reeval.md](../../adr/issue-630-preset-discovery-reeval.md)
  （accepted — clap-validator を release pin ではなく upstream commit
  build pin とする決定記録）
- Discovery: `docs/discovery/issue-630-preset-discovery-reeval.md`
- Verification: `evidence/rf630/preset-discovery-reeval-verification.md`

## Implementation

### Changes

- `ci/plugin-abi-validator/run.mjs`: `CLAP_FIXTURE_LIBRARY_EXCLUDES`
  を削除し fixture target を `excludes: []` に変更。missing-validator
  message を upstream-commit build 前提へ更新し、stale comment を修正。
  `spawnCapture` の file-backed stdio は維持。
- `ci/plugin-abi-validator/tests/run.test.mjs`: 除外 import を除去し、
  preset-discovery を含まないことを固定する contract test へ更新。
- `evidence/rf630/abi-validator/`: pinned upstream-commit
  clap-validator で canonical lane を再実走し run.json/raw txt を更新。
- `docs/adr/issue-630-preset-discovery-reeval.md`: clap-validator を
  release pin ではなく upstream commit `47e2424` の provisioned binary
  pin とする判断を記録。
- `evidence/rf630/preset-discovery-reeval-verification.md`: validator
  `--version`・license・build/provision 手順を追記。
- `docs/impl/exp/issue-630-preset-discovery-reeval-exp.jsonl`: lease 命名に
  合わせて rename し、各 event の `experiment` path を更新。

### Decisions During Implementation

- clap-validator pin: 0.4.1 release binary では preset-discovery
  metadata receiver が deadlock し、fix は upstream master のみ
  （release >0.4.1 未発行）のため、Coordinator 承認の固定 commit
  `47e2424` build を `%LOCALAPPDATA%/ruviform/plugin-abi-validator/
  pin-47e2424/clap-validator.exe` に provision し `CLAP_VALIDATOR` で
  供給。Cargo dependency / commit 済み binary にはしない（ADR に記録）。
- exclusion 撤去: `CLAP_FIXTURE_LIBRARY_EXCLUDES` を削除し fixture
  target を `excludes: []` に変更。`test_fault` の plugin-level
  `param-(set|fuzz)` 除外と informational unfiltered leg
  （Unsupported・verdict 対象外）は維持。
- experiment 命名: lease に合わせ `docs/impl/exp/
  issue-630-preset-discovery-reeval-exp.jsonl` へ rename し、JSONL 各
  event の `experiment` path を `edit_experiment_log.js` で更新。
- docs index: `docs/**/README.md` は変更せず、Coordinator が統合時に
  再生成。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-630-preset-discovery-reeval-exp.jsonl` — 本 slice
  の仮説・provisioning・除外撤去・canonical 実走・summary を seq 1-5 で
  記録。
- `docs/impl/exp/issue-630-preset-discovery-reeval.md` — companion 実験記録。

### Rejected

- なし（upstream commit pin の provisioned binary 方式を採用。0.4.1
  release 継続・release 待ち・Cargo 取り込みは ADR の Considered
  Options で却下）。

## Validation

実施環境: Windows（worktree
`G:/orca/workspaces/ruviform/rf630-preset-discovery`）、Node v26.12.1、
Rust/cargo 1.99.1。canonical = `mise run` 経由。

| コマンド | exit | 結果 |
| --- | --- | --- |
| `node --test ci/plugin-abi-validator/tests/` | 0 | 32 pass / 0 fail |
| `CLAP_VALIDATOR=.../pin-47e2424/clap-validator.exe VST3_VALIDATOR=.../vst3sdk-build/bin/Release/validator.exe mise run test-plugin-abi-validator` | 0 | verdict Passed、canonical 7 leg 全 Passed |
| `mise run fmt-check` | 0 | Pass |
| fixture crate check/lint/test（`ruviform-plugin-test-vst3` / `ruviform-plugin-test-clap`） | 0 | Pass |
| `mise run check-l1-real-plugin` / `lint-l1-real-plugin` / `test-l1-real-plugin` | 0 | Pass |
| `mise run test-rt` | 0 | Pass |
| `mise exec -- prek run --all-files` | 0 | Pass（初回は `vst3-validator.txt` の trailing whitespace/EOF を hook が自動修正 → 再実行で全 Pass） |

preset-discovery crawl/load は raw
`evidence/rf630/abi-validator/clap-*.txt` で success として観測され、
run.json の fixture CLAP leg command に `--exclude preset-discovery-(crawl|load)`
は無い。informational `test_fault` unfiltered leg は Unsupported
（validator exit 101・JSON 無し）で verdict 対象外。

## Risks

- master build pin は署名済み release artifact ではなく、`--version` は
  `clap-validator 0.4.1` のまま。pin 識別は source commit /
  `git describe`（`0.4.1-5-g47e2424`）/ SHA-256
  `b4bd645877bba3d5abe4300a84c5785312ce16f7d1179a2fd3177eedc45afe26` と
  verification 記録に依存する。
- upstream が fix を含む release を発行した場合、pin 更新の要否を別途
  判断する必要がある。
- informational fault leg の tool 側 panic（exit 101）は継続記録であり、
  canonical verdict には含めない。

## Follow-ups

- upstream clap-validator が 0.4.1 超 release を発行したら、固定 commit
  pin の更新を検討する。
- docs index（`docs/**/README.md`）は Coordinator が統合時に再生成する。
- 統合検証は merge 後 main SHA の必須 CI 証拠が正本。
