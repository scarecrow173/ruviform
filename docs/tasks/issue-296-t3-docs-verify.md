---
id: "TASK-034RBIYghnEBLw9llcomyd"
type: "task"
status: "done"
title: "T3 保証範囲文書化・impl record・最終検証 (Issue #296)"
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
    - "docs/plans/issue-296-io-file-os-backend.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-296-io-file-os-backend.md"
    - "docs/tasks/issue-296-t2-real-fs-tests.md"
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

# T3 保証範囲文書化・impl record・最終検証 (Issue #296)

## Work

保証範囲の文書化と最終検証: `lib.rs` crate doc・`README.md` に
capability matrix（OS/FS × CreateNew/Replace/FileSync/DirSync）・
Path root 信頼境界・Symlink/Network FS・Windows 共有 Mode・
Temp identity・Close 段階の報告限界を記載。`docs/impl/ir/` に impl
record を作成し、実行/未実行の検証を正直に分類（Linux/macOS は
CI matrix 委譲）。最後に `git fetch origin` → `git merge origin/main`
→ `Cargo.lock` 再解決 → `mise run verify-ci` で最終検証する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-296-io-file-os-backend.md`
- `docs/designs/issue-296-io-file-os-backend.md`
- `docs/specs/issue-296-io-file-os-backend.md`

## Dependencies

- 先行: `docs/tasks/issue-296-t2-real-fs-tests.md`
- 依存: なし

## Affected Files

- `crates/ruviform-io-file/README.md` `src/lib.rs`
- `docs/impl/ir/issue-296-*.md`
- `Cargo.lock`（main merge 後の再解決）

## Done When

 - [ ] README/lib.rs が capability matrix と信頼境界を正確に記述
- [ ] impl record が実行/未実行の検証を分類する
- [ ] main merge 後に `mise run verify-ci` が緑

## Verification

 - [ ] `mise run verify-ci`（fmt-check→check-ci→lint-ci→test-ci）全緑
