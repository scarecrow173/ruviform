---
id: "TASK-034WZGjTqfOXG0HuO24s2c"
type: "task"
status: "done"
title: "Issue 532 DComp audit doc chain task"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-532-dcomp-audit-task.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-532-dcomp-audit-plan.md"
  implemented-by:
    - "docs/impl/ir/issue-532-dcomp-audit.md"
  depends-on:
    - "docs/plans/issue-532-dcomp-audit-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/specs/issue-532-dcomp-audit.md"
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/test-specs/issue-532-dcomp-audit.md"
    - "docs/impl/ir/issue-532-dcomp-audit.md"
    - "docs/impl/exp/issue-532-dcomp-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-532-dcomp-audit-plan.md"
  derived-by:
    - "docs/impl/ir/issue-532-dcomp-audit.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-audit.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
---

# Issue 532 DComp audit doc chain task

## Work

Issue #532 の残件を headless で処理する文書監査スライス。`crates/ruviform-ui*` の Fake/`NativeSurfaceSlot` 契約・headless 試験の既存被覆を調査し、対象 SHA・検査方法・観測を `docs/discovery/issue-532-dcomp-native-dock.md`（正本）に記録。L2/L3 再開条件と方式 A/B/C 調査手順・担当（#388/#389/SV #176）を記録。Blocked by 誤登録の解消提案を記録。

## Classification

- 種別: 文書監査（コード変更なし）
- 検証レベル: OS-L1（headless・文書のみの軽量チェック）
- 先行 Slice: Deferred 調査（現行 L1 Gate 非 Blocking）

## Upstream Documents

- `docs/specs/issue-532-dcomp-audit.md`
- `docs/designs/issue-532-dcomp-audit-design.md`
- `docs/designs/issue-532-dcomp-native-dock.md`
- `docs/plans/issue-532-dcomp-audit-plan.md`

## Dependencies

- `docs/plans/issue-532-dcomp-audit-plan.md`

## Affected Files

- `docs/discovery/issue-532-dcomp-native-dock.md`（新規・正本）
- `docs/designs/issue-532-dcomp-native-dock.md`（新規）
- `docs/specs/issue-532-dcomp-audit.md`（新規）
- `docs/designs/issue-532-dcomp-audit-design.md`（新規）
- `docs/plans/issue-532-dcomp-audit-plan.md`（新規）
- `docs/tasks/issue-532-dcomp-audit-task.md`（本ファイル）
- `docs/test-specs/issue-532-dcomp-audit.md`（新規）
- `docs/impl/ir/issue-532-dcomp-audit.md`（新規）
- `docs/impl/exp/issue-532-dcomp-audit.md`（新規）
- `docs/<type>/README.md`（index 再生成。impl/ir は apply で再生成、impl/exp の .md 一覧は手動追記）

## Done When

- 正本監査記録が F1（制約）・F2（L1 被覆）・F3（CI 対応付け）・F4（依存状態）・再開条件・A/B/C 手順・Blocked by 提案を含む。
- 実機調査設計が再開条件・Step 1〜4・方式 A/B/C 採否条件・契約維持条件を含む。
- 文書チェーンの front matter relations が相互に解決可能。
- 触った `docs/<type>/` dir の index が再生成される。
- `mise run fmt-check` + `mise exec -- prek run --files` が exit 0。
- 実 Native Dock 未成立が「未確証」として明記されている。

## Verification

- `mise run fmt-check`
- `mise exec -- prek run --files <変更ファイル>`
- `doc-maintenance` の `index apply` + `doc-status`
- `review-protocol` によるレビュー（blocker なしを承認として扱う）
