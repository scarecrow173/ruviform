---
id: "SPEC-630presetDiscoveryReeval02"
type: "spec"
status: "approved"
title: "Issue #630 preset-discovery 外部検証再評価 slice — spec"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-preset-discovery-reeval.md"
    - "docs/specs/issue-630-abi-validator-rerun.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-630-preset-discovery-reeval.md"
  implements:
    - "docs/discovery/issue-630-preset-discovery-reeval.md"
  implemented-by:
    - "docs/designs/issue-630-preset-discovery-reeval.md"
  depends-on:
    - "docs/specs/issue-630-abi-validator-rerun.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 preset-discovery 外部検証再評価 slice — spec

## 目的

upstream fix（PR #41 / commit `47e2424`）を含む clap-validator で
CLAP fixture の preset-discovery crawl/load を実走し、前 slice の
宣言的除外を解除した証拠を `evidence/rf630/abi-validator/` に残す。

## 要求

1. `CLAP_FIXTURE_LIBRARY_EXCLUDES` を削除し、fixture CLAP target の
   `excludes` を空にする。`preset-discovery-(crawl|load)` を
   `--exclude` しない。
2. clap-validator は upstream commit `47e2424` 由来 build を使い、
   path・version・SHA-256・source commit を記録する。0.4.1 release
   binary を再び使った場合は preset-discovery が tool-side failure
   として顕在化することを許容する。
3. canonical lane `mise run test-plugin-abi-validator` を実走し、
   verdict・leg 別 status/counts・tool/artifact SHA-256・raw output
   を run.json / txt に記録する。
4. preset-discovery crawl/load/descriptor-consistency が raw output
   で実行され、少なくとも crawl/load が failed/crashed でないことを
   確認する。除外が残っていれば Failed/NotRun として扱う。
5. `crates/`・`Cargo.lock`・`mise.toml`・`.github/**`・docs index は
   変更しない。fail-closed verdict と informational leg の扱いは維持。

## 受入条件

- [ ] `node --test ci/plugin-abi-validator/tests/` が全 pass。
- [ ] canonical lane が exit 0 で verdict Passed、または preset-
      discovery 以外の理由で Failed/NotRun なら証拠つきで報告。
- [ ] run.json の全 fixture CLAP leg command に
      `--exclude preset-discovery` が無く、preset-discovery case の
      成功が raw txt で確認できる。
- [ ] doc chain（slug `issue-630-preset-discovery-reeval`）と検証
      md を作成し、docs index は触らない。
