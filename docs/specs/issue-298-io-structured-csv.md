---
id: "SPEC-0340TuOY8fzYFyCTHKGsWs"
type: "spec"
status: "approved"
title: "Issue #298 ruviform-io-structured — CSV Record Streaming・Dialect・Typed Serde (IO-IMP-04)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/298"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/219"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #298 ruviform-io-structured — CSV Record Streaming・Dialect・Typed Serde (IO-IMP-04)

## What Should Be Built

IO-IMP-03（Issue #297）が確立した `ruviform-io-structured` の共通
scaffold（`DocumentError` 族・`InputLimits`・`file` feature 合成・
feature matrix）の上に、IO-02（Issue #284）v0.2 合意済み設計の
**CSV Record Streaming** を `csv` feature/module として追加する。
新規クレートは作らない。

公開面（feature `csv` の `csv` module）:

- `CsvReader<'a, R: Read>` — caller が所有する `&mut R` を借用する
  Record 逐次読取器。`read_record(&mut self, &mut CsvRecord) ->
  Result<bool, DocumentError>`（EOF = `Ok(false)`）とオプションの
  `read_typed<T: DeserializeOwned>(&mut self) ->
  Result<Option<T>, DocumentError>` を同じカーソル上で提供する。
  隠れた rewind・先読みによる見えない消費は行わない。
- `CsvRecord` — バイト指向の再利用可能 Record buffer。
  内部は連続 `Vec<u8>` + field 境界 index（設計 v0.2 の形状）。
  `field_bytes(i) -> Option<&[u8]>`・`field_str(i) ->
  Result<Option<&str>, DocumentError>`・`len()`・`iter()`・
  `position()` を公開し、`csv::ByteRecord`/`StringRecord` 等の
  backend 型は公開しない。
- `CsvWriter<'a, W: Write>` — Record 単位の出力
  （`write_record`・`write_typed<T: Serialize>`）と明示
  `finish()`。`finish()` 成功は内部 buffered write が writer に
  flush されたことのみを示し、file sync/publish を含まない。
- 単一文書系と同じ契約形の convenience:
  `from_reader<T>(reader, options) -> Result<Vec<T>, DocumentError>`
  （全 record を typed で収集）・`to_writer<T: Serialize>(writer,
  records: &[T], options) -> Result<(), DocumentError>`。
- `CsvReadOptions` / `CsvWriteOptions` — `limits`（InputLimits
  byte budget）に加え、`max_record_bytes` / `max_field_bytes` /
  `max_fields` / `max_records`（いずれも有限既定値の hard limit）と
  Delimiter/Quote/Newline(Terminator)/Header/Flexible/Trim/
  Comment/DoubleQuote/Escape/Quoting の Dialect 面と
  `strip_utf8_bom` を公開する。
- `feature = "file"` 併用時の `read_file`/`write_file` — 既存の
  `open_document`/`write_document` helper 上に合成し、
  `DocumentFileReadError`/`DocumentFileWriteError` の失敗軸分離を
  そのまま再利用する。

設計上の責務境界（IO-02 v0.2 合意）:

- `max_record_bytes` / `max_field_bytes` / `max_fields` /
  `max_records` は record/field の **バッファ肥大化前**に適用する
  Hard Limit であり、読取後だけのサイズ検査を採用しない。
- 文字コード/BOM: 入力は ASCII 互換 byte 列として parse し、UTF-8
  BOM は `strip_utf8_bom`（既定 true）で除去。UTF-16 系 BOM 入力は
  `InvalidEncoding` で拒否（decode は提供しない）。
  `field_str`/`read_typed` 経路での非 UTF-8 field は
  `InvalidEncoding`。
- CSV Formula Injection（`=`,`+`,`-`,`@` 始まり field）への
  sanitize/escape はデータを変えるため暗黙に適用しない。必要な
  Export Policy は Consumer 責務として文書化する。
- Backend（`csv`/`csv-core`）の型・Error・feature 名は公開 API に
  露出しない。診断は `Error::source()` の `dyn Error` 経由のみ。

## Why Now

IO-IMP-03 で scaffold の共通契約は確立済み。CSV は AudioLens の
Dataset/artifact 一覧や Synvelis の export 系で実需がある format で
あり、IO-IMP-05 XML に先立ち「単一文書型ではない streaming
format」が共通契約に乗ることを実証する Issue でもある。設計 v0.2
で合意済みの record buffer 再利用・先読み境界・確保前上限の契約を
この Issue で固定する。

## Users and Value

- AudioLens / Synvelis: CSV export・dataset manifest 系の型付き
  record streaming と file publish 合成。
- Ruviform 内部: 大きな CSV を逐次処理する CLI/analysis 用途に
  record buffer 再利用・上限強制済みの安全な入口を提供。

## Scope

### In Scope

- `crates/ruviform-io-structured` への `csv` feature/module 追加:
  `CsvReader`/`CsvRecord`/`CsvWriter`/`CsvReadOptions`/
  `CsvWriteOptions`/`CsvTerminator`/`CsvTrim`/`CsvQuoteStyle`/
  `from_reader`/`to_writer`/（`file` 併用時）`read_file`/
  `write_file` と rustdoc（典型例・制限・non-RT 明記）。
- `DocumentFormat::Csv` variant と、`DocumentPosition` への
  `record`（record index）成分の追加（non_exhaustive struct の
   additive 拡張・既存 component は `None`）。
- Backend: parser engine = `csv-core`（buffer 肥大化前の hard limit
  を実現するため）・record container/serde 接着/writer = `csv`。
  いずれも private 実装詳細。
- 試験: typed round-trip・dialect（TSV/custom quote/CRLF/LF/
  comment/no-quoting）・quoted newline・BOM・非 UTF-8・不正 quote
  の寛容 parse・空行 skip・empty record・巨大 field/record・
  record/field 数上限・非方形（rigid/flexible）・typed error・
  `file` 合成・record buffer 再利用・byte/line/record 位置・
  継続 streaming の増分消費・feature matrix・deterministic fuzz。
- crate README・`crates/README.md` の記述更新。

### Out of Scope

- XML（IO-IMP-05 / Issue #299）。
- 既存 `json`/`yaml`/`toml` feature・`file` feature・
  `ruviform-io-file` の契約変更。
- Consumer 独自 Schema/意味論・Export Policy（formula injection
  対策・NULL 規約・列順序の意味）。
- 新規 CI job/lane の追加（feature matrix 検証は #297 と同様
  auxiliary evidence として実施）。
- CSV 以外の encoding 変換（Shift_JIS・UTF-16 等の decode）。
- Writer 側の byte budget（設計上も要件外 — 出力肥大は
  Serialize 実装と Consumer 責務）。

## Requirements

- `default = []` を維持し、`csv` は他 format feature と独立に
  組合せ可能。`csv` + `file` で `read_file`/`write_file` が有効化。
- MSRV 1.89・edition 2024・`workspace.lints` 継承・unsafe 不使用・
  全公開 enum `#[non_exhaustive]`・全 API 非 RT（crate doc 明記）。
- 依存追加は公開から 7 日以上経過した版・メンテナンス状況・
  ライセンス・依存木サイズを確認して記録する。
- Backend 型・Error・feature 名を公開 API に露出しない
  （serde trait のみ例外）。公開 API 監査を試験/目視で pin する。
- 実現できない制限を黙って成功扱いせず、`Unsupported` /
  `LimitExceeded` / 文書化された制約のいずれかに分類する。
- `DocumentError::limit()` に超過した公開 option 名と上限を保持する
  （`max_bytes`/`max_record_bytes`/`max_field_bytes`/`max_fields`/
  `max_records`）。
- `ruviform-io-file` → `ruviform-io-structured` の逆依存を作らない。

## Acceptance Criteria

- [ ] AC1: `csv` feature で `CsvReader::read_record`/`read_typed`/
  `CsvWriter`/`from_reader`/`to_writer` が typed round-trip する。
  巨大 field/record・quoted newline・CRLF・BOM・不正 quote・
  空 record・複数 dialect を Golden/Negative/deterministic fuzz で
  試験し、期待どおり受理または分類される。
- [ ] AC2: `max_bytes`/`max_record_bytes`/`max_field_bytes`/
  `max_fields`/`max_records` の各上限が `LimitExceeded`（`limit()`
  に名と値）として、肥大化前に強制される。record buffer 再利用・
  offset/record 位置・borrow lifetime・継続 streaming の増分消費
  （入力を全読みせず逐次処理できること）を試験で pin。
- [ ] AC3: typed serde が不可能/不正な record は `DataModel` 等の
  明示エラーとなり、silent truncation がない。非 UTF-8 field は
  `InvalidEncoding`。
- [ ] AC4: 公開 API に backend 型が露出しない。`default`・`csv`
  単独・`csv`+`file`・all-features の feature matrix が
  コンパイル・テストで成立する（workspace task 外の feature 検証は
  auxiliary evidence として記録）。
- [ ] AC5: MSRV 1.89・non-RT・依存 DAG（io-file への一方向）・
  `mise run fmt`/`check`/`lint`/`test` 全緑。streaming の
  benchmark baseline を実測して記録する。

## Deferred Design Concerns

- `read_file` が `Vec<T>` を返す設計のメモリ上限 | Consumer の
  大容量 CSV 実需が確定したら paged/streaming file API を別途検討 |
  巨大入力の `Vec` 収集は `max_records`/`max_bytes` で bound 済みだが
  typed 変換後の heap は advisory。|
- Formula Injection 対策の Export Policy | Consumer 側要件が
  確定してから | 本クレートが値を書き換える既定を持たない。|
- `CsvWriter` の行番号/途中失敗時の部分出力位置 | Writer 側
  position を実需が出てから検討 | 現状は `finish()` 成功のみを
  契約にする。|

## Sources and Relations

- 正本: Issue #298 本文（作業範囲・Acceptance・Done/非対象）。
- 設計基準: Issue #284（IO-02）v0.1/v0.2 合意済み追補（CSV:
  byte 指向 CsvRecord・read_record/read_typed・明示 finish・
  確保前 Hard Limit・Formula Injection は明示 Policy）。
- 依存実装: Issue #297（`ruviform-io-structured` scaffold・
  `DocumentError` 族・`BoundedRead`・`file` feature helper）、
  Issue #295/#296（`ruviform-io-file`）。
- 横断 Gate: Issue #288（IO-06）、Issue #219（Cross-Repo）。
