---
id: "TASK-7VWD8KuRBS2voBCngFoIJF"
type: "task"
status: "done"
title: "A1 IO MSRV lane 追加・DAG/RT audit 証跡・検証"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/designs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/plans/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/tasks/issue-306-a1-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-msrv-dag-rt-audit.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "evidence/rf306/report.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-306-io-msrv-dag-rt-audit.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-306-io-msrv-dag-rt-audit.md"
    - "docs/tasks/issue-306-f2-verification-handoff.md"
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
    - "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
  references: []
  defers: []
  deferred-by: []
---

# A1 IO MSRV lane 追加・DAG/RT audit 証跡・検証

## Work

<!-- Describe the implementation slice. -->

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-306-io-msrv-dag-rt-audit.md`

## Dependencies

- `docs/tasks/issue-306-f2-verification-handoff.md`（slice 2
  検証完了済み）

## Affected Files

- `mise.toml`・`evidence/rf306/report.md`・
  `crates/ruviform-io-e2e/README.md`・
  `docs/development-environment.md`・本 doc チェーン

## Done When

- [ ] `mise run test-io-msrv` が rust@1.89.0 で exit 0
- [ ] DAG/RT audit 3 判定が `evidence/rf306/report.md` に
      記録済み
- [ ] License 現行状態（仮置き・未決定）が記録済み
- [ ] `mise run fmt-check`/`check`/`lint`/`test` が pass

## Verification

- [ ] `mise run test-io-msrv` 実走
- [ ] `cargo metadata`/`cargo tree` audit 実行
- [ ] workspace lane 非回帰

## 内容

`mise.toml` に `test-io-msrv` task を追加し rust@1.89.0 で
実走する。`cargo metadata`/`cargo tree` で io-e2e leaf・
media-io Domain-Core3 方向・rt 非 io/blocking を機械確認し、
`evidence/rf306/report.md` に slice 3 節として追記する。
License 現行状態をそのまま記録し、NotRun/Unsupported を
正直に列挙する。

## 完了条件

- test-spec R1〜R10 の全判定が実測で満たされる
- 偽装 fixture・公開 API 変更・新規 CI framework を
  追加していない
