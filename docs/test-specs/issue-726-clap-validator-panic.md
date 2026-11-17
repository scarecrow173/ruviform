---
id: "TSPEC-726clapValidatorPanic06"
type: "test-spec"
status: "approved"
title: "Issue #726 clap-validator param-fuzz panic — test specification"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-726-clap-validator-panic.md"
    - "docs/designs/issue-726-clap-validator-panic.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-726-clap-validator-panic.md"
  implements:
    - "docs/specs/issue-726-clap-validator-panic.md"
    - "docs/designs/issue-726-clap-validator-panic.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-726-clap-validator-panic.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #726 test specification

## Feature: ABI-validator lane の fail-closed 契約（外部 validator 異常終了）

本 slice は調査・判断系であり、runner のコード変更は契約違反が
確認された場合のみ発生する。検証の主眼は「外部 validator が JSON を
出さず異常終了した場合に、leg verdict が正直に記録され run.json が
必ず出力される」契約の維持確認である。

## Rule: validator 異常終了の fail-closed 記録

- Example: clap-validator が非 0 exit かつ JSON 未出力で終了した場合、
  `normalizeClapLeg` は leg を `Unsupported`（tool-side termination）
  として記録し、`Failed`/`Passed` にしない —
  `run.test.mjs` の "nonzero exit without JSON is Unsupported" が pin。
- Example: canonical leg が `Unsupported` の場合、`evaluate` は
  verdict を `Unsupported`・exit 1 とし Pass にしない —
  "Unsupported canonical leg fails closed too" が pin。
- Example: informational leg（test_fault unfiltered）は verdict に
  介入せず、`Unsupported` のまま evidence に残る —
  "informational legs never affect the verdict" が pin。

## Rule: panic 再現の証拠保存

- Example: `clap-validator validate --json --plugin-id
  org.ruviform-test.clap-test-fault --include param-fuzz-basic
  target/debug/ruviform_plugin_test_clap.dll` は exit 101・stdout
  空・stderr に `sandbox.rs:98` deserialize panic と `cli.rs:55`
  scoped thread panic を記録する — `evidence/rf726/filter-runs/` が pin。
- Example: 同一 filter を test_gain に適用した対照 run は success で
  完走する — fixture 固有の interaction であることの pin。

## Rule: runner contract test の非回帰

- Example: `node --test ci/plugin-abi-validator/tests/` が全件 pass
  すること。

## Rule: spawnCapture の stdio 捕捉契約（runner 修正に伴う追加）

lane 実走で pipe stdio の ETIMEDOUT 誤記録を実測確認したため、
`spawnCapture`（file-backed stdio）を `runClapLeg` に適用した。
以下を contract test が pin する:

- Example: validator process が stdio を継承する grandchild を残して
  終了しても、`spawnCapture` は child exit で復帰し stdout/stderr を
  回収する — "spawnCapture returns at child exit despite
  stdio-inheriting grandchildren" が pin（pipe 実装では grandchild が
  pipe EOF を阻害し timeout する退行を検出）。
- Example: 存在しない executable・hang する child は引き続き
  `proc.error`（ENOENT/ETIMEDOUT）を返し fail-closed —
  "spawnCapture still reports spawn failures and timeouts
  fail-closed" が pin。
- Example: verdict ロジック（`normalizeClapLeg`/`evaluate`）は無変更
  のため既存 pin がそのまま有効 — leg 定義・exclude 条件・
  informational leg の扱いは不変。

## Non-goals

- clap-validator 自体の panic 修正の検証（upstream の責務）。
- fixture の fault param が fatal mode を踏むこと自体の検証
  （rf#630 の L1 fault lane が担う）。
- preset-discovery timeout・param-set-* failed の解消検証
  （0.4.1 既知事象・upstream release 待ち）。

## Risk

- runner contract test が panic signature を pin していなければ、
  将来の run.mjs 変更で「validator 異常終了を Passed/Failed に
  誤分類」する退行を検出できない — 現行 test :161-172 がこれを
  防いでいることを本 spec で明示する。
