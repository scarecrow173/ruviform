---
id: "TASK-emXL9hZuOAzbTa1mAxJSN3"
type: "task"
status: "done"
title: "T1 TAR header Parser・共通型拡張・Streaming Reader/Pending (Issue #303)"
created: "2026-11-05"
updated: "2026-11-05"
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
    - "docs/plans/issue-303-io-archive-tar.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-303-io-archive-tar.md"
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
    - "docs/test-specs/issue-303-io-archive-tar.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 TAR header Parser・共通型拡張・Streaming Reader/Pending (Issue #303)

## Work

`src/tar/header.rs`（512 block field 定数・checksum 検証/生成・
octal/base-256 数値・USTAR magic 判定・PAX record parse・header
分解）を実装する。共通型を最小拡張する
（`ArchiveEntryInfo.link_name: Option<ArchiveName>`・
`EntryReadReceipt.disposition: EntryDisposition` と ZIP 側呼出しの
整合）。`src/tar.rs` module root と `src/tar/read.rs`
（`TarArchiveReader<R: Read>`・`next_entry`・`Ready/EntryOpen/
Pending/EndSeen/Failed`・`TarEntryReader`・`finish`/
`discard_remaining`・`discard_pending`・archive `finish` の drain+
終端検証・budget 統合）を実装し、`tests/common/tar.rs` の byte 級
builder と `tests/tar_reader.rs` の否定行列を整備する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-303-io-archive-tar.md`
- `docs/designs/issue-303-io-archive-tar.md`

## Acceptance

- `next_entry` が USTAR/GNU `L`/`K`/PAX `x` を実効値へ解決し、
  `S`/`3`/`4'`/未知 typeflag を `Unsupported`、構造欠陥を設計の
  kind へ分類する。
- 途中 Drop → Pending → `discard_pending` 復帰・失敗時 `Failed`・
  `finish`/`discard_remaining` の disposition 分離が動作する。
- budget 5 種が事前・途中で強制され、reader 側テストが緑。
