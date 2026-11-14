---
id: "PLAN-034XefvOx2KUgJEbc2rTtz"
type: "plan"
status: "completed"
title: "Issue #558 Script/Analyzer契約 文書スライス実装計画"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/558"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-558-script-contract-plan.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-558-script-contract.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-558-script-contract-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-558-script-contract.md"
    - "docs/designs/issue-558-script-contract-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-558-script-contract.md"
  references:
    - "docs/adr/issue-558-script-backend-contract.md"
    - "evidence/rf558/contract-trace.md"
    - "evidence/rf558/consumer-applicability.md"
    - "evidence/rf558/verification.md"
  defers: []
  deferred-by: []
---

# Issue #558 Script/Analyzer契約 文書スライス実装計画 Implementation Plan

## Goal

Issue #558 の DESIGN-ONLY スライスとして、言語非依存 Script/Analyzer 契約の文書チェーン（spec/design/ADR/test-spec/plan/task/IR/exp）と `evidence/rf558/` 証拠を完成させ、管理 index を再生成して検証を通す。

## Scope

**1 実装ストリーム**: 文書のみ。`crates/**`・`Cargo.toml`/`Cargo.lock`/`mise.toml`/CI・Consumer コードは対象外。実装 crate・公開 API・PoC コードは生成しない（#559/#560 + #553 判定後）。

## Upstream Documents

- `docs/specs/issue-558-script-contract.md`
- `docs/adr/issue-558-script-backend-contract.md`

## Design Inputs

- `docs/designs/issue-558-script-contract-design.md`（status: approved）

## File Map

- `docs/specs/issue-558-script-contract.md` - 要件・受入条件（SPEC-034Xe8fKEL6vpFX9pkgPU0）
- `docs/designs/issue-558-script-contract-design.md` - 契約条項・Fail-Closed 表・Fault Matrix・Fixture（DESIGN-034XeaFjxZePL0m6LYuEeS）
- `docs/adr/issue-558-script-backend-contract.md` - 決定記録 Option C（ADR-034Xe9xZEqz5ftcu5yCqXB）
- `docs/test-specs/issue-558-script-contract.md` - Rule/Example 検証仕様（TSPEC-034XecRP3Wpuq0BscTS283）
- `docs/plans/issue-558-script-contract-plan.md` - 本計画
- `docs/tasks/issue-558-script-contract-task.md` - 実装スライス
- `docs/impl/ir/issue-558-script-contract.md` - 実装記録
- `docs/impl/exp/issue-558-script-contract.md` - 実験記録
- `evidence/rf558/contract-trace.md` - 受入条件→契約条項→根拠対応表
- `evidence/rf558/consumer-applicability.md` - 2 Consumer Reuse/New/Deferred 判定ログ
- `evidence/rf558/verification.md` - 検証記録
- `docs/{specs,designs,adr,test-specs,plans,tasks,impl}/README.md` - 管理 index（`doc-maintenance` で再生成）

## Implementation Sequence

- [x] spec 作成（要件・受入条件・Deferred）
- [x] ADR 作成（Option C 採用の決定記録）
- [x] design 作成（公開型・Fail-Closed 表・Fault Matrix・Fixture・2 Consumer サマリ）
- [x] test-spec 作成（Rule/Example を契約条項へ写像）
- [x] evidence 作成（contract-trace・consumer-applicability・verification）
- [ ] plan/task/IR/exp 作成（本ファイル含む文書チェーン完成）
- [ ] `review-protocol` レビュー実施・blocker 解消
- [ ] 管理 index 再生成（`doc-maintenance index apply`）
- [ ] 文書監査（`audit_docs`/`audit_adr`/`audit_impl_record`/`doc_status`）
- [ ] `mise run fmt-check` + `prek run --files` 実行
- [ ] コミット・push・PR 作成

## Dependencies and Risks

- **依存**: design が approved であること（plan-doc design gate 充足）。本スライスは文書のみで外部依存なし。
- **リスク**: 契約の網羅性不足 → review-protocol で spec/design/ADR/test-spec をレビューし blocker を解消。未実測項目の Pass 偽装 → `evidence/rf558/verification.md` に実コマンド・結果を記録し未実行は NotRun/Deferred。
- **リスク**: 管理 index の手編集禁止 → `doc-maintenance` の preview-first（`index plan`→`index apply`）に従う。

## Task Breakdown

- `docs/tasks/issue-558-script-contract-task.md` - 文書チェーン完成・index 再生成・監査・検証・コミットの単一スライス（文書のみのため 1 タスクに集約）。

## Verification

- [ ] `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir docs/specs` 他、触れた全 dir で preview → `index apply` で再生成。
- [ ] `audit_docs`（spec/design/test-spec）・`audit_adr`・`audit_impl_record`・`doc_status` で front matter・relation・index 整合を確認。
- [ ] `mise run fmt-check`（Markdown/構成ファイルの軽量チェック）exit 0。
- [ ] `mise exec -- prek run --files <変更ファイル>` で Markdown/リンクチェック exit 0。
- [ ] `evidence/rf558/verification.md` に実行コマンド・OS・toolchain・exit code・結果を記録。

## Open Questions / Gaps

- 実装 crate 配置・公開 API の Rust 確定 → Deferred（#559/#560 PoC + #553 判定後）。本スライスでは契約レベル定義のみ。
- `Send`/`Sync` 可能 Session の提供形態 → Deferred（#559 PoC で実測後）。
- 共通 API の製品昇格・Rhai 本番採用 → Deferred（実証後判断）。
- AudioLens への Script 適用 → Deferred（Ruviform 昇格で AL 依存木に影響が出た時点）。

## Self-Review

- [x] spec の全受入条件が design/test-spec/evidence へ写像されている（`evidence/rf558/contract-trace.md`）。
- [x] プレースホルダなし。ファイルパス・relation・status 値・用語が一貫。
- [x] 実装コード・crate・CI・依存変更を含まない（DESIGN-ONLY）。

## Implementation Handoff

- Recommended mode: inline execution（文書のみ。Worker が直接作成）。
- Independent work streams: none（単一文書スライス）。
- User approval required before dispatch: review-protocol のレビューで blocker なしを承認として扱う（Worker 共通契約）。

## Review Handoff

- Reviewer は spec の受入条件と design の契約条項・Fail-Closed 表・Fault Matrix・Fixture の対応を確認。
- `evidence/rf558/consumer-applicability.md` の Reuse/New/Deferred 判定が #553 の非重複表材料として妥当か確認。
- 未実測項目が NotRun/Deferred と明記され Pass に偽装されていないことを確認。
