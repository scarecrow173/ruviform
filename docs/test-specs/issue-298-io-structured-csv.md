---
id: "TSPEC-034Mm7FbtxkfxmQCgVk1m4"
type: "test-spec"
status: "approved"
title: "Issue #298 ruviform-io-structured — CSV Module 試験仕様（IO-IMP-04）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/298"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-298-io-structured-csv.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-298-io-structured-csv.md"
    - "docs/designs/issue-298-io-structured-csv.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-298-io-structured-csv.md"
    - "docs/designs/issue-298-io-structured-csv.md"
    - "docs/test-specs/issue-297-io-structured-json-yaml-toml.md"
  refines:
    - "docs/plans/issue-298-io-structured-csv.md"
  refined-by: []
  derives-from:
    - "docs/plans/issue-298-io-structured-csv.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-298-io-structured-csv.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #298 ruviform-io-structured — CSV Module 試験仕様（IO-IMP-04）

## 試験配置

既存 `tests/structured.rs` に CSV セクションを追加
（`#[cfg(feature = "csv")]` ブロック）。format 固有の試験は
`mod csv_tests` 内にまとめる。`file` 合成は
`#[cfg(all(feature = "csv", feature = "file"))]` で別ブロック。

## Golden / Positive

| ID | 内容 | 期待 |
| --- | --- | --- |
| G1 | typed round-trip（`from_reader` → 同 `to_writer` → 再 parse） | 値一致・`to_writer` が `finish` まで成功 |
| G2 | quoted field（comma・quote・newline 内包） | 内容保持で field を返す |
| G3 | quoted newline（`"a\nb"` 跨ぎ record） | 1 record として復元 |
| G4 | CRLF / LF / CR terminator 混在 | 各 record 正常取得（`Terminator::CRLF` 既定） |
| G5 | UTF-8 BOM 入力（`strip_utf8_bom` default） | BOM 除去後 record 正常 |
| G6 | `strip_utf8_bom=false` + BOM | 先頭 field に BOM bytes が残る |
| G7 | 空行・空 record | 空行 skip・`record.is_empty()`/0-field record を正しく処理 |
| G8 | dialect: TSV（tab delimiter）・`;`・custom quote `'`・comment `#`・no-quoting | 指定 dialect で正しく parse |
| G9 | header あり typed deserialize（`has_headers=true`） | struct 名と header の対応で `T` が取れる |
| G10 | `has_headers=false` で header 無し CSV | 全行が data record として返る |
| G11 | `read_record` → `CsvRecord` reuse | 複数 record で同一 `CsvRecord` を再利用し値が更新される |
| G12 | `field_str` UTF-8 / `field_bytes` | byte/str 双方でアクセス可能 |
| G13 | `position()`（byte/line/record index） | 各 record の開始位置が正しい |

## Negative

| ID | 内容 | 期待 kind/limit |
| --- | --- | --- |
| N1 | `max_bytes` 超過入力 | `LimitExceeded{max_bytes}`（boundary exact/超過の 2 ケース） |
| N2 | `max_record_bytes` 超過 record | `LimitExceeded{max_record_bytes}` |
| N3 | `max_field_bytes` 超過 field | `LimitExceeded{max_field_bytes}` |
| N4 | `max_fields` 超過 record | `LimitExceeded{max_fields}` |
| N5 | `max_records` 超過入力 | `LimitExceeded{max_records}` |
| N6 | 不正 quote（unclosed quote） | 寛容 parse — CSV では `Syntax` ではなく field として受理、または `End` まで読む（backend 仕様どおり・明文化） |
| N7 | 非 UTF-8 field で `field_str`/`read_typed<String>` | `InvalidEncoding` |
| N8 | UTF-16 BOM | `InvalidEncoding` |
| N9 | `has_headers=true` で header 数≠data 数（rigid） | `DataModel` |
| N10 | `has_headers=false` で行ごとの field 数不一致（rigid） | `DataModel` |
| N11 | typed deserialize 失敗（型不一致） | `DataModel`（silent truncation なし） |
| N12 | `delimiter == quote` 等の不成立 dialect | `Unsupported` |
| N13 | `write_record` で flexible=false 非方形 | `DataModel` |
| N14 | `finish` 時に下層 writer が error | `Io` |
| N15 | `read_typed` の途中で下層 `Read` error | `Io` |

## 構造・制約試験

| ID | 内容 | 期待 |
| --- | --- | --- |
| S1 | streaming — `Read` を途中で切っても先頭 record が返る | 増分消費を pin（全入力一括読みでないこと） |
| S2 | record buffer reuse — 同一 `CsvRecord` で N record 読み | `data`/`ends` capacity が単調非減・中身が逐次上書き |
| S3 | `read_record` → `read_typed` の混在 | カーソルが正しく進む（二重取得は非対象で次 record へ） |
| S4 | borrowed reader lifetime — `CsvReader` を `&mut cursor` で作り drop 後も cursor 使用可 | borrow が `&mut` のみ・`'static` でないことを静的に確認 |
| S5 | `read_file`/`write_file` round-trip（csv+file） | `DocumentFileReadError`/`DocumentFileWriteError` で各失敗軸分離 |
| S6 | deterministic fuzz — seeded LCG で random dialect CSV + random truncations | panic なし・結果が規定 kind に分類される |
| S7 | `DocumentPosition.record` が record index を返す | `position().record()` が 0/1/2... |
| S8 | 公開 API audit — csv.rs の pub signature に `csv::`/`csv_core::` 型名がない | 目視/grep で backend 型非公開を pin |

## 補助検証（canonical ではない）

- feature matrix: `cargo check/test --no-default-features`・`--features csv`・`--features csv,file`・`--all-features`。
- MSRV: `mise exec rust@1.89.0 -- cargo check -p ruviform-io-structured --all-features`。
- peak memory 観測: `Read` を仕掛けて入力全読みしないことを確認（S1）+ record buffer capacity の単調非減（S2）+ `from_reader` の `Vec<T>` が `max_records` bound であることを assert。

## 未実施・制約

- 外部 fuzz harness（`cargo fuzz` 等）は repo 基盤にないため、
  deterministic seeded LCG fuzz で代替し、外部 fuzz 化は別 Issue。
- 実 hardware/OS/GUI 依存の検証は対象外（全て headless）。
- `read_file` の `Vec<T>` は advisory bound — 巨大 CSV の実測
  メモリは performance baseline として impl record に記録。
