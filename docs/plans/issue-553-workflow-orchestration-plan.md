---
id: "PLAN-bGpV2PYHt511Xo50RSXU97"
type: "plan"
status: "completed"
title: "Issue #553 Workflow/CLI orchestration契約 文書スライス実装計画"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/553"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-553-workflow-orchestration-plan.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-553-workflow-orchestration.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-553-workflow-orchestration-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-553-workflow-orchestration.md"
    - "docs/designs/issue-553-workflow-orchestration-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-553-workflow-orchestration.md"
  references:
    - "docs/adr/issue-553-workflow-cli-contract.md"
    - "evidence/rf553/contract-trace.md"
    - "evidence/rf553/consumer-applicability.md"
    - "evidence/rf553/verification.md"
  defers: []
  deferred-by: []
---

# Issue #553 Workflow/CLI orchestration契約 文書スライス実装計画 Implementation Plan

## Goal

Issue #553 の DESIGN-ONLY スライスとして、Consumer-neutral Workflow/CLI orchestration 契約の文書チェーン（spec/design/ADR/test-spec/plan/task/IR/exp）と `evidence/rf553/` 証拠を完成させ、管理 index を再生成して検証を通す。

## Scope

**1 実装ストリーム**: 文書のみ。`crates/**`・`Cargo.toml`/`Cargo.lock`/`mise.toml`/CI・Consumer コードは対象外。実装 crate・公開 API・PoC コードは生成しない（再評価 Trigger 成立後の別 Issue）。

## Upstream Documents

- `docs/specs/issue-553-workflow-orchestration.md`
- `docs/designs/issue-553-workflow-orchestration-design.md`
- `docs/adr/issue-553-workflow-cli-contract.md`
- `docs/test-specs/issue-553-workflow-orchestration.md`

## File Map

| ファイル | 種別 | 内容 |
| --- | --- | --- |
| `docs/specs/issue-553-workflow-orchestration.md` | spec | 問題・Audience・Scope・Non-goals・受入条件・Evidence |
| `docs/designs/issue-553-workflow-orchestration-design.md` | design | 棚卸し・契約語彙（C2.1〜C2.6）・2 Consumer Scenario・Fault Matrix・Deferred |
| `docs/adr/issue-553-workflow-cli-contract.md` | adr | Option A/B/C 比較・Option B 採用・Candidate 分類・後続 Issue・再評価 Trigger |
| `docs/test-specs/issue-553-workflow-orchestration.md` | test-spec | R-PLAN/R-VALID/R-PORT/R-STATE/R-CANCEL/R-IDEM/R-NONRT/R-GRANT/R-CLI/R-TYPE/R-PRECISION・EX-* |
| `docs/plans/issue-553-workflow-orchestration-plan.md` | plan | 本計画 |
| `docs/tasks/issue-553-workflow-orchestration-task.md` | task | 文書チェーン完成・検証タスク |
| `docs/impl/ir/issue-553-workflow-orchestration.md` | impl/ir | 実装記録 |
| `docs/impl/exp/issue-553-workflow-orchestration.md` | impl/exp | 文書監査・検証試行記録 |
| `evidence/rf553/contract-trace.md` | evidence | 受入条件→契約条項→根拠対応表 |
| `evidence/rf553/consumer-applicability.md` | evidence | 2 Consumer 適用性判定ログ |
| `evidence/rf553/verification.md` | evidence | 検証記録 |

## Dependency Graph

`spec → design → adr → test-spec → plan → task → ir/exp`。design は adr に depends-on、plan は design に depends-on、task は plan に implements、ir は task に implements。test-spec は spec/design/adr を verifies。

## Risk Register

| リスク | 影響 | 対応 |
| --- | --- | --- |
| 契約が文書に留まり実装がない | 共通化が未実証 | ADR で Deferred を明示し再評価 Trigger を固定。test-spec で将来の検証 Rule を固定 |
| Synvelis AUTX-05/06 未確定で generic を固定 | Consumer 側発見を取り込めず手戻り | Synvelis 具象を先行させ、再評価 Trigger で generic を再検討 |
| AudioLens が orchestration を必要としない | 片側成功での共通化採用 | consumer-only 継続を明示し、AudioLens/第三 Consumer の独立必要性を Trigger にする |
| Consumer 型流入・第二正本 | 境界違反 | design §C2.6/C4 と test-spec R-GRANT/R-TYPE/R-PORT で禁止 |

## Rollback Strategy

文書のみの変更であり、`git revert` で文書チェーンと index を戻せる。コード・依存・CI への影響はない。

## Verification Matrix

| 検証 | 方法 | 結果 |
| --- | --- | --- |
| 文書 front matter・relation・構造 | `doc-status`/`audit_docs`/`audit_adr` | 自スコープ error なし |
| 管理 index 整合 | `doc-maintenance index plan/apply` | 触れた dir で `in-sync` |
| Rust fmt | `mise run fmt-check` | 文書のみだが正規検証として実行 |
| Markdown/リンク軽量チェック | `mise exec -- prek run --files <変更ファイル>` | Pass |
| Runtime/Consumer/OS 実機 | `mise run check`/`test`/`test-rt`/`test-ci-consumer-verify` | **NotRun**（コード変更なし・文書契約のみ） |
