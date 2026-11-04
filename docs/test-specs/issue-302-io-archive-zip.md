---
id: "TSPEC-034RT7OuFi1aZ0rhvdSFEt"
type: "test-spec"
status: "approved"
title: "Issue #302 ruviform-io-archive ZIP 検証・Budget・Writer テスト仕様"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "docs/specs/issue-302-io-archive-zip.md"
    - "https://github.com/scarecrow173/ruviform/issues/302"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-archive/tests/zip_reader.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/zip_writer.rs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-302-io-archive-zip.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-302-io-archive-zip.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #302 ruviform-io-archive ZIP 検証・Budget・Writer テスト仕様

## Purpose

`ruviform-io-archive` の ZIP 契約 — Indexed Reader の構造/Budget 検証・
排他借用 Entry・`verify_all` の検証サマリ・Writer の明示 finish 分離 — を
byte レベルの fixture（正常系 + 手作り否定系）で pin する。Issue #302
Acceptance（構造/CRC/Local-CD 整合・truncated・overlap・Entry 数・
大 Header の Limit、Index 読取/途中 Drop/再 Open/verify_all、Writer
finish、ZIP64・Stored/Deflate・未知 Codec/Encryption の分類）が正本。
Secure Extraction 自体の FS 安全性は #304 の Gate であり本 spec では
検証しない。

## Feature

ZIP Archive の Index 解析・Entry 展開・Archive 生成において、構造欠陥・
宣言値と実測の不一致・Budget 超過・未対応機能が、設計どおりの
`ArchiveErrorKind` に分類され、`finish` 成功のみが完了証拠となる。

## Rules

- R1（Index/Budget 事前強制）: `cd_count > max_entries` または
  `cd_size > max_metadata_bytes` の EOCD は CD byte を読まず
  `LimitExceeded`。record 途中の累積 metadata が上限を超えても
  `LimitExceeded`。`name_len > max_path_bytes` → `LimitExceeded`。
- R2（構造分類）: file `< 22`・EOCD 不在・CD 切れ → `Truncated`。
  署名不一致・範囲外 offset・宣言 count ≠ 実 record 数 → `Malformed`。
  `cd_offset + cd_size > eocd_offset` → `Malformed`/`Overlap`。
- R3（overlap）: Entry 下界領域（`local_offset .. local_offset+30+
  comp_size`）が CD 領域または他 Entry 下界領域と重複 → `Overlap`。
  Entry open 時に実 data 領域が CD 領域へ侵入 → `Overlap`。
- R4（Local/CD 整合）: Local Header の署名不正 → `Malformed`、
  name/method/flag・非 zero 時の size/crc が CD と矛盾 → `Inconsistent`。
- R5（Entry 展開検証）: CRC 不一致 → `ChecksumMismatch`。実展開
  size ≠ 宣言 → `SizeMismatch`（超過検出は read() 時点で即 Err、
  超過 Byte は呼出し側へ返さない）。実測 > `max_entry_uncompressed_bytes`
  → `LimitExceeded`。
- R6（所有権/再検証）: `ZipEntryReader` は同時 1 つ（借用規則）。
  途中 Drop → 別 index・同一 index の再 open が可能で、その Entry は
  receipt 上 unverified のまま。`verify_all` は全 Entry の
  `entries_total`/`entries_verified`/`payload_bytes` を正確に返す。
- R7（全体 Budget）: `verify_all` の累積展開 Byte が
  `max_total_uncompressed_bytes` を超過 → `LimitExceeded`。
- R8（Unsupported 分類）: general purpose bit0（encryption）・未知
  method・Multi-disk（非 0 disk 番号）・飽和 EOCD field/ZIP64 locator →
  `Unsupported`（黙って処理しない）。
- R9（Writer finish 分離）: `ZipEntryWriter.finish` が Entry 構造を
  確定し、`ZipArchiveWriter.finish` が CD/EOCD を書く。Entry を finish
  せず Drop → 後続 `start_entry`/`finish` は `InvalidState`。
  Writer 出力は自前 Reader で round-trip 可能（Stored/Deflate 双方）。
- R10（Writer 限界）: name `> u16::MAX`・size `> u32::MAX`・entry 数
  `> 0xFFFF` は `InvalidInput`/`Unsupported`/`LimitExceeded` で拒否
  （ZIP64 出力は作らない）。
- R11（共通型契約）: `ArchiveName` は raw bytes を保持し `to_str` は
  UTF-8 のみ `Some`。`ArchiveEntryInfo` の size は CD 宣言値。
  `EntryReadReceipt.checksum` は ZIP では `Verified` のみ。

## Examples

- Stored+Deflate の 3 Entry archive を writer で生成 → reader で
  `entry_count==3`・各 `open_entry` → read 全量 → `finish()` で
  `EntryReadReceipt{checksum:Verified, uncompressed_bytes=実測}` →
  `verify_all` で `entries_verified==3`。
- Stored Entry の data byte を 1 byte 反転 → `open_entry` 全読 →
  `finish()` が `ChecksumMismatch`、`verify_all` が同 Err。
- 宣言 uncomp_size=0 の Stored Entry に実 data を残す/実 data を削る →
  `SizeMismatch`（超過は read 時点）。
- `max_entries=1` の limits で 3 Entry archive を open → `LimitExceeded`
  （CD byte 未到達）。
- local_offset を CD 領域内に書き換えた fixture → open で `Overlap`/
  `Malformed`（Entry 領域 × CD 領域）。
- Entry Writer を `finish` せず `drop` → `archive.finish()` が
  `InvalidState`、再度 `start_entry` も `InvalidState`。

## Guarantees

- `open` が成功 = EOCD/CD が Budget 内で整合し、Index が読めたことのみ。
  全 Entry の CRC/実測検証は `verify_all`/`entry.finish` 成功時のみ。
- Budget 判定は「宣言値の事前拒否」と「実測の途中遮断」の両層で機能し、
  超過 Byte は下流 `Read` 呼出しへ渡さない。
- Writer が `finish` せずに出力を Completed とみなす経路は存在しない。

## Non-goals

- 実 FS への展開・Path Traversal 防止・TOCTOU（#304）。
- 他ツール（Info-ZIP/Windows 展開）との相互運用性試験 — round-trip は
  自前 Reader/Writer 間のみ。
- ZIP64 実データの受理試験（Unsupported 拒否のみ pin）。
- CRC の暗号学的真正性（CRC は整合性検出であり authenticity 検証は
  対象外）。

## Risk

- 内部 Parser の分類がずれると、破損 Archive を `Malformed` 以外で
  誤受理/誤分類し、#304 が偽の「検証済み」を前提にするリスク。
- Deflate 経路で宣言超過の即時遮断を怠ると爆弾防御が形骸化する。
- Writer の非 Completed 化を緩めると、未完成 ZIP が `finish` 成功として
  下流 publish へ流れるリスク。
