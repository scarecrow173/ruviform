---
id: "TASK-034WZKWej3HLlqCGxHSDcH"
type: "task"
status: "in-progress"
title: "Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付けタスク"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "docs/specs/issue-275-obs-owner-map.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/designs/issue-275-obs-owner-map-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/plans/issue-275-obs-owner-map-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-275-obs-owner-map-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/impl/exp/issue-275-obs-owner-map.md"
      - type: "file"
        path: "evidence/rf275/obs-owner-map-audit.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-275-observability-integration-design.md"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-275-obs-owner-map-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-275-obs-owner-map-plan.md"
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
    - "docs/test-specs/issue-275-obs-owner-map.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
---

# Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付けタスク

## Work

Issue #275 の残件（headless 監査・文書スライス）を実施する。

1. `evidence/rf275/obs-owner-map-audit.md` に Issue 状態
   （`gh issue view`）・crate/feature/公開 API・CI lane の
   実測 snapshot を固定する。
2. `docs/designs/issue-275-observability-integration-design.md`
   に OBS-01〜11 の Owner Issue（URL 付き）・公開 API/型・
   feature・既存 CI（Native Build/Link/Headless/RT）Case 別の
   対応表と、Deferred 範囲の Owner/Trigger 表を追記する。
   AL #127・SV #28 の独立 Headless 受入を #281/#294 への
   引き渡しとして明記する。
3. `issue-275-obs-owner-map` 文書チェーン（spec/design/
   test-spec/plan/task/impl ir・exp）を完成させる。
4. 触った dir の生成 index を再生成し、軽量検証
   （`mise run fmt-check`・`prek run --files`）と
   review-protocol レビューを行う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-275-obs-owner-map-plan.md`
- `docs/specs/issue-275-obs-owner-map.md`
- `docs/designs/issue-275-obs-owner-map-design.md`
- `docs/test-specs/issue-275-obs-owner-map.md`
- `docs/designs/issue-275-observability-integration-design.md`
  （更新対象の正本）

## Dependencies

- 親 spec/design（issue-275-observability-integration-design）
  は approved。
- 実装 Issue #276〜#280・#289〜#293 は CLOSED（実装正本）。
  OBS-06 #281・#294・#219・AL #127・SV #28 は OPEN（検証
  Owner — 本タスクの完了条件ではない）。

## Affected Files

- `docs/designs/issue-275-observability-integration-design.md`
- `evidence/rf275/obs-owner-map-audit.md`（新規）
- `docs/specs|designs|test-specs|plans|tasks/issue-275-obs-
  owner-map*.md`（新規チェーン）
- `docs/impl/ir/issue-275-obs-owner-map.md`・
  `docs/impl/exp/issue-275-obs-owner-map.md`（新規）
- `docs/{specs,designs,plans,tasks,test-specs}/README.md`・
  `docs/impl/{ir,exp}/README.md`（生成 index の再生成/
  手動追記）

## Done When

- [ ] OBS-01〜11 の Owner/API/Feature/CI Case 対応表が統合
      設計正本に追記されている
- [ ] AL #127・SV #28 の引き渡しと Deferred Owner/Trigger が
      明記されている
- [ ] 文書チェーンと監査証拠が完成している
- [ ] index が再生成/追記され、軽量検証と review-protocol
      レビューが完了している

## Verification

- [ ] `mise run fmt-check` が成功する
- [ ] `mise exec -- prek run --files <変更ファイル>` が成功する
- [ ] `doc-maintenance`/`sync_impl_index`/`audit_docs` で
      整合を確認する
- [ ] 対応表の全行が `evidence/rf275/obs-owner-map-audit.md`
      の実測と一致する
- [ ] review-protocol で blocker が残っていない
