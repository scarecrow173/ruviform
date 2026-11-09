---
id: "TASK-034UtEdJ4J9ekRBHmpucta"
type: "task"
status: "done"
title: "F1 IO E2E fixture 実装 — Fault/Fuzz・Directory・OS/FS probe"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
  references: []
  defers: []
  deferred-by: []
---

# F1 IO E2E fixture 実装 — Fault/Fuzz・Directory・OS/FS probe

## Work

<!-- Describe the implementation slice. -->

## Classification

- `normal-plan-task`

## Upstream Documents

- <!-- plan, spec, ADR, or task path -->

## Dependencies

- <!-- prerequisite, blocker, or related task -->

## Affected Files

- <!-- file, directory, module, or interface -->

## Done When

- [ ] <!-- completion criterion -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->

## 内容

plan `docs/plans/issue-306-io-e2e-fault-fuzz.md` の File Map に
従い、`ruviform-io-e2e` の tests 層に Fault/Fuzz・Directory 境界・
OS/FS probe の fixture を実装する。公開 API のみを消費し、
`vfs`・`#[cfg(test)]` mock・production 変更は行わない。

## 完了条件

- 5 つの新規 test file と common helper 拡張が `cargo test -p
  ruviform-io-e2e` で pass（link skip は許容・記録必須）
- 否定系が typed 分類 + 非残存/recovery handle を assert
