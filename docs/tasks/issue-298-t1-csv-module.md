---
id: "TASK-034TKw7zfSgzOzI8oNY79D"
type: "task"
status: "done"
title: "Issue #298 T1 — csv module 本体と共有拡張"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/298"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-structured/src/csv.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-structured/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-structured/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/input.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
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
    - "docs/tasks/issue-297-t1-crate-and-contract.md"
  refines:
    - "docs/plans/issue-298-io-structured-csv.md"
  refined-by: []
  derives-from:
    - "docs/plans/issue-298-io-structured-csv.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #298 T1 — csv module 本体と共有拡張

## 目的

`csv` feature で公開される `csv` module（`src/csv.rs`）と、それを
可能にする共有構造の additive 拡張を実装する。

## 対象変更

- `Cargo.toml`: `csv = ["dep:csv", "dep:csv-core"]` feature・
  optional 依存 `csv = "=1.4.0"`・`csv-core = "=0.1.13"`（いずれも
  `default-features = false`）。
- `src/error.rs`: `DocumentFormat::Csv`・`DocumentPosition.record`
  追加。`with_parts`/`with_limit` の cfg に `feature = "csv"` 追加。
- `src/lib.rs`: `#[cfg(feature = "csv")] pub mod csv;`・
  input/file module cfg に `csv` 追加。
- `src/input.rs`: 変更不要（`BoundedRead` は変更なし・`read_to_string`
  の cfg には `csv` を含めない — CSV は byte 処理のため）。
- `src/csv.rs`: 新規。Options/Record/Reader/Writer/一発 API。

## 実装仕様（design 参照）

- `CsvReadOptions`/`CsvWriteOptions`/`CsvTerminator`/`CsvTrim`/
  `CsvQuoteStyle` は `#[non_exhaustive]` + builder setter。
- `CsvRecord` は連続 `Vec<u8>` + `Vec<usize>` ends。
- `CsvReader<'a, R: Read>` は `BoundedRead<&'a mut R>` + 8 KiB
  inbuf + `csv_core::Reader` 駆動 loop（`OutputFull`/`OutputEndsFull`
  で hard limit、`InputEmpty` で補充、`End` で EOF）。
- `read_typed<T: DeserializeOwned>` は `ByteRecord` +
  `deserialize(header)` を内部的に利用。
- `CsvWriter<'a, W: Write>` は `csv::Writer` の薄い wrapper・
  `finish` = `flush`。
- `from_reader`/`to_writer` は `Vec<T>`/`&[T]` の convenience。

## 受入基準

- [ ] `cargo check --features csv`・`--all-features` が通る。
- [ ] AC1/AC2/AC3 に対応する unit test が `tests/structured.rs` に
  追加され `mise run test` で全緑。
- [ ] backend 型が公開 signature にない。
- [ ] `cargo doc` で csv module の rustdoc が生成される。

## 完了条件

- `mise run fmt`/`check`/`lint`/`test` 全緑（変更範囲内）。
- T2 で扱う file feature テスト・README・impl record は未対象。
