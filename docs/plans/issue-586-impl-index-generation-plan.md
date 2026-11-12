---
id: "PLAN-8R4q0cr04yTyNYSk6Y5f3B"
type: "plan"
status: "approved"
title: "Issue #586 実施計画 — ir front matter 是正と ir/exp index 再生成"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/586"
    - "docs/specs/issue-586-impl-index-generation.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-586-impl-index-generation-plan.md"
      - type: "file"
        path: "docs/tasks/issue-586-impl-index-generation.md"
      - type: "file"
        path: "docs/test-specs/issue-586-impl-index-generation.md"
    modified:
      - type: "directory"
        path: "docs/impl/ir/"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements:
    - "docs/specs/issue-586-impl-index-generation.md"
  implemented-by:
    - "docs/tasks/issue-586-impl-index-generation.md"
  depends-on:
    - "docs/specs/issue-586-impl-index-generation.md"
    - "docs/designs/issue-586-impl-index-generation-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-586-impl-index-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-586-impl-index-generation-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-586-impl-index-generation.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #586 実施計画 — ir front matter 是正と ir/exp index 再生成

## Steps

1. doc-chain(spec/design/plan/task/test-spec)を作成し本 plan に記録する。
2. `new_impl_record.js` で Issue #586 の in-progress IR を作成し、作業中に維持する。
3. `docs/impl/ir` の 30 件の front matter を design の分類 A–E に従って機械修正する(本文・ファイル名・既存 id は不変)。
4. `docs/impl/ir/README.md` に marker を挿入し、`docs/impl/exp/README.md` 末尾に managed region を追加する。
5. `index plan --dir docs/impl/ir` / `--dir docs/impl/exp` で plan を取り、`--expect <planHash>` 付き `index apply` → 再 plan で `in-sync` を確認。引数なし `index apply` が exit 0 であることも確認する。
6. 触った docs dir(specs/designs/plans/tasks/test-specs)の管理 index を `doc-maintenance` の index apply で再生成する。
7. `audit_impl_record.js` / `audit_experiment_log.js` を変更後に実行し baseline と比較する。main を merge して index を再生成し、commit → push → PR。

## File map

- 新規: `docs/{specs,designs,plans,tasks,test-specs}/issue-586-impl-index-generation*.md`(5 件)、`docs/impl/ir/issue-586-*.md`(IR 1 件)
- FM 修正: `docs/impl/ir/` の 30 件(一覧は design 参照)
- 再生成: `docs/impl/ir/README.md`、`docs/impl/exp/README.md`、各 docs dir の `README.md`

## Verification matrix

| 検証 | コマンド | 合格条件 |
| --- | --- | --- |
| ir index | `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index plan --dir docs/impl/ir` | `rejected` でなく `in-sync`(apply 後) |
| exp index | 同上 `--dir docs/impl/exp` | `in-sync` |
| 全 index | `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index apply` | exit 0 |
| audit | `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` / `audit_experiment_log.js --json` | schema 適合済みファイルの error 非増加、invalid-front-matter 解消を確認 |
| 不変性 | `git diff` で既存文書の本文・ファイル名・id 差分なし | FM のみ |

## Risks

- index plan の source 検証がさらに違反を検出する可能性 → apply 前に plan を再取得し blocker を潰してから進める。
- `index apply` が生成 index を大きく書き換える → owner の sync engine のみを使い手編集しない。managed region 外の手書き内容は保持される。
- audit error 合計の増加 → front matter 適合化で missing-section が可視化される分は既知の pre-existing 不足として分離報告する。
