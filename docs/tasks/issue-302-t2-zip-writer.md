---
id: "TASK-034RT8aupBUW6NpSHSrRWq"
type: "task"
status: "completed"
title: "T2 ZIP Writer・Entry finish→Archive finish・否定fixture (Issue #302)"
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
    - "docs/tasks/issue-302-t1-scaffold-reader.md"
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
    - "docs/test-specs/issue-302-io-archive-zip.md"
  references: []
  defers: []
  deferred-by: []
---

# T2 ZIP Writer・Entry finish→Archive finish・否定fixture (Issue #302)

## Work

`src/zip/write.rs` に `ZipArchiveWriter<W: Write + Seek>` /
`ZipEntryWriter`（`Write` 実装・CRC32/圧縮 Byte 計数・Local Header
patch-back・Poisoned 遷移）を実装する。`EntryWriter.finish` が Entry
構造を、`ZipArchiveWriter.finish` が CD/EOCD を確定し、Entry を finish
せず Drop した Archive は `InvalidState` で非 Completed にする。
`tests/zip_writer.rs` で finish 型分離・非 Completed・Writer 限界・
Stored/Deflate round-trip を pin する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-302-io-archive-zip.md`
- `docs/designs/issue-302-io-archive-zip.md`
- `docs/specs/issue-302-io-archive-zip.md`

## Dependencies

- `docs/tasks/issue-302-t1-scaffold-reader.md`（共通型・Reader・内部
  Record 定義を再利用）

## Affected Files

- `crates/ruviform-io-archive/src/zip.rs`・`src/zip/write.rs`・
  `tests/zip_writer.rs`

## Done When

- [x] `ZipEntryWriter.finish` → `ZipArchiveWriter.finish` の型分離が
      コンパイル可能な公開 API として存在する
- [x] 未 finish Drop の Archive が `start_entry`/`finish` で
      `InvalidState` を返すことをテストで pin
- [x] Stored/Deflate の round-trip（writer 出力を自前 reader で
      `verify_all`）が緑

## Verification

- [x] `mise run check` / `mise run lint` / `mise run test` が
      新クレート込みで緑
