---
id: "IMPL-034RTBzSkSM8okNaDF9rGQ"
type: "impl"
status: "completed"
title: "Issue 302 T2 ZIP writer"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/302"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-archive/src/zip/write.rs"
      - type: "file"
        path: "crates/ruviform-io-archive/tests/zip_writer.rs"
      - type: "file"
        path: "docs/impl/ir/issue-302-t2-zip-writer.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-archive/src/zip.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-302-t2-zip-writer.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-302-io-archive-zip.md"
    - "docs/impl/ir/issue-302-t1-archive-scaffold-and-zip-reader.md"
    - "https://github.com/scarecrow173/ruviform/issues/286"
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

# Issue 302 T2 ZIP writer

## Summary

`Write + Seek` 宛先への ZIP writer を実装した。`start_entry` が zero
値の Local Header を先行書き込みし、`ZipEntryWriter` が CRC32・計測
size を記録しながら payload を stream し、entry `finish()` で seek
して Local Header を patch-back、`ZipArchiveWriter::finish()` で
中央 Directory + EOCD を書き出す。entry 完了と archive 完了を別の
明示 `finish` に分離し、未完成 entry の Drop は archive を poison
して `InvalidState` にする — 「Drop が完成扱いにならない」契約を
型状態ではなく runtime poison で保証する。

## Context

- 実施タスク: [T2](../../tasks/issue-302-t2-zip-writer.md)
- 上流: [design](../../designs/issue-302-io-archive-zip.md)、
  [test-spec](../../test-specs/issue-302-io-archive-zip.md) の R9/R10、
  [T1 IR](./issue-302-t1-archive-scaffold-and-zip-reader.md)
- 設計 Issue #286（IO-04）の finish 分離・Pending 状態機械契約

## Implementation

### Changes

- `src/zip/write.rs` — `ZipArchiveWriter<W>`（records ledger +
  `poisoned` フラグ）、`ZipEntryWriter<'a, W>`（排他 borrow・sink は
  `CountWrite` で compressed bytes を計測）、`ZipEntryOptions`・
  `ZipCompressionMethod`（`#[non_exhaustive]`、Stored/Deflate）。
  `CountWrite` shim で deflate の実 compressed bytes を正確に把握する。
- `add_directory` — trailing `/` の empty stored entry として扱い、
  external attrs で dir を標記。
- `tests/zip_writer.rs` — round-trip、empty archive、dir marker、
  drop-poison → `InvalidState`、entry/archive finish 分離、name/entry
  count（65 535）limit、measured size 記録の 10 件。

### Decisions During Implementation

- Local Header patch-back 方式を採用（data descriptor は出力しない）
  — `Write + Seek` が必須なので最も単純で広く読める形式を選んだ。
- 未完成 entry Drop は archive を poison（`InvalidState`）— 「途中
  Drop = 無害な cancel」ではなく「partial bytes は既に sink に出た
  ので archive 全体を完成不可にする」が正直な失敗分類。
- `finish` での `&mut W` 回収: sink（`&'a mut W` を所有）を
  `encoder.finish()` で消費して `&'a mut W` を取り戻し、seek/patch を
  行う — 複数の `&mut self.inner` 同時生存を避ける分割 borrow 設計。
- LFH 部分書き込み失敗時も archive を poison — 断片的 bytes が既に
  流れた以上、後続 start/finish を成功させると壊れた archive が
  「完成」に見えてしまう。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo test -p ruviform-io-archive`: `tests/zip_writer.rs` 10 件
  全パス（round-trip/poison/finish 分離/limit/measured size を網羅）
- writer→reader round trip は `ZipArchiveReader` での再解析 +
  `verify_all` で検証

## Risks

- deflate の実 compressed bytes は `CountWrite` shim 経由の実測値 —
  `DeflateEncoder` の内部 buffer は `finish()` で flush されるため
  patch-back 時点で確定している（round-trip で確認）。
- 65 535 entry 境界試験は空 stored entry 65 535 個を書くため
  ~2 MiB を生成する（一瞬で完了）。

## Follow-ups

- T3: ledger/index 再生成・workspace 全検証（別 IR）
- ZIP64 出力は非対応のまま — 超過時は `Unsupported` で早期分類済み
