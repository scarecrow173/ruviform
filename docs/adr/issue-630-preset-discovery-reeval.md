---
id: "ADR-SAljZ24QzAaSgusRPiWYAy"
type: "adr"
status: "accepted"
title: "Issue #630 clap-validator preset-discovery 検証を upstream commit build pin で実施"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-preset-discovery-reeval.md"
    - "docs/specs/issue-630-preset-discovery-reeval.md"
    - "docs/designs/issue-630-preset-discovery-reeval.md"
  changes:
    added:
      - type: "file"
        path: "docs/adr/issue-630-preset-discovery-reeval.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-630-abi-validator.md"
    - "evidence/rf630/preset-discovery-reeval-verification.md"
    - "docs/impl/ir/issue-630-preset-discovery-reeval.md"
  refines:
    - "docs/designs/issue-630-abi-validator.md"
  refined-by: []
  derives-from:
    - "docs/discovery/issue-630-preset-discovery-reeval.md"
    - "docs/specs/issue-630-preset-discovery-reeval.md"
    - "docs/designs/issue-630-preset-discovery-reeval.md"
  derived-by: []
  verifies: []
  verified-by:
    - "evidence/rf630/preset-discovery-reeval-verification.md"
  references:
    - "https://github.com/free-audio/clap-validator/pull/41"
    - "https://github.com/free-audio/clap-validator/commit/47e242404df89f3c8454aaf1c02bb0caca6db2ce"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Issue #630 clap-validator preset-discovery 検証を upstream commit build pin で実施

## Context and Problem Statement

Issue #630 の外部 ABI 検証では、CLAP fixture の preset-discovery
crawl/load を実走する必要がある。既存の
`docs/designs/issue-630-abi-validator.md` は provisioning として
`clap-validator 0.4.1` の pinned release binary を記述しているが、
この release binary では preset discovery metadata receiver が
deadlock し、`preset-discovery-(crawl|load)` を完走できない。

upstream `free-audio/clap-validator` では PR #41（merge commit
`47e242404df89f3c8454aaf1c02bb0caca6db2ce`、fix commit
`91ee84082fdb4aeb1d8cdc96a285a886333ca895`）が master に取り込まれたが、
決定時点で 0.4.1 より新しい release は発行されていない。したがって、
release binary の pin だけでは preset-discovery 再評価の要件を満たせない。

## Decision Drivers

- preset-discovery crawl/load を `--exclude` せず実走し、失敗・crash が
  無いことを外部 validator の生出力で確認できること。
- clap-validator は workspace の Cargo 依存や commit 済み binary ではなく、
  既存の外部 tool provisioning 経路で供給すること。
- binary の由来（upstream repo、commit、build 手順、path、SHA-256、
  license）を監査可能な形で固定すること。
- runner の fail-closed な tool 解決と verdict 集約を維持し、
  informational leg の扱いを変えないこと。

## Considered Options

- 0.4.1 release binary を継続し、preset-discovery exclusion を維持する
  — 必須 case が実行されず、Issue #630 の外部検証要件を満たさない。
- upstream release `>0.4.1` の発行を待つ — 決定時点で未発行であり、
  本 slice の証拠取得を不確定に遅らせる。
- clap-validator を Cargo dependency または repo 内 build 対象として
  取り込む — 外部 validator を Cargo 管理外の provisioned binary とする
  既存設計から外れ、toolchain/依存の再現性責務が repo 側へ移る。
- upstream master の固定 commit `47e2424` を build し、provisioned
  binary として pin する — fix を含み、供給経路を既存の外部 tool
  解決に留められる。

## Decision Outcome

Coordinator 承認のもと、CLAP validator は upstream commit
`47e242404df89f3c8454aaf1c02bb0caca6db2ce`（PR #41 merge、post-0.4.1
master）から build した binary を固定 pin として使う。binary は
Cargo 依存ではなく、`%LOCALAPPDATA%/ruviform/plugin-abi-validator/
pin-47e2424/clap-validator.exe` に provision し、runner には
`--clap-validator` / `CLAP_VALIDATOR`（または `--validator-dir` を
pin directory に向ける規約）で明示供給する。

この決定は `docs/designs/issue-630-abi-validator.md` の
「`clap-validator 0.4.1` pinned release binary」という記述を
supersede するものではなく、本 slice の preset-discovery 再評価に限り
release pin を commit-pinned master build へ読み替える refinement である。
`--version` 出力は `clap-validator 0.4.1` のままだが、source checkout の
`git describe --tags --always --dirty` は `0.4.1-5-g47e2424`、binary の
SHA-256 は `b4bd645877bba3d5abe4300a84c5785312ce16f7d1179a2fd3177eedc45afe26`
であり、license は MIT（upstream `Cargo.toml` の `license = "MIT"` と
`LICENSE`）である。binary 自体は commit しない。

## Consequences

- Good: preset-discovery crawl/load が unfiltered で実行され、canonical
  lane の verdict を Passed として記録できる。tool provenance が
  commit/build/path/SHA-256/license で固定され、0.4.1 release pin との
  混同を避けられる。Cargo.lock・workspace 依存・docs index を変更せずに
  済む。
- Bad: master build pin は署名済み release artifact ではないため、
  `--version` だけでは 0.4.1 release binary と区別できない。再現には
  upstream checkout/build/provision 手順と SHA-256 照合が必要になる。
  upstream が fix を含む release を発行した場合は、pin の更新を別途
  判断する。

## Implementation Plan

- Affected paths: `ci/plugin-abi-validator/run.mjs`、
  `ci/plugin-abi-validator/tests/run.test.mjs`、
  `evidence/rf630/abi-validator/`、
  `evidence/rf630/preset-discovery-reeval-verification.md`、
  `docs/impl/exp/issue-630-preset-discovery-reeval-exp.jsonl`、
  `docs/impl/ir/issue-630-preset-discovery-reeval.md`、および本 ADR を含む
  Issue #630 doc chain。
- Constraints: `crates/`・`Cargo.lock`・`mise.toml`・`.github/**`・
  docs index（`docs/**/README.md`）は変更しない。validator binary を
  commit しない。`test_fault` の plugin-level `param-(set|fuzz)` 除外と
  informational unfiltered leg の Unsupported 記録は維持する。
- Patterns to follow: 既存の外部 validator 解決（明示 flag → env →
  validator-dir 規約）と fail-closed 診断、run.json/evidence への
  tool/artifact SHA-256 記録、実施した検証のみを Passed とする verdict。
- Verification: `node --test ci/plugin-abi-validator/tests/`、
  `CLAP_VALIDATOR=<pin>/clap-validator.exe VST3_VALIDATOR=<vst3 validator>
  mise run test-plugin-abi-validator`、および
  `evidence/rf630/preset-discovery-reeval-verification.md` の
  provenance 記録で確認する。

## More Information

- Issue: <https://github.com/scarecrow173/ruviform/issues/630>
- Upstream fix: <https://github.com/free-audio/clap-validator/pull/41>
- Pinned commit: <https://github.com/free-audio/clap-validator/commit/47e242404df89f3c8454aaf1c02bb0caca6db2ce>
- Spec/design: `docs/specs/issue-630-preset-discovery-reeval.md`、
  `docs/designs/issue-630-preset-discovery-reeval.md`
- Provisioned binary: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/
  pin-47e2424/clap-validator.exe`（SHA-256
  `b4bd645877bba3d5abe4300a84c5785312ce16f7d1179a2fd3177eedc45afe26`）
