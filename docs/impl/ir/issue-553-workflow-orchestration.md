---
id: "IMPL-0J35geTVP3p5cuN80AKQMT"
type: "impl"
status: "completed"
title: "Issue #553 Workflow/CLI orchestration契約 文書チェーン実装記録"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/553"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/issue-553-workflow-orchestration.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-553-workflow-orchestration-task.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-553-workflow-orchestration.md"
    - "docs/designs/issue-553-workflow-orchestration-design.md"
    - "docs/adr/issue-553-workflow-cli-contract.md"
    - "docs/test-specs/issue-553-workflow-orchestration.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/impl/exp/issue-553-workflow-orchestration.md"
    - "evidence/rf553/verification.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-553-workflow-orchestration.md"
    rejected: []
---

# Issue #553 Workflow/CLI orchestration契約 文書チェーン実装記録

## Summary

Issue #553 の DESIGN-ONLY スライスとして、Consumer-neutral Workflow/CLI orchestration 契約の文書チェーン（spec/design/ADR/test-spec/plan/task/IR/exp）と `evidence/rf553/` 証拠を作成した。実装コード・crate・CI・依存変更は含まない。

## Context

- Task: `docs/tasks/issue-553-workflow-orchestration-task.md`
- Plan: `docs/plans/issue-553-workflow-orchestration-plan.md`
- Spec: `docs/specs/issue-553-workflow-orchestration.md`
- Design: `docs/designs/issue-553-workflow-orchestration-design.md`（approved）
- ADR: `docs/adr/issue-553-workflow-cli-contract.md`（accepted）
- Test-spec: `docs/test-specs/issue-553-workflow-orchestration.md`

## Implementation

### Changes

- `docs/specs/issue-553-workflow-orchestration.md` - 要件・受入条件・Non-goals・Evidence を定義（SPEC-thW5o4Z4OMp1n1quFRk4uh）。
- `docs/designs/issue-553-workflow-orchestration-design.md` - 棚卸し対応表（C1）・契約語彙（C2.1〜C2.6）・2 Consumer Scenario（C3）・Fault/Recovery Matrix（C4）・Deferred（C5）を定義（DESIGN-sh006tWu0ht5WmSqG0V85w）。
- `docs/adr/issue-553-workflow-cli-contract.md` - Option A（共通 Engine 今実装）/B（契約語彙のみ・実装 Deferred）/C（No-go）を比較し Option B を採用する決定記録（ADR-JwwKl26JnYjSWNJAuhGJs5）。
- `docs/test-specs/issue-553-workflow-orchestration.md` - 契約条項を Rule（R-PLAN〜R-PRECISION）と Example（EX-SV/EX-AL/EX-CANCEL/EX-RESUME）へ写像（TSPEC-cFTjIZpnYGLaA5BcC74ahh）。
- `docs/plans/issue-553-workflow-orchestration-plan.md` - 文書スライスの実装計画。
- `docs/tasks/issue-553-workflow-orchestration-task.md` - 文書チェーン完成・検証タスク。
- `evidence/rf553/contract-trace.md` - 受入条件→契約条項→根拠対応表。
- `evidence/rf553/consumer-applicability.md` - 2 Consumer Reuse/Extend/New/Consumer-only/Deferred 判定ログ。
- `evidence/rf553/verification.md` - 検証記録（実行コマンド・結果）。
- `docs/{specs,designs,adr,test-specs,plans,tasks,impl/ir,impl/exp}/README.md` - `doc-maintenance` で管理 index を再生成（`docs/impl/exp/README.md` は generated index が `*.jsonl` のみ対象のため、free-form `.md` 記録は「Reports and raw data」節へ手動追記）。

### Decisions During Implementation

- **ADR Option B 採用**: 共通 Engine/CLI crate の今実装（A）・完全 No-go（C）を却下し、契約語彙のみ文書化・実装 Deferred を採用。根拠は `docs/impl/exp/issue-553-workflow-orchestration.md` の試行記録と `evidence/rf553/consumer-applicability.md` の判定。
- **DESIGN-ONLY 境界**: 実装 crate・公開 API・PoC コードを生成せず、契約レベル定義（型・不変条件・Fail-Closed 規則・Owner 境界）に留めた。Synvelis AUTX-05/06/03 と AudioLens 既存 stage-job が具象 Owner。
- **第二正本・Consumer 型流入の禁止**: `synvelis-gateway`/`synvelis-project`/GPUI/AudioLens 製品型/独自 Grant Policy を generic 契約へ流入させず、Capability/Policy/Project/Job/Approval の第二正本を作らないことを design §C2.6/C4 と test-spec R-GRANT/R-TYPE/R-PORT で固定。
- **2 Consumer 独立判定**: Synvelis（Consumer-only 具象 + 契約語彙適用可能）と AudioLens（consumer-only 継続・現要件で orchestration 不要）を分離し、「1 Consumer の成功 ≠ 共通採用」と片側成功での採用禁止を明記。
- **Script 非重複**: rf#558〜560 の Script Runtime/Analyzer 契約と分離し、本契約の Plan は Script lowering の出力先であり VM を orchestration に含めない。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-553-workflow-orchestration.md` - 既存 API 棚卸し・Synvelis/AudioLens 境界抽出・文書規約適用の試行記録。ADR Option B 採用の根拠。

### Rejected

- なし（文書スライスのため実装アプローチの比較実験は不要。ADR で代替案を検討済み）。

## Validation

- `evidence/rf553/verification.md` に実行コマンド・OS・toolchain・exit code・結果を記録。
- 文書監査（`audit_docs`/`audit_adr`/`audit_impl_record`/`doc_status`）で front matter・relation・index 整合を確認。
- `mise run fmt-check` + `prek run --files` で Markdown/リンクの軽量チェックを実行。
- 未実行項目（実 Runtime 共存・実 OS・実 Job/Resume/Cancel・両 Consumer 移行・Workflow Engine 実装）は NotRun/Deferred と明記。

## Risks

- 契約が文書に留まり実装がない → 共通化が未実証。ADR で Deferred を明示し再評価 Trigger を固定。test-spec で将来の検証 Rule を固定。
- Synvelis AUTX-05/06 未確定で generic を固定 → Consumer 側発見を取り込めず手戻り。Synvelis 具象を先行させ、再評価 Trigger で generic を再検討。
- AudioLens が orchestration を必要としない → 片側成功での共通化採用。consumer-only 継続を明示し、AudioLens/第三 Consumer の独立必要性を Trigger にする。
- 未実測項目を Pass と誤認 → `evidence/rf553/verification.md` に実コマンド・結果を記録し未実行は NotRun/Deferred。
- 管理 index の手編集禁止 → `doc-maintenance` の preview-first に従う。

## Follow-ups

- RF-AUTX-02 候補（Deferred）: Ruviform 共通 orchestration 実装。再評価 Trigger 成立後に起票。
- Synvelis 側追従: synvelis#28（Ruviform 依存追従）・synvelis#259〜#267（AUTX lane）が具象 Owner。generic 契約の adapter 適合は各 AUTX Issue の受入で確認。
- AudioLens 側追従: audio-lens#127（継続追従）・#189（旧 WebView 撤去）。orchestration 適用は現要件で consumer-only 継続。
- 実装 crate 配置・公開 API の Rust 確定 → 再評価 Trigger 成立後の別 Issue。
- 両 Consumer 独立受入 → rf#294・SV #28・AL #127。
