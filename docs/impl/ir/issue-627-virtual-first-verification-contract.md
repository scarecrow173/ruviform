---
id: "IMPL-034XAVfCRzURaeZTUI61HZ"
type: "impl"
status: "draft"
title: "Issue #627 仮想優先3repo検証契約 implementation record"
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
        path: "docs/impl/ir/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/specs/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/designs/issue-627-virtual-first-verification-contract-design.md"
      - type: "doc"
        path: "docs/designs/issue-627-policies-revision-proposal.md"
      - type: "doc"
        path: "docs/test-specs/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "evidence/rf627/policies-baseline-comparison.md"
      - type: "doc"
        path: "evidence/rf627/verification.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/issue-627-virtual-first-verification-contract.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-627-virtual-first-verification-contract.md"
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-627-virtual-first-verification-contract.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
  verified-by: []
  references:
    - "evidence/rf627/policies-baseline-comparison.md"
    - "evidence/rf627/verification.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #627 仮想優先3repo検証契約 implementation record

## Summary

Issue #627 の docs-only slice を実施した。本番コード・CI・`POLICIES.md` 本文の変更は一切なく、検証契約の文書チェーン（spec → design×2 → test-spec → plan → task → IR）と証拠（`evidence/rf627/`）のみを作成した。三 repo の `POLICIES.md` 仮想優先改訂は作業中に全 repo で main merge 済みと判明したため、deliverable を「改訂案」から「着地済み改訂の照合記録 + 残件整理」に転換した。

## Context

- Task: `docs/tasks/issue-627-virtual-first-verification-contract.md`（TASK-034XAUE4Ki5ftJQfWI15b1）
- Plan: `docs/plans/issue-627-virtual-first-verification-contract-plan.md`（PLAN-034XATq1VmM49W1cv5EO0P）
- Spec: `docs/specs/issue-627-virtual-first-verification-contract.md`（SPEC-034XAreKCCKojmpwC7OvJR）
- 上位契約: `docs/specs/issue-219-consumer-compat-contract.md`、`docs/specs/issue-294-cross-consumer-candidate-gate.md`
- 前提の更新: 三 repo の `POLICIES.md` 改訂が既に着地（RF PR #634 `3b8b7322`、SV PR #449 `8aa5cb0`、AL PR #212 `eb62440`）。Coordinator の orchestration 通知で確認。

## Implementation

### Changes

- `docs/specs/issue-627-virtual-first-verification-contract.md` — Problem/Why Now/Scope/Requirements/AC を「着地済み改訂の照合 + 契約確定」の責務に更新。status: approved。
- `docs/designs/issue-627-virtual-first-verification-contract-design.md` — 検証契約の設計。`environment_kind` taxonomy（`deterministic-backend`/`virtual-os-endpoint`/`test-plugin`/`real-window`/`physical-device`）、直交 Case schema（必須14項目）、3 境界 Case（本番OS-API通過/実ABIバイナリ/実画素Native Dock）、独立 Oracle 規則と repo 別所有権、三 repo 受入マトリクス、Dogfooding/adapter 境界、明示ルール5項。status: approved。
- `docs/designs/issue-627-policies-revision-proposal.md` — 着地済み改訂の post-landing 照合記録。baseline SHA 表、変更/非変更箇所の行単位対応、repo 間残差分（audio-lens `\` 行末スタイル等）、契約適合判定、ゲート移管台帳（RF #251/#253/#388/#389/#532/#294、SV #23/#114/#115/#117/#126/#127/#128/#176、AL #127）、旧物理 Required の扱い4ルール。status: approved。
- `docs/test-specs/issue-627-virtual-first-verification-contract.md` — Feature/Rule/Example 構造で契約を検証可能化（R1〜R13、E1〜E8）。status: approved。
- `docs/plans/issue-627-virtual-first-verification-contract-plan.md` — T1〜T10 の実施計画と検証 matrix。status: approved。
- `docs/tasks/issue-627-virtual-first-verification-contract.md` — タスク追跡。status: in-progress → 完了時 done。
- `evidence/rf627/policies-baseline-comparison.md` — 三 repo baseline SHA・取得コマンド・行単位 diff・非変更箇所・残差分・照合限界。
- `evidence/rf627/verification.md` — 本 slice の検証 ledger（graph router・doc_status・review-protocol・prek・git scope）。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md` — `doc_maintenance.js index apply` による再生成。

### Decisions During Implementation

- **deliverable の転換**: Issue 起票時は「改訂案の確定」だったが、三 repo の改訂が既に merge 済みと判明したため、「post-landing 照合記録 + 残件整理」に転換した（Coordinator 通知と整合）。`POLICIES.md` 本文は引き続き非変更。
- **`environment_kind` の値集合**: ポリシー本文の開いた列挙（「等」付き和英混在）に対し、契約では `deterministic-backend`/`virtual-os-endpoint`/`test-plugin`/`real-window`/`physical-device` の kebab-case 閉集合を canonical として定義。意味論は一致。
- **review-protocol を承認の代替として使用**: Worker 共通契約に従い、design gate（approved design 必須）を review-protocol の blocker なしで充足させた。
- **doc-driven-dev-graph router の blocked 結果は repo 横断の既存状態**として記録し、本 slice の文書内容の問題とは区別した。

## Related Experiments

### Adopted

<!-- なし（docs-only slice、実験ログなし） -->

### Rejected

<!-- なし -->

## Validation

- `git status`/`git diff`: 変更が許可範囲（`docs/{specs,designs,plans,tasks,test-specs,impl/ir}/issue-627-*`、`evidence/rf627/`、生成 index）に限定されることを確認。
- `doc_status.js lint`: 新規文書の broken-relation/index 指摘は残ファイル作成と index 再生成で解消。repo 横断の既存 finding（106 blocking、主に他 spec の traceability-missing-upstream）は本 slice 範囲外として記録。
- review-protocol: Reviewer A/B の独立パスで指摘を検証・修正（spec の陳腐化した「承認前」表現、`environment_kind` の適用範囲表現、`resolved_ruviform_sha` の Ruviform 自身の場合の扱い）。blocker なし → spec/design×2/test-spec/plan を approved に遷移。
- `mise exec -- prek run --all-files`: 実行結果は `evidence/rf627/verification.md` に記録。
- doc-driven-dev-graph router (`route_graph.js`): `status: blocked`（repo 全体の既存 broken relation・focus-required）。本 slice 範囲外の既存状態として記録。

## Risks

- 三 repo の `POLICIES.md` が今後改訂された場合、本照合記録の baseline が陳腐化する → baseline SHA を証拠に固定済み。
- audio-lens の行末 `\` スタイル等の既存差異は残件として記録したが修正は POLICIES.md 非変更制約で本 slice 対象外 → 別 Issue 候補。
- 旧物理 Required Case の実際の状態遷移は各 repo のレビュー待ち（本 slice は移管台帳の確定まで）。
- #628〜#631 実装で `environment_kind` 追加や schema 変更が必要になった場合は契約改訂として扱う。

## Follow-ups

- 三 repo での本記録のレビューと、旧物理 Required Case の状態遷移（各 repo の Issue/PR）。
- #628 Test Runtime・#629 仮想 OS Endpoint・#630 Test Plugin・#631 GPUI Scenario Runner の実装（本契約を入力として開始可能）。
- audio-lens `\` 行末スタイル統一の別 Issue 起票検討。
- `physical-device` opt-in Case の個別定義（製品の正式サポート宣言時）。
