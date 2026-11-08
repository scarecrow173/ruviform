---
id: "PLAN-UmRPBDr4lZ2y8Aj9H6tdYe"
type: "plan"
status: "approved"
title: "Issue #377 実装計画 — QueryBinding adopt-on-join"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-377-query-binding-epoch.md"
    - "docs/designs/issue-377-query-binding-epoch.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-377-query-binding-epoch.md"
    - "docs/designs/issue-377-query-binding-epoch.md"
  implemented-by:
    - "docs/tasks/issue-377-query-binding-epoch.md"
  depends-on:
    - "docs/designs/issue-377-query-binding-epoch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-377-query-binding-epoch.md"
    - "docs/designs/issue-377-query-binding-epoch.md"
  derived-by:
    - "docs/tasks/issue-377-query-binding-epoch.md"
  verifies:
    - "docs/test-specs/issue-377-query-binding-epoch.md"
  verified-by:
    - "docs/test-specs/issue-377-query-binding-epoch.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/377"
  defers: []
  deferred-by: []
---

# Issue #377 実装計画

## 段階

| 段 | 内容 | 検証 |
| --- | --- | --- |
| P1 | `ui/binding/test/link.test.ts` に late-created query の恒久テストを追加（test-spec 全ケース） | `mise run test-ui-binding` で期待する RED を確認 |
| P2 | `ui/binding/src/link.ts` の `registerQuery` に adopt-on-join を実装 + `link.ts`/`query.ts`/`contract.ts` の docstring 明確化 | 同上で GREEN |
| P3 | `mise run check-ui-binding` / `test-ui-binding` / `build-ui-binding` / `check-ui-guitest` / `test-ui-guitest` / `fmt-check` | 各 gate green |
| P4 | `origin/main` 取込・生成 index 再同期・review-protocol・論理単位 commit → push → 日本語 PR | PR 本文に検証実測と #378 向け副次的発見を記録 |

## 依存

- P2 は P1 の RED 確認に依存する。
- P3 は P2 完了後。`test-ui-guitest` は `replay.ts` が `createQuery` の
  唯一の in-repo caller であることの回帰確認として実施する。

## リスク

| リスク | 対策 |
| --- | --- |
| attach 済み link への in-flight 付き外部 query 登録で既存 request が cancel される | 意図した意味論（join = epoch 遷移）。doc comment と恒久テストで明文化・固定する |
| `registerQuery` 二重呼出で `attach` が再実行される | 既存の Set membership と同じく実害なし（同 epoch 再採用 + 空の cancelAll）。新規の防御は追加しない |
| query の `viewId` が link と不一致のまま残る | 既存挙動を維持（`setContext` 相当機構は query に無い）。本 Issue 範囲外として design に記録 |
