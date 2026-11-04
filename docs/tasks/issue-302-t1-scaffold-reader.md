---
id: "TASK-034RT89xVtZ6NMuRh08JQg"
type: "task"
status: "completed"
title: "T1 ruviform-io-archive クレート新設・Archive共通型・ZIP Indexed Reader (Issue #302)"
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

# T1 ruviform-io-archive クレート新設・Archive共通型・ZIP Indexed Reader (Issue #302)

## Work

`crates/ruviform-io-archive` を新設し workspace `members` に登録する。
共通公開型（`ArchiveError`/`ArchiveErrorKind`・`ArchiveName`・
`EntryKind`・`ArchiveEntryInfo`・`ArchiveReadLimits`・
`ArchiveReadReceipt`/`EntryReadReceipt`/`EntryChecksum`）と、内部
Parser による ZIP Indexed Reader（EOCD/CD 解析・Budget 統合・排他借用
`ZipEntryReader`・`finish` 検証・`verify_all`・途中 Drop→再 Open）を
実装する。依存は `crc32fast` + `flate2`(rust_backend) に限定し Backend
型を公開 API に出さない。Reader 側の構造/Budget/否定 fixture テストを
`tests/zip_reader.rs` に実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-302-io-archive-zip.md`
- `docs/designs/issue-302-io-archive-zip.md`
- `docs/specs/issue-302-io-archive-zip.md`

## Dependencies

- なし（crate 依存は外部 crate `crc32fast`/`flate2` のみ。Issue #286
  v0.1/v0.2 契約が実装基準）

## Affected Files

- `crates/ruviform-io-archive/**`（新設。Writer 以外の全 source）
- `Cargo.toml`（members）・`Cargo.lock`（cargo 再解決）

## Done When

- [x] 共通公開型と `ZipArchiveReader`/`ZipEntryReader`/`verify_all` が
      コンパイル可能で rustdoc 典型使用例・non-RT 明記がある
- [x] `tests/zip_reader.rs` で Index 読取・途中 Drop→別/再 Open・
      `verify_all` receipt・Budget 事前/途中強制・truncated/overlap/
      Local-CD 不整合・Unsupported 分類が pin されている

## Verification

- [x] `mise run check` / `mise run lint` / `mise run test` が
      新クレート込みで緑
