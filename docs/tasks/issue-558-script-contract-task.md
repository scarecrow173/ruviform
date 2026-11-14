---
id: "TASK-034XegnqNA9Eq3THKwvpVO"
type: "task"
status: "done"
title: "Issue #558 Script/Analyzer契約 文書チェーン完成・検証"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/558"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-558-script-contract-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-558-script-contract.md"
      - type: "doc"
        path: "docs/impl/exp/issue-558-script-contract.md"
      - type: "doc"
        path: "evidence/rf558/verification.md"
    modified:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
  implements:
    - "docs/plans/issue-558-script-contract-plan.md"
  implemented-by:
    - "docs/impl/ir/issue-558-script-contract.md"
  depends-on:
    - "docs/plans/issue-558-script-contract-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-558-script-contract.md"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/issue-558-script-backend-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-558-script-contract.md"
  references:
    - "evidence/rf558/verification.md"
  defers: []
  deferred-by: []
---

# Issue #558 Script/Analyzer契約 文書チェーン完成・検証

## Work

Issue #558 の DESIGN-ONLY スライスとして作成した文書チェーン（spec/design/ADR/test-spec/plan/task/IR/exp）と `evidence/rf558/` 証拠を完成させ、触れた docs ディレクトリの管理 index を `doc-maintenance` で再生成し、文書監査・`mise run fmt-check`・`prek run --files` を実行して検証を通す。実装コード・crate・CI・依存変更は含まない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-558-script-contract-plan.md`
- `docs/specs/issue-558-script-contract.md`
- `docs/designs/issue-558-script-contract-design.md`
- `docs/adr/issue-558-script-backend-contract.md`
- `docs/test-specs/issue-558-script-contract.md`

## Dependencies

- `docs/plans/issue-558-script-contract-plan.md`（approved）
- `docs/designs/issue-558-script-contract-design.md`（approved — plan-doc design gate 充足）

## Affected Files

- `docs/specs/issue-558-script-contract.md` - 要件・受入条件
- `docs/designs/issue-558-script-contract-design.md` - 契約条項・Fail-Closed 表・Fault Matrix・Fixture
- `docs/adr/issue-558-script-backend-contract.md` - 決定記録
- `docs/test-specs/issue-558-script-contract.md` - Rule/Example
- `docs/plans/issue-558-script-contract-plan.md` - 実装計画
- `docs/tasks/issue-558-script-contract-task.md` - 本タスク
- `docs/impl/ir/issue-558-script-contract.md` - 実装記録
- `docs/impl/exp/issue-558-script-contract.md` - 実験記録
- `evidence/rf558/{contract-trace,consumer-applicability,verification}.md` - 証拠
- `docs/{specs,designs,adr,test-specs,plans,tasks,impl}/README.md` - 管理 index（再生成）

## Done When

- [x] spec/design/ADR/test-spec/plan/task/IR/exp の文書チェーンが front matter・relation を持ち作成済み。
- [x] `evidence/rf558/` に contract-trace・consumer-applicability・verification が存在する。
- [ ] 触れた docs ディレクトリの管理 index が `doc-maintenance index apply` で再生成されている。
- [ ] `audit_docs`/`audit_adr`/`audit_impl_record`/`doc_status` が blocker なしで通過する。
- [ ] `mise run fmt-check` が exit 0。
- [ ] `mise exec -- prek run --files <変更ファイル>` が exit 0。
- [ ] `evidence/rf558/verification.md` に実行コマンド・OS・toolchain・exit code・結果が記録されている。
- [ ] Conventional Commits でコミット・push・PR 作成済み。

## Verification

- [ ] `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir <dir>` → `index apply --dir <dir>`（docs/specs・designs・adr・test-specs・plans・tasks・impl）
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit`（または `audit_docs`）で文書整合を確認
- [ ] `node .agents/skills/adr-doc/scripts/audit_adr.js` で ADR 監査
- [ ] `node .agents/skills/impl-doc/scripts/audit_impl_record.js` で IR 監査
- [ ] `mise run fmt-check`
- [ ] `mise exec -- prek run --files docs/specs/issue-558-script-contract.md docs/designs/issue-558-script-contract-design.md docs/adr/issue-558-script-backend-contract.md docs/test-specs/issue-558-script-contract.md docs/plans/issue-558-script-contract-plan.md docs/tasks/issue-558-script-contract-task.md docs/impl/ir/issue-558-script-contract.md docs/impl/exp/issue-558-script-contract.md evidence/rf558/contract-trace.md evidence/rf558/consumer-applicability.md evidence/rf558/verification.md`
