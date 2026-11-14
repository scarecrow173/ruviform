---
id: "TASK-034XAUE4Ki5ftJQfWI15b1"
type: "task"
status: "done"
title: "Issue #627 T1 — 仮想優先3repo検証契約・POLICIES照合・移管台帳の文書化"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-627-virtual-first-verification-contract.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
  verified-by: []
  references:
    - "evidence/rf627/policies-baseline-comparison.md"
    - "evidence/rf627/verification.md"
  defers: []
  deferred-by: []
---

# Issue #627 T1 — 仮想優先3repo検証契約・POLICIES照合・移管台帳の文書化

## Work

Issue #627 の docs-only slice。三 repo の `POLICIES.md`（仮想優先改訂適用後）を行単位で照合し、検証契約（`environment_kind` taxonomy・直交 Case schema・3 境界 Case・独立 Oracle・三 repo 受入マトリクス・Dogfooding 責務）を設計文書として確定し、旧物理 Required Issue のゲート移管台帳と「仮想 Pass ≠ 物理 Pass」ルールを記録する。`POLICIES.md` 本文・本番コード・CI 設定は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-627-virtual-first-verification-contract-plan.md`
- `docs/specs/issue-627-virtual-first-verification-contract.md`
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
- `docs/designs/issue-627-policies-revision-proposal.md`
- `docs/test-specs/issue-627-virtual-first-verification-contract.md`

## Dependencies

- 三 repo `POLICIES.md` baseline: RF `3b8b7322` / SV `8aa5cb0` / AL `eb62440`（読み取りのみ）
- plan T1〜T6（baseline 比較・spec・設計・照合記録・test-spec・レビュー）は本タスク内で完了済み

## Affected Files

- `docs/specs/issue-627-virtual-first-verification-contract.md`
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
- `docs/designs/issue-627-policies-revision-proposal.md`
- `docs/test-specs/issue-627-virtual-first-verification-contract.md`
- `docs/plans/issue-627-virtual-first-verification-contract-plan.md`
- `docs/tasks/issue-627-virtual-first-verification-contract.md`
- `docs/impl/ir/issue-627-virtual-first-verification-contract.md`
- `evidence/rf627/policies-baseline-comparison.md`
- `evidence/rf627/verification.md`
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md`（`doc_maintenance.js index apply` による再生成のみ）

## Done When

- [x] spec/design×2/test-spec/plan/task/IR の doc-chain が front matter relations 整合で存在する
- [x] `evidence/rf627/policies-baseline-comparison.md` に三 repo baseline SHA と行単位 diff が記録されている
- [x] `evidence/rf627/verification.md` に検証・レビュー・衛生チェックの ledger が記録されている
- [x] review-protocol で blocker が残っていない
- [x] `POLICIES.md`・範囲外ファイルに差分がない
- [x] `mise exec -- prek run --all-files` が Pass
- [x] 触れた dir の index が再生成され整合している

## Verification

- [x] `git status` で変更範囲が許可パスに限定されること
- [x] `doc_maintenance.js index apply --dir <touched dirs>` + `doc_status.js lint` で新規 blocking finding がないこと
- [x] `mise exec -- prek run --all-files` exit 0
- [x] review-protocol verdict: ACCEPT（blocker なし）
