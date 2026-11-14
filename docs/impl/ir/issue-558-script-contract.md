---
id: "IMPL-034XegYdJwFzsQhPajRIAg"
type: "impl"
status: "completed"
title: "Issue #558 Script/Analyzer契約 文書チェーン実装記録"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/558"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/issue-558-script-contract.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-558-script-contract-task.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-558-script-contract.md"
    - "docs/designs/issue-558-script-contract-design.md"
    - "docs/adr/issue-558-script-backend-contract.md"
    - "docs/test-specs/issue-558-script-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/impl/exp/issue-558-script-contract.jsonl"
    - "evidence/rf558/verification.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-558-script-contract.jsonl"
    rejected: []
---

# Issue #558 Script/Analyzer契約 文書チェーン実装記録

## Summary

Issue #558 の DESIGN-ONLY スライスとして、言語非依存 Script/Analyzer 契約の文書チェーン（spec/design/ADR/test-spec/plan/task/IR/exp）と `evidence/rf558/` 証拠を作成した。実装コード・crate・CI・依存変更は含まない。

## Context

- Task: `docs/tasks/issue-558-script-contract-task.md`
- Plan: `docs/plans/issue-558-script-contract-plan.md`
- Spec: `docs/specs/issue-558-script-contract.md`
- Design: `docs/designs/issue-558-script-contract-design.md`（approved）
- ADR: `docs/adr/issue-558-script-backend-contract.md`（accepted）
- Test-spec: `docs/test-specs/issue-558-script-contract.md`

## Implementation

### Changes

- `docs/specs/issue-558-script-contract.md` - 要件・受入条件・Deferred を定義（SPEC-034Xe8fKEL6vpFX9pkgPU0）。
- `docs/designs/issue-558-script-contract-design.md` - 公開型・Registry/Dispatch・値モデル・Budget/Cancel・Analyzer Port・Fail-Closed 表（F-01〜F-18）・Fault Matrix（M-01〜M-13）・Fixture（FX-P-01〜P-08・FX-N-01〜N-24）・2 Consumer サマリを定義（DESIGN-034XeaFjxZePL0m6LYuEeS）。
- `docs/adr/issue-558-script-backend-contract.md` - 言語非依存 Port + 実行時 Registry/Dispatch（Option C）を採用する決定記録（ADR-034Xe9xZEqz5ftcu5yCqXB）。
- `docs/test-specs/issue-558-script-contract.md` - 契約条項を Rule（R-REG〜R-DOC）と Example（FX-P/N）へ写像（TSPEC-034XecRP3Wpuq0BscTS283）。
- `docs/plans/issue-558-script-contract-plan.md` - 文書スライスの実装計画。
- `docs/tasks/issue-558-script-contract-task.md` - 文書チェーン完成・検証タスク。
- `evidence/rf558/contract-trace.md` - 受入条件→契約条項→根拠対応表。
- `evidence/rf558/consumer-applicability.md` - 2 Consumer Reuse/New/Deferred 判定ログ。
- `evidence/rf558/verification.md` - 検証記録（実行コマンド・結果）。
- `docs/{specs,designs,adr,test-specs,plans,tasks,impl}/README.md` - `doc-maintenance` で管理 index を再生成。

### Decisions During Implementation

- **ADR Option C 採用**: Vendor 型公開（A）・単一言語固定（B）・ビルド時排他（D）を却下し、言語非依存 Port + 実行時 Registry/Dispatch を採用。根拠は `docs/impl/exp/issue-558-script-contract.jsonl` の decision イベント。
- **DESIGN-ONLY 境界**: 実装 crate・公開 API・PoC コードを生成せず、契約レベル定義（型・不変条件・エラー分類）に留めた。実 Runtime 共存・3OS・native FFI は #559/#560 へ委譲。
- **Fail-Closed の機械検査化**: 判定表（F-01〜F-18）と Fault Matrix（M-01〜M-13）を Fixture（FX-P/N）へ割当、test-spec Rules で検証可能な形に固定。
- **2 Consumer 独立判定**: Synvelis（Reuse+Consumer-only）と AudioLens（Deferred/NotApplicable）を分離し、「1 Consumer の成功 ≠ 共通採用」を明記。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-558-script-contract.jsonl` - 言語非依存 Port + 実行時 Registry/Dispatch（ADR Option C）の採用決定。

### Rejected

- なし（文書スライスのため実装アプローチの比較実験は不要。ADR で代替案を検討済み）。

## Validation

- `evidence/rf558/verification.md` に実行コマンド・OS・toolchain・exit code・結果を記録。
- 文書監査（`audit_docs`/`audit_adr`/`audit_impl_record`/`doc_status`）で front matter・relation・index 整合を確認。
- `mise run fmt-check` + `prek run --files` で Markdown/リンクの軽量チェックを実行。
- 未実行項目（実 Runtime 共存・3OS・native FFI・Rhai 本番採用・AL Script 適用）は NotRun/Deferred と明記。

## Risks

- 契約の網羅性不足 → review-protocol で spec/design/ADR/test-spec をレビューし blocker を解消。
- 未実測項目を Pass と誤認 → `evidence/rf558/verification.md` に実コマンド・結果を記録し未実行は NotRun/Deferred。
- 管理 index の手編集禁止 → `doc-maintenance` の preview-first に従う。

## Follow-ups

- #559: Luau+Rhai 同一バイナリ共存・実行時選択・Fail Closed の実測（FX-P-07・FX-N-01〜N-08）。
- #560: Luau.Analysis C++ native FFI・版整合・3OS 配布。
- #553: Workflow/CLI 共通化判断（本契約の 2 Consumer 判定を材料に）。
- 実装 crate 配置・公開 API の Rust 確定 → #559/#560 PoC + #553 判定後。
- Synvelis/AudioLens 独立受入 → #294・SV #28・AL #127。
