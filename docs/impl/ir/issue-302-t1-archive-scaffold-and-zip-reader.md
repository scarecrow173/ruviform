---
id: "IMPL-034RTBzh681ofU6ln4xYYr"
type: "impl"
status: "completed"
title: "Issue 302 T1 archive scaffold and ZIP reader"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/302"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-archive/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-archive/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/entry.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/limits.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/zip.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/zip/record.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/src/zip/read.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/zip_reader.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/README.md"
      - type: "file"
        path: "docs/impl/ir/issue-302-t1-archive-scaffold-and-zip-reader.md"
      - type: "file"
        path: "docs/specs/issue-302-io-archive-zip.md"
      - type: "file"
        path: "docs/designs/issue-302-io-archive-zip.md"
      - type: "file"
        path: "docs/plans/issue-302-io-archive-zip.md"
      - type: "file"
        path: "docs/test-specs/issue-302-io-archive-zip.md"
      - type: "file"
        path: "docs/tasks/issue-302-t1-scaffold-reader.md"
      - type: "file"
        path: "docs/tasks/issue-302-t2-zip-writer.md"
      - type: "file"
        path: "docs/tasks/issue-302-t3-ledger-verify.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/issue-302-t1-scaffold-reader.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-302-io-archive-zip.md"
    - "docs/designs/issue-302-io-archive-zip.md"
    - "docs/plans/issue-302-io-archive-zip.md"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/303"
    - "https://github.com/scarecrow173/ruviform/issues/304"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 302 T1 archive scaffold and ZIP reader

## Summary

IO-IMP-08（Issue #302）の scaffold 部分として独立クレート
`ruviform-io-archive` を新設し、形式中立の公開契約
（`ArchiveError`/`ArchiveErrorKind`・`ArchiveName`・`EntryKind`・
`ArchiveEntryInfo`・`ArchiveReadLimits`・receipts）と、ZIP indexed
Reader（`Read + Seek`）を実装した。EOCD 末尾探索→budget 前検査→
中央 Directory 一括解析→open 時の Local Header 照合という lazy な
検証段階分けを採り、`verify_all()` が全エントリの measured
size/CRC32/region 重複を網羅検証する。

## Context

- 実施タスク: [T1](../../tasks/issue-302-t1-scaffold-reader.md)
- 上流: [plan](../../plans/issue-302-io-archive-zip.md)、
  [design](../../designs/issue-302-io-archive-zip.md)、
  [spec](../../specs/issue-302-io-archive-zip.md)、
  [test-spec](../../test-specs/issue-302-io-archive-zip.md)、
  設計 Issue #286（IO-04）・親 Issue #282
- 後続: TAR = Issue #303、Secure Extraction = Issue #304
  （本 scaffold の公開型に乗る）
- 規約参照: `ruviform-io-file`（IO-IMP-01/02）の型状態・分類語彙・
  backend 隠蔽パターンを踏襲

## Implementation

### Changes

- `src/error.rs` — `ArchiveErrorKind`（`#[non_exhaustive]`）と
  `ArchiveError`（`Cow<'static,str>` message + `Option<Box<dyn Error +
  Send + Sync>>` source）。`From<ArchiveError> for io::Error` を提供し、
  `Read` impl から分類済みエラーを `io::Error` 経由でも取得可能にした。
- `src/entry.rs` — `ArchiveName`（raw bytes 保持・`to_str`/`to_string_lossy`）、
  `EntryKind`（`#[non_exhaustive]`）、`ArchiveEntryInfo`。
- `src/limits.rs` — `ArchiveReadLimits`（`#[non_exhaustive]` + public
  fields + `Default`）。entries / metadata bytes / path bytes /
  per-entry・total uncompressed bytes の 5 軸 budget。
- `src/receipt.rs` — `EntryReadReceipt`（measured bytes +
  `EntryChecksum`）と `ArchiveReadReceipt`（total/verified/unverified/
  payload bytes の正確な内訳）。
- `src/zip/record.rs` — private な on-disk record 定義と LE helper。
- `src/zip/read.rs` — `ZipArchiveReader` + 排他 borrow の
  `ZipEntryReader`。EOF 完全一致 EOCD 探索（~64KiB 窓）、CD 前 budget
  ゲート、record 単位解析、open 時の Local/CD 整合・region 監査、
  hard-cap 付き stream read（cap 到達時に 1 byte probe で超過を
  `SizeMismatch`/`LimitExceeded` に分類）、`finish()` による
  CRC/size 確定、`verify_all` の全走査 + data region 重複 audit。

### Decisions During Implementation

- 検証段階分け: `new` = index 解析 + lower-bound region audit、
  Local Header 照合と payload 検証は `open_entry`/`verify_all` に
  遅延。Issue #286 の「index は構造、payload は検証」方針どおり。
- 上限超過の分類: cap 到達時に source を 1 byte probe し、追加出力が
  あれば `hard_cap < declared` なら `LimitExceeded`、等しければ
  `SizeMismatch` — 宣言超過と budget 超過を区別できる。
- Drop で未検証を表すのは `verified: Vec<bool>` スロット —
  `ZipEntryReader` が `&mut` で指し、`finish` 成功時のみ true。
- Stored/Deflate の source は `Take`（CD 宣言 comp_size で bounded）を
  共有し、Deflate は `DeflateDecoder<Take>`。backend 型は public API に
  一切露出しない。
- `read_local_header` は `&mut self` メソッドではなく free fn —
  `record`（`self.entries` の borrow）との disjoint-field 衝突を避ける。

## Related Experiments

### Adopted

なし（方針確定済みの機械的タスクのため exp は作成していない）。

### Rejected

なし。

## Validation

- `cargo check -p ruviform-io-archive --all-targets`: パス
- `cargo clippy -p ruviform-io-archive --all-targets -- -D warnings`: パス
- `cargo test -p ruviform-io-archive`: `tests/zip_reader.rs` 24 件 +
  共有 build → 全パス（truncated/malformed/inconsistent/overlap/
  unsupported/budget/CRC/partial-drop/verify_all を網羅）

## Risks

- `read()` は cap 到達時に 1 byte の probe read を行う。`Ok(0)` を
  「clean EOF」と見なす前提は deflate decoder が stream 終端で 0 を
  返すことに依存 — round-trip/deflate 試験で確認済み。
- EOCD 探索は後方向の「signature + comment_len 完全一致」規則。
  細工された comment 内偽 signature は後続 CD 解析で失敗に倒れる
  （fail-closed）。

## Follow-ups

- T2: `ZipArchiveWriter`/`ZipEntryWriter`（別 IR）
- T3: ledger/index 再生成・全 workspace 検証（別 IR）
- IO-IMP-09/10 向け: `ArchiveName` の path-safety 契約文書は抽出層
  実装時に再確認する
