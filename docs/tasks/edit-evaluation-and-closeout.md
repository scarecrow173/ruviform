---
id: "TASK-034KPgQj9u5utlkxH3JGqv"
type: "task"
status: "todo"
title: "edit 評価射影 + テスト・文書クローズ"
created: "2026-10-23"
updated: "2026-10-23"
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
    - "docs/plans/implement-issue-96-edit-model.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-96-edit-model.md"
    - "docs/tasks/edit-crate-and-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# edit 評価射影 + テスト・文書クローズ

## Work

`Timeline::evaluate` の評価射影（`Evaluation`/`ActiveClip`）を実装し、
test-spec 全ケースのテスト・crate README・`crates/README.md` 索引行・
検証列・doc 監査で Issue #96 の doc チェーンを閉じる（plan 手順 3–5）。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-96-edit-model.md
- docs/tasks/edit-crate-and-model.md

## Dependencies

- docs/tasks/edit-crate-and-model.md（骨格 + データモデル型・mutation）

## Affected Files

- crates/ruviform-edit/src/eval.rs（`Evaluation`/`ActiveClip`/`evaluate`）
- crates/ruviform-edit/src/lib.rs（re-export 追加）
- crates/ruviform-edit/README.md（crate README 新設）
- crates/README.md（索引行追加 — engine と同順位の利用側層）
- docs/*（チェーン frontmatter: task status・plan Verification チェック）

## Done When

- [ ] `evaluate` が design の算術経路（`at` 1 回変換・半開包含・後開始/
  同開始後挿入優先・`source_time` 写像・到達可能残区間・`TimeOverflow`）
  で `Evaluation`/`ActiveClip` を返す
- [ ] test-spec 全ケースパス
- [ ] crate README + `crates/README.md` 索引行・検証列全パス・
  doc 監査チェーン起因 findings 0

## Verification

- [ ] `mise run fmt-check`/`check`/`lint`/`test` パス
- [ ] `doc_status.js audit` findings 0・依存が `ruviform-media-core` のみ
