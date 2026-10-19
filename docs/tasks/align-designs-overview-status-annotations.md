---
id: "TASK-034IRZAn8ce6BYjOG6KhrZ"
type: "task"
status: "done"
title: "docs/designs/overview.md の status 注記整合"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  implemented-by: []
  depends-on:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  blocks:
    - "verify-issue-1-repo-reflection.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# docs/designs/overview.md の status 注記整合

## Work

plan `PLAN-034IR3qOr15LGNY9mEindJ` のスライス T-IDX（Phase X-1）。`docs/designs/overview.md` 本文中の `ruviform-architecture-design-baseline.md` 参照に残る stale 注記を approved へ更新し、「#3 の境界」注記に #1 A.8 反映タスクの実施を追記する（索引整合。design 明示タスク外の plan 追加スライス）。

- [ ] X-1. `ruviform-architecture-design-baseline.md` 参照に付く stale 注記を approved へ更新する: L35 `（draft）` → `（approved）`、L40 `、draft` → `、approved`、L94 `(draft)` → `(approved)` の 3 形式。あわせて L42「README／AGENTS 本文の改稿は #3 の境界」に「#1 設計ベースライン approved に伴う設計反映の差分更新は #1 A.8 タスクとして実施」旨を追記する（designs README 索引は approved を表示済み・body 注記のみ追従）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/apply-issue-1-architecture-baseline-to-repo.md](../plans/apply-issue-1-architecture-baseline-to-repo.md) — `PLAN-034IR3qOr15LGNY9mEindJ`（approved）。Phase T-IDX / V-6 の正本
- [../designs/overview.md](../designs/overview.md) — 編集対象。設計成果物の全体マップ
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。注記が参照する design 本体

## Dependencies

- 共有前提: plan S-1（変更前ベースライン記録。`(draft)` 注記箇所の記録を含む）は pre-dispatch ステップとして実施済みであること
- 後続をブロック: `verify-issue-1-repo-reflection.md`（V-6）
- T-PUB・T-README とは互いに独立（S-1 後に並行可能）
- `docs/designs/overview.md` 自身の front matter `status: "draft"` は本タスクでも変更しない（map 文書の lifecycle 判断は plan Open Questions / Gaps で範囲外と明示）

## Affected Files

- `docs/designs/overview.md` — baseline 参照の stale 注記 3 箇所（L35・L40・L94）の更新と L42 への追記（annotation-edit。front matter `status` は無変更）

## Done When

- [ ] `ruviform-architecture-design-baseline.md` 参照の `(draft)` 系注記 3 箇所がすべて `(approved)` 系に更新されている
- [ ] L42 の「#3 の境界」注記に #1 A.8 反映タスク実施の旨が追記されている
- [ ] front matter `status: "draft"` が変更されていない
- [ ] `audit_docs.js` が新規の不整合を報告しない（V-6）

## Verification

- [ ] `grep -nE "（draft）|、draft|\(draft\)" docs/designs/overview.md` が一致を返さない
- [ ] `grep -n "approved" docs/designs/overview.md` が L35・L40・L94 相当の更新済み注記を返す
- [ ] `grep -n '^status: "draft"' docs/designs/overview.md` が front matter の `status: "draft"` を返す（無変更確認）
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` が新規の不整合を報告しない（V-6）
