---
id: "TSPEC-732clapLifecycleAudit06"
type: "test-spec"
status: "approved"
title: "Issue #732 production CLAP adapter lifecycle 監査 — test specification"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
    - "docs/designs/issue-732-clap-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-732-clap-lifecycle-audit.md"
  implements:
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
    - "docs/designs/issue-732-clap-lifecycle-audit.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #732 test specification

## Feature: CLAP lifecycle 契約（production adapter）

本 slice は監査タスクであり、adapter の振る舞い変更が発生した場合のみ
新規 unit test を追加する。監査で欠陥なしと判定された経路は、
既存 test（`plugin.rs` `#[cfg(test)] mod tests`・`adapt.rs` の
`assert_chain`/`map_process_result` test）と外部 validator 実走で
契約適合を検証する。

## Rule: lifecycle 遷移の契約適合（変更時のみ red-green）

- Example: 欠陥修正が発生した場合、違反した CLAP 契約
  （例: 受理 phase・終端化・status 写像）を直接検証する unit test を
  先に失敗させ、修正後に pass させる。
- Example: 欠陥なしの場合、新規 test は追加せず、既存 test の pass と
  監査根拠（evidence/rf732/lifecycle-audit.md）で契約適合を示す。

## Rule: validator lane の fail-closed 契約（run.mjs 変更）

`ci/plugin-abi-validator/tests/run.test.mjs` の contract test で pin:

- Example: `--clap-production` 未指定の run は production leg を
  出力しない（既定 lane 契約不変）。
- Example: `--clap-production` 指定で production dll が無い場合、
  `production_gain` leg は `NotRun`（`--clap-production-dll` を
  示唆）となり verdict は Passed にならない。
- Example: tool/artifact/build 欠落はいずれも NotRun —
  既存 evaluate/normalize の verdict 語彙を変更しない。

## Rule: 外部 ABI 実走（EXT-L3・opt-in）

- Example: clap-validator 0.4.1（provisioned binary）で
  `org.ruviform.clap-gain-poc` を `--clap-production` leg として実走し、
  counts と leg verdict を `evidence/rf732/` に記録する。
- Example: validator 未 provision または異常終了（rf#726 の
  param-fuzz panic 等）の場合、その leg を NotRun/Unsupported として
  正直に記録し Pass としない。

## Rule: L1 real-plugin 非回帰

- Example: `mise run test-l1-real-plugin` で production cdylib を
  host bridge 経由で実走し、lifecycle 変更（発生時）が E2E を
  破壊しないことを確認する。
