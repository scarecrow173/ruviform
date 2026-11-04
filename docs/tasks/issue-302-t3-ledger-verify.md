---
id: "TASK-034RT8bdlM0FbHyrczoIR2"
type: "task"
status: "completed"
title: "T3 共有台帳・main取込・最終検証・IR (Issue #302)"
created: "2026-11-04"
updated: "2026-11-04"
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
    - "docs/plans/issue-302-io-archive-zip.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-302-io-archive-zip.md"
    - "docs/tasks/issue-302-t2-zip-writer.md"
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

# T3 共有台帳・main取込・最終検証・IR (Issue #302)

## Work

`crates/README.md` のクレート台帳（総数・汎用 I/O 節）と
`docs/*/README.md` 生成索引を更新する。`git fetch origin` →
`git merge origin/main` で並行 worker（ruv-297/ruv-300）の crate
追加と衝突解消し、`Cargo.lock` を再解決する。`mise run fmt`/`check`/
`lint`/`test` の最終検証を実施し、`docs/impl/ir/` に IR を記録して
コミット・push・PR 作成・Required Check 待機まで行う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-302-io-archive-zip.md`
- `docs/specs/issue-302-io-archive-zip.md`

## Dependencies

- `docs/tasks/issue-302-t2-zip-writer.md`（実装完了後に台帳/検証）

## Affected Files

- `crates/README.md`・`docs/specs|designs|plans|tasks|test-specs|
  impl/ir/README.md`（生成索引）・`Cargo.lock`・
  `docs/impl/ir/issue-302-*.md`（新設）

## Done When

- [x] `git merge origin/main` 済み・衝突は自 crate 追加分を保持して解消
- [x] `mise run fmt`/`check`/`lint`/`test` が緑・IR が記録されている
- [x] push + PR 作成 + Required Check 待機まで完了

## Verification

- [x] `mise run fmt` / `mise run check` / `mise run lint` /
      `mise run test`（workspace 全体）
