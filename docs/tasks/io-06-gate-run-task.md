---
id: "TASK-HFfUybb3K6gZXTbsmRlDGW"
type: "task"
status: "done"
title: "IO-06 Gate matrix 実走と証拠記録（Issue #288 slice）"
created: "2026-11-11"
updated: "2026-11-12"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/io-06-gate-run.md"
      - type: "doc"
        path: "docs/designs/io-06-gate-run-design.md"
      - type: "doc"
        path: "docs/plans/io-06-gate-run-plan.md"
      - type: "doc"
        path: "docs/tasks/io-06-gate-run-task.md"
      - type: "doc"
        path: "docs/test-specs/io-06-gate-run.md"
      - type: "doc"
        path: "docs/impl/ir/io-06-gate-run-ir.md"
      - type: "file"
        path: "evidence/rf288/io06-gate-matrix.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/io-06-gate-run-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/io-06-gate-run-plan.md"
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
    - "docs/test-specs/io-06-gate-run.md"
  references: []
  defers: []
  deferred-by: []
---

# IO-06 Gate matrix 実走と証拠記録（Issue #288 slice）

## Work

Issue #288 の受入 matrix（IO-01〜05 + 合成 E2E、正常系・否定系
× Capability/OS/FS）を Windows 実機で実走可能な mise lane の
実測と既存証拠（`evidence/rf306/`・ci.yml・consumer-verify.yml
の run 記録）へ対応付け、`evidence/rf288/io06-gate-matrix.md`
へセルごとの判定を記録する。コード変更なし。macOS/Linux 実機・
Network/Cloud FS・実 4GiB・consumer 固有受入・`Indeterminate`
系 fixture 化は対象外（NotRun/Unsupported として記録）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [plan](../plans/io-06-gate-run-plan.md)
- [spec](../specs/io-06-gate-run.md)
- [design](../designs/io-06-gate-run-design.md)
- [test-spec](../test-specs/io-06-gate-run.md)
- Gate 判定正本: [issue-288 design](../designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)

## Dependencies

- `mise.toml` の io 系 task・workspace task が main に実在する
  こと（作成時に確認済み）。
- `evidence/rf306/` の 3 レポートが main に存在すること。
- `rust@1.89.0` toolchain が `mise exec` で利用可能なこと
  （`rustc --version` で確認済み）。

## Affected Files

- `docs/specs/io-06-gate-run.md`（新規）
- `docs/designs/io-06-gate-run-design.md`（新規）
- `docs/plans/io-06-gate-run-plan.md`（新規）
- `docs/tasks/io-06-gate-run-task.md`（新規）
- `docs/test-specs/io-06-gate-run.md`（新規）
- `docs/impl/ir/io-06-gate-run-ir.md`（新規）
- `evidence/rf288/io06-gate-matrix.md`（新規）

## Done When

- [x] matrix 全セルに判定と根拠がある
      （`evidence/rf288/io06-gate-matrix.md`）。
- [x] Windows 実走 lane の exit code・OS・toolchain が記録されて
      いる（全 18 lane。check/lint は flake 診断の上で再実走
      exit 0 を確定値として記録）。
- [ ] 実走結果と残存 NotRun が Issue #288 comment と PR 本文に
      記録されている。
- [x] matrix の穴が列挙されている（7 件）。
- [x] `mise run fmt-check` + doc-status 系 lint で本 PR 追加分に
      新規 blocking finding がない。

## Verification

- [x] `mise run fmt-check` pass。
- [x] doc-status 系 lint/audit で本 PR 追加分に broken relation・
      未解決参照がないこと（既存の範囲外 finding は分離して記録）。
- [x] matrix の参照先（lane 名・Issue/PR/run ID/evidence パス）が
      実在することを確認済み。
- [x] `git status`/`git diff --stat` で変更が許可範囲のみである
      ことを確認。
