---
id: "PLAN-034XATq1VmM49W1cv5EO0P"
type: "plan"
status: "approved"
title: "Issue #627 仮想優先検証契約 実装・ロールアウト計画"
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
        path: "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/impl/ir/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/specs/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/designs/issue-627-virtual-first-verification-contract-design.md"
      - type: "doc"
        path: "docs/designs/issue-627-policies-revision-proposal.md"
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
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  implemented-by:
    - "docs/tasks/issue-627-virtual-first-verification-contract.md"
  depends-on:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
  derived-by:
    - "docs/tasks/issue-627-virtual-first-verification-contract.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
  references:
    - "evidence/rf627/policies-baseline-comparison.md"
    - "evidence/rf627/verification.md"
  defers: []
  deferred-by: []
---

# Issue #627 仮想優先検証契約 実装・ロールアウト計画

## Goal

Issue #627 の docs-only slice として、仮想優先3repo検証契約（Case schema・境界 Case・独立 Oracle・受入マトリクス）・着地済み POLICIES 改訂の照合記録・ゲート移管台帳・証拠を確定し、#628〜#631 が従う契約をレビュー可能な状態で納品する。

## Scope

単一ストリーム。docs のみの変更であり、Rust コード・CI・`POLICIES.md` 本文の変更を含まない。三 repo の `POLICIES.md` 改訂は既に着地済み（RF `3b8b7322` / SV `8aa5cb0` / AL `eb62440`）のため、本 plan の「実装」は文書チェーンと証拠の確定である。三 repo でのレビュー・承認と旧物理 Required の状態遷移は本 slice の外側の外部 gate。

## Upstream Documents

- `docs/specs/issue-627-virtual-first-verification-contract.md`（SPEC-034XAreKCCKojmpwC7OvJR）
- `docs/specs/issue-219-consumer-compat-contract.md`、`docs/specs/issue-294-cross-consumer-candidate-gate.md`（上位契約）
- Issue #627: https://github.com/scarecrow173/ruviform/issues/627

## Design Inputs

- `docs/designs/issue-627-virtual-first-verification-contract-design.md`（契約: Gate taxonomy・Case schema・境界 Case・Oracle・マトリクス・Dogfooding）
- `docs/designs/issue-627-policies-revision-proposal.md`（着地済み改訂の照合記録・残差分・ゲート移管台帳）

## File Map

- `docs/specs/issue-627-virtual-first-verification-contract.md` — 要求・受入条件・スコープの正本
- `docs/designs/issue-627-virtual-first-verification-contract-design.md` — 検証契約の設計（`environment_kind`・Case schema・境界 Case・Oracle・マトリクス・明示ルール）
- `docs/designs/issue-627-policies-revision-proposal.md` — 三 repo `POLICIES.md` の行単位照合・適合判定・残件・移管台帳
- `docs/test-specs/issue-627-virtual-first-verification-contract.md` — Feature/Rule/Example による契約の検証仕様
- `docs/plans/issue-627-virtual-first-verification-contract-plan.md` — 本書
- `docs/tasks/issue-627-virtual-first-verification-contract.md` — タスク追跡
- `docs/impl/ir/issue-627-virtual-first-verification-contract.md` — 実施記録
- `evidence/rf627/policies-baseline-comparison.md` — 三 repo baseline SHA・行単位 diff 記録
- `evidence/rf627/verification.md` — 検証・レビュー・衛生チェックの ledger

## Implementation Sequence

- [x] T1: 三 repo `POLICIES.md` の baseline 取得と行単位比較 → `evidence/rf627/policies-baseline-comparison.md`
- [x] T2: spec 作成（問題・要求・受入条件・スコープ）
- [x] T3: 契約設計（`environment_kind` taxonomy・Case schema・3 境界 Case・独立 Oracle・受入マトリクス・Dogfooding・明示ルール）
- [x] T4: POLICIES 照合記録（変更/非変更箇所・残差分・適合判定・ゲート移管台帳・旧物理 Required の扱い）
- [x] T5: test-spec（Rule/Example への写像）
- [x] T6: review-protocol によるレビュー（blocker 解消を承認として扱う）
- [x] T7: docs index 再生成（`doc_maintenance.js index apply`、触れた全 dir）
- [x] T8: `evidence/rf627/verification.md` への検証 ledger 記録
- [x] T9: `mise exec -- prek run --all-files` による衛生チェック
- [ ] T10: コミット・push・PR 作成・worker_done 報告

## Dependencies and Risks

- **依存**: #628〜#631 は本契約（特に Case schema・`environment_kind`・境界 Case）を入力として必要とする。本 plan の完了がそれらの前提。
- **リスク**: 三 repo の `POLICIES.md` が今後さらに改訂された場合、本照合記録の baseline が陳腐化する → baseline SHA を証拠に固定済み。改訂時は再照合。
- **リスク**: audio-lens の `\` 行末スタイル等の既存差異を「残件」として記録したが、修正は POLICIES.md 非変更制約により本 slice では不可 → 別 Issue 候補として記録。
- **リスク**: doc-driven-dev-graph router が repo 全体の既存 broken relation により blocked を返す → 本 slice の文書内容の問題ではなく repo 横断の既存状態として記録し、index 整合は doc-maintenance で個別に担保。

## Task Breakdown

- T1〜T5: 文書成果物（本 plan の File Map どおり）
- T6: review-protocol（spec/design×2/test-spec/evidence を対象。Reviewer A/B → validate → judge → blocker 解消）
- T7〜T9: index 再生成・証拠 ledger・衛生チェック
- T10: 提出

## Verification

- [x] `git status`/`git diff` で変更が許可範囲（`docs/{specs,designs,plans,tasks,test-specs,impl/ir}/issue-627-*`、`evidence/rf627/`、生成 index のみ）に限定されること
- [x] `POLICIES.md`・`docs/development-environment.md`・`mise.toml`・`.github/workflows/ci.yml`・`ci/summary/`・`crates/` に差分がないこと
- [x] `doc_maintenance.js index apply` で触れた dir の index を再生成し、`doc_status.js lint`/`audit` で新規 blocking finding がないこと（既存の repo 横断 finding は別記録）
- [x] `mise exec -- prek run --all-files` が exit 0
- [x] review-protocol の最終 verdict が ACCEPT / ACCEPT WITH NON-BLOCKING NOTES
- [x] test-spec の全 Rule が設計・照合記録・証拠のいずれかに写像されていること（写像は test-spec 本文と本 plan で確認）

## Open Questions / Gaps

- 旧物理 Required Case の実際の `requirement`/`result` 遷移は各 repo のレビュー待ち（本 slice は移管台帳の確定まで）。
- `environment_kind` の値集合は契約で閉集合として固定したが、#628〜#631 実装で追加が必要になった場合は契約改訂として扱う。
- audio-lens の行末 `\` スタイルの統一は別 Issue 候補（POLICIES.md 非変更制約のため本 slice 対象外）。

## Self-Review

- [x] Every spec requirement maps to a task.（spec Requirements 1〜7 → T1 行単位比較、T3 Case schema/境界/Oracle/マトリクス、T4 移管台帳・旧 Required 扱い、T6 明示ルールのレビュー、T9 POLICIES 非変更の検証）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（docs-only のため委譲不要）
- Independent work streams: none
- User approval required before dispatch: yes → review-protocol の blocker なしを承認として扱う（Worker 共通契約）
- If approved: 本 plan に従い T7〜T10 を inline で実行
- If declined or unavailable: 同上（本 slice は委譲を必要としない）

## Review Handoff

- レビュアーは特に以下を確認されたい: (a) `environment_kind` の値集合と「何を証明しないか」の対応が三 repo の運用で誤読されないか、(b) 移管台帳の旧 Issue 網羅性（RF #251/#253/#388/#389/#532/#294、SV #23/#114/#115/#117/#126/#127/#128/#176、AL #127）、(c) 「仮想 Pass ≠ 物理 Pass」の明示ルールが文書間で矛盾しないか。
