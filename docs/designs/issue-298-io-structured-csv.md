---
id: "DESIGN-034k6xAsDaYM7wuYe0CKOF"
type: "design"
status: "approved"
title: "Issue #298 ruviform-io-structured — CSV Module 設計（IO-IMP-04）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/298"
    - "https://github.com/scarecrow173/ruviform/issues/284"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-structured/src/csv.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-io-structured/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-structured/README.md"
      - type: "file"
        path: "crates/ruviform-io-structured/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/input.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/file.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/specs/issue-298-io-structured-csv.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-298-io-structured-csv.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-298-io-structured-csv.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines:
    - "docs/specs/issue-298-io-structured-csv.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-298-io-structured-csv.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #298 ruviform-io-structured — CSV Module 設計（IO-IMP-04）

## 概要

既存 `ruviform-io-structured` crate に `csv` feature を追加し、
`csv` module（`src/csv.rs`）として CSV Record Streaming を実装する。
JSON/YAML/TOML（Issue #297）が確立した共通契約 — caller 借用
`Read`/`Write`・`InputLimits` byte budget・正規化 `DocumentError`・
backend 型非公開・`file` feature 合成・非 RT — をそのまま継承し、
CSV 固有の Record 逐次読取・Dialect 制御・確保前 Hard Limit を
実現する。

### Backend 選定

| 役割 | 採用 crate | 理由 |
| --- | --- | --- |
| Parse engine | `csv-core` 0.1.x | Pull 型 NFA で caller 所有の入力/出力 buffer を渡すため、record/field 数・byte 数を **確保前** に強制できる。上位 `csv` crate の Reader は record 組立てに unbounded `expand_fields()`/`expand_ends()` を内部呼出しするため、肥大化前上限を保証できない。 |
| Record 組立て・Serde 接着・Writer | `csv` 1.4.x | `ByteRecord` の typed deserialize・`Writer` の RFC 4180 準拠 quoting ロジックは実績のある `csv` を再利用。いずれも private。 |
| typed deserialize | `serde` | 既存契約どおり公開 API 境界に出る唯一の外部 trait。 |

`csv` crate 内部の `read_byte_record_impl`（fill_buf → `core.read_record`
→ OutputFull/OutputEndsFull 時の拡張）と同じ駆動パターンを継承しつつ、
拡張点を `max_record_bytes`/`max_fields`/`max_field_bytes` に接続する。

## 詳細設計

### Feature・依存

```toml
[features]
csv = ["dep:csv", "dep:csv-core"]

[dependencies]
csv = { version = "=1.4.0", optional = true, default-features = false }
csv-core = { version = "=0.1.13", optional = true, default-features = false }
```

依存理由: `csv`（record container・serde・writer の成熟実装・
Unlicense/MIT・活発メンテ）と `csv-core`（pull 型 parser・同
ライセンス・`csv` の正規分離部品）を 7 日超経過の pin 版で採用。
`csv` が `csv-core` を内部依存するため両者の版整合は `Cargo.lock`
が保証。`serde` は非 optional で既存どおり。

### 共有構造の拡張（変更は additive）

- `error.rs`: `DocumentFormat::Csv`（`#[non_exhaustive]` enum の
  追加 variant・Display `"csv"`）。`DocumentPosition` に
  `record: Option<u64>` フィールドを additive 追加
  （non_exhaustive struct の拡張 — 既存コンストラクタ
  `new(byte,line,column)` は `record: None` で不変、CSV 用に
  `with_record(...)`/`record_index` 系の builder アクセサを追加）。
- `input.rs`: 変更なし（`BoundedRead` をそのまま使用）。
- `file.rs`: `open_document`/`write_document` 利用箇所の cfg に
  `feature = "csv"` を追加（本体ロジック不変）。
- `lib.rs`: `#[cfg(feature = "csv")] pub mod csv;` と input/file
  module の cfg 条件へ `csv` 追加。

### Options

```rust
#[non_exhaustive]
pub struct CsvReadOptions { /* fields private */
    delimiter: u8, quote: u8, terminator: CsvTerminator,
    comment: Option<u8>, double_quote: bool, escape: Option<u8>,
    quoting: bool, trim: CsvTrim, has_headers: bool, flexible: bool,
    strip_utf8_bom: bool,
    limits: InputLimits,          // max_bytes (16 MiB 既定)
    max_record_bytes: usize,      // 1 MiB 既定
    max_field_bytes: usize,       // 256 KiB 既定
    max_fields: usize,            // 4096 既定
    max_records: u64,             // 1_000_000 既定
}
```

`CsvTerminator { CrLf, Any(u8) }`、`CsvTrim { None, Headers, Fields, All }`、
`CsvQuoteStyle { Necessary, Always, NonNumeric, Never }` — いずれも
`#[non_exhaustive]`。`CsvWriteOptions` は delimiter/quote/
terminator/quote_style/double_quote/escape/has_headers/flexible/
comment を持つ。

Options は builder 風 setter（`delimiter(u8) -> Self` 等）+
`new()`/`Default`。コンストラクタで dialect の矛盾のみ検証:
`delimiter == quote` → `Unsupported`（backend が黙って壊れる既知の
不成立組合せ。Issue の「実現不能制約は Unsupported」に該当）。

### Reader アーキテクチャ

```rust
pub struct CsvReader<'a, R: Read> {
    src: BoundedRead<&'a mut R>,   // InputLimits byte budget
    prefix: Vec<u8>,               // BOM probe で over-read した先頭バイト
    inbuf: Box<[u8]>,              // 8 KiB 入力 chunk
    in_pos: usize, in_len: usize,  // inbuf 内消費位置/有効長
    exhausted: bool,               // BoundedRead 真 EOF
    core: csv_core::Reader,        // pull 型 NFA parser
    options: CsvReadOptions,
    headers: Option<CsvRecord>,    // has_headers 時に確保（typed 用）
    headers_needed: bool,          // has_headers && 未読込
    read_buf: CsvRecord,           // read_typed/内部用 scratch
    expected_fields: Option<usize>,// rigid モードのフィールド数
    byte_pos: u64,                 // parser が消費済み byte（BOM 分含む）
    record_no: u64,                // 消費済み record 数（0-based 次 index）
    record_start: RecordPos,       // 現在 record の開始 byte/line
    read_line: u64,                // core.line()（\n ベース行番号）
    state: ReadState,              // NotEof/Eof/IordError
}
```

#### `read_record(&mut self, record: &mut CsvRecord) -> Result<bool, DocumentError>`

csv crate `read_byte_record_impl` と同型の駆動 loop:

1. 先頭で header 処理 — `headers_needed` なら先に 1 record を
   `read_buf` に内部 parse し `headers` に保持（`has_headers=true`
   ではその record は caller に返さず、次の record を parse して
   返す。`has_headers=false` なら header として返す）。
2. `core.read_record(input, &mut data[outlen..], &mut ends[endlen..])`
   を loop。
   - `InputEmpty` → `fill_buf` 相当: inbuf を圧縮/補充（`BoundedRead`
     から読む。`was_limit_exceeded()` で `LimitExceeded{max_bytes}`）。
     BoundedRead が真 EOF → 次の `read_record(&[])` が `Record`
     （terminator なし最終 record）または `End` を返す。
   - `OutputFull` → **確保前上限**: data buffer を `max_record_bytes`
     まで倍増。cap 到達で `LimitExceeded{max_record_bytes}`。
     かつ in-flight field（outlen − 直近 ends 値）が
     `max_field_bytes` 超過で `LimitExceeded{max_field_bytes}`。
   - `OutputEndsFull` → ends buffer を `max_fields`+1 まで倍増、
     到達で `LimitExceeded{max_fields}`。
   - `Record` → `endlen` を record 長として確定。完成 field の
     byte 長を走査して `max_field_bytes` 検査。`flexible=false`
     で `expected_fields` と不一致 → `DataModel`（record 位置）。
     `record_no` 加算、`max_records` 到達で
     `LimitExceeded{max_records}`。trim 適用（後述）。
   - `End` → `Ok(false)`（EOF）。
3. 位置: record 開始 byte/line を `RecordPos` に保持。
   `byte_pos += nin`・`read_line = core.line()` で更新。

trim は field 境界確定後に data buffer 内で in-place 圧縮
（各 field の先後端空白を詰め、ends を再構築）。backend 側の
`ByteRecord::trim` と同じ意味。

#### `read_typed<T: DeserializeOwned> -> Result<Option<T>, DocumentError>`

内部 `read_record(&mut self.read_buf)` で record を取得し、
`read_buf.fields()` を `ByteRecord::from(&fields)` で backend の
container に充填し `deserialize(header_byte_record)` で `T` を得る。
`Option` header（`has_headers` 時のみ保持）を渡す。backend error の
`ErrorKind::{Deserialize, Utf8, UnequalLengths}` を `DataModel`/
`InvalidEncoding`/`DataModel` に正規化（backend の position を
`DocumentPosition` に写し）。**silent truncation しない** —
deserialize 全体が失敗したら record 全体を返さずエラー終了。

`read_record` 直後に `read_typed` してもよい（既に読んだ record を
型付け直すだけでカーソルは進まない — 但し本実装では `read_typed`
は `read_buf` を内部消費するので混在は行単位で系列化される。
設計では「同一 record の byte/typed 二重取得」は非対象とし、
read_record → read_typed の混在は次 record へ進む動作と定義する）。

#### Strict EOF・先読み境界

- `BoundedRead` は `max_bytes` ちょうどで真 EOF を識別するため
  1 byte 先の probe を行う（`was_limit_exceeded()`）— これは
  Issue #297 で確立済みの共有挙動で、over-limit probe 用に
  **underlying reader から 1 byte だけ余分に要求**する。CSV module
  独自の追加先読みはしない。
- Record 終端の terminator なし最終 record（EOF 直前）を受理する
  のは `read_record(&[])` が `Record` を返す場合のみ。parse が
  `End` を返すまで caller の `read` は完走しないため、未消費の
  malformed trailing bytes は存在しない（parser は不正 quote を
  構文エラーにしないが、それは CSV 仕様どおりの寛容 parse であり
  未消費データではない — 「TrailingData」は CSV には存在しない
  format 特性として文書化）。

#### `CsvRecord`

```rust
pub struct CsvRecord {
    data: Vec<u8>,          // 連結 field bytes（再利用 buffer）
    ends: Vec<usize>,       // field 境界（end offsets、長さ=field 数）
    pos: Option<DocumentPosition>, // record の開始 byte/line/record
}
```

- `field_bytes(i) -> Option<&[u8]>`・`field_str(i) ->
  Result<Option<&str>, DocumentError>`（非 UTF-8 → `InvalidEncoding`）・
  `len()`・`is_empty()`・`iter()`（`&[u8]` を yield）・
  `position()`・`clear()`・`as_slice()`。
- backend の `ByteRecord`/`StringRecord` 型はどこにも出ない。
  typed 経路で内部的に `ByteRecord` に詰め替えるが、それは私有。
- `ends` は data 内の各 field の終端 index（開始は前 ends）。
  `set_len(n)` で field 数確定。`record.data`/`ends` は
  caller の `CsvRecord` を再利用するため record 間で容量を保持。

### Writer

`csv::Writer<&'a mut W>` の薄い wrapper:

```rust
pub struct CsvWriter<'a, W: Write> { inner: csv::Writer<&'a mut W>, options: CsvWriteOptions }
```

- `write_record<I,S>(iter)`: `I: IntoIterator<Item=S>`,
  `S: AsRef<[u8]>` → `inner.write_record`（flexible=false 時の
  非方形は backend の `UnequalLengths` → `DataModel`）。
- `write_typed<T: Serialize>(&T)` → `inner.serialize`（`has_headers`
  時のみ struct field 名から header を自動出力 — backend 契約）。
- `finish(self) -> Result<(), DocumentError>`: `inner.flush()` —
  「writer への flush まで」を契約とし、file publish/sync は
  `file` feature 側の `PublishPlan` 責務。
- ErrorKind: `Io`→`Io`、`Serialize`/`UnequalLengths`→`DataModel`、
  `Utf8`→`InvalidEncoding`。

### 一発 API と file feature

```rust
pub fn from_reader<T: DeserializeOwned>(r: &mut impl Read, o: &CsvReadOptions)
    -> Result<Vec<T>, DocumentError>;
pub fn to_writer<T: Serialize + ?Sized>(w: &mut impl Write, records: &[T], o: &CsvWriteOptions)
    -> Result<(), DocumentError>;
#[cfg(feature = "file")]
pub fn read_file<T: DeserializeOwned>(p: &Path, o: &CsvReadOptions)
    -> Result<Vec<T>, DocumentFileReadError>;
#[cfg(feature = "file")]
pub fn write_file<T: Serialize + ?Sized>(p: &Path, records: &[T], plan: &PublishPlan, o: &CsvWriteOptions)
    -> Result<(), DocumentFileWriteError>;
```

`read_file`/`write_file` は `file.rs` の `open_document`/
`write_document` にそのまま乗せる（呼出し closure が csv の
`from_reader`/`to_writer` を呼ぶ形）。`Vec<T>` 収集の heap は
`max_records`/`max_bytes` で bound されるが advisory 上限として
文書化（設計 v0.2 どおり）。

### エラー正規化表

| 分類 | CSV での典型 |
| --- | --- |
| `Io` | 下層 `Read`/`Write` の I/O error・`finish` の flush 失敗 |
| `InvalidEncoding` | UTF-16 BOM・`field_str`/typed 経路の非 UTF-8 field |
| `Syntax` | （CSV は quote 未閉を受理するため事実上なし） |
| `DataModel` | 非方形（rigid）・serde deserialize 失敗・writer `Serialize`/`UnequalLengths` |
| `TrailingData` | （存在しない — 上記 strict EOF 参照） |
| `LimitExceeded` | `max_bytes`/`max_record_bytes`/`max_field_bytes`/`max_fields`/`max_records` |
| `Unsupported` | `delimiter == quote` 等の不成立 dialect 組合せ |

`DocumentError::limit()`/`position()`/`format()`/`kind()` を継承。
`Display` は backend message を出さない方針を継承（backend 詳細は
`source()` のみ）。

### メモリ・boundedness 不変条件

- record 組立 buffer（data+ends）は caller が `CsvRecord` を
  再利用する限り追加確保なし（初回確保以降は capacity 内に収まる）。
- 入力 buffer は 8 KiB 固定、`BoundedRead` が全体を bound。
- typed 経路は `ByteRecord` への詰め替え（record byte 上限内）と
  逆方向 `Vec<T>`（`max_records` で bound）のみ。
- `read_record` 1 呼出しの peak 追加確保 = 現 record の data/ends
  拡張 ≤ `max_record_bytes` + `max_fields` entries。

### Non-RT・encoding・formula injection

- すべての API は blocking/allocating/非 RT。crate doc/README に明記。
- BOM: `strip_utf8_bom`（既定 true）で UTF-8 BOM のみ除去。
  UTF-16 LE/BE BOM は `InvalidEncoding` で拒否。
- Formula injection: sanitize を既定で行わない。field が `=`,`+`,
  `-`,`@` で始まってもそのまま出力し、必要な対策は Consumer 側の
  Export Policy 責務として README に明記。

## 影響範囲

| 対象 | 変化 |
| --- | --- |
| `Cargo.toml` | `csv` feature・`csv`/`csv-core` optional dep 追加 |
| `src/lib.rs` | `pub mod csv`・input/file cfg に `csv` 追加 |
| `src/error.rs` | `DocumentFormat::Csv`・`DocumentPosition.record` 追加 |
| `src/input.rs` | 不変（`BoundedRead` 再利用） |
| `src/file.rs` | cfg 条件に `csv` 追加 |
| `src/csv.rs` | 新規 |
| `tests/structured.rs` | CSV 試験セクション追加 |
| `crates/README.md` | CSV feature の記述追加 |
| `Cargo.lock` | `csv`/`csv-core` 追加 |

既存 `json`/`yaml`/`toml`/`file` feature・既存公開 API の挙動は
変更しない（`DocumentFormat`/`DocumentPosition` の additive 拡張は
後方互換）。
