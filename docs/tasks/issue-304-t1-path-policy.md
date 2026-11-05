---
id: "TASK-PGN5ItjVgMoSpqmL4YJtjn"
type: "task"
status: "done"
title: "T1 Path Policy・canonical 衝突・FS trait 骨格 (Issue #304)"
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
    - "docs/plans/issue-304-io-archive-extract.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-304-io-archive-extract.md"
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
    - "docs/test-specs/issue-304-io-archive-extract.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 Path Policy・canonical 衝突・FS trait 骨格 (Issue #304)

## Work

`extract` module の骨格（`src/extract.rs` + `extract/` 配下）を
作り、`SafeRelativePath`（`ArchiveName -> SafeRelativePath` の
唯一の検証入口）と canonical 衝突キー（NFC + lowercase）を実装する。
`ArchiveReadLimits` に `max_path_depth` を追加し、`extract/fs.rs`
に `PlatformExtractOps`/`StagingDir`/`StagingFile`/`NodeIdentity`
trait・factory・`#[cfg(test)]` mock を定義する。`unicode-normalization`
依存を Cargo.toml に追加し、`tests/extract_path.rs` で否定/受理
行列を pin する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-304-io-archive-extract.md`
- `docs/designs/issue-304-io-archive-extract.md`

## Acceptance

- `SafeRelativePath::new` が `..`/絶対/drive/UNC/`\`/`:`/予約名/
  末尾 `.`/空白/NTFS 禁止文字/空・`.` component/非 UTF-8/NUL を
  拒否し、通常の相対 file/dir 名のみ受理することを `extract_path`
  で pin する。
- canonical key で casefold・NFC/NFD 差・同一 entry が同一 key に
  畳まれることを単体試験で pin する。
- `ArchiveReadLimits` に `max_path_depth` が追加され、既存 reader が
  そのまま動く（`#[non_exhaustive]` 追加 field）。
- `extract/fs.rs` の trait/mock がコンパイルされ、`libc`/`windows-sys`/
  `unicode-normalization` が `Cargo.toml` に追加される。
