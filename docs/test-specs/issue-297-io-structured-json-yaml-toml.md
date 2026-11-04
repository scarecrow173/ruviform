---
id: "TSPEC-034RTeRrELQYdXaCBD66Jy"
type: "test-spec"
status: "approved"
title: "Issue #297 ruviform-io-structured テスト仕様"
created: "2026-11-04"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/297"
  changes: []
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
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #297 ruviform-io-structured テスト仕様

## Purpose

`ruviform-io-structured`（IO-IMP-03 / Issue #297）の公開契約を pin
する: typed round-trip・Strict EOF・入力 Byte Budget・Error kind
分類・Backend 型非露出・file feature の Publish 情報保全・
feature matrix。IO-IMP-04/05 が前提にする scaffold 契約でもある。

## Feature

Format 別 `from_reader`/`to_writer`/`read_file`/`write_file` が
単一文書を Serde 型で往復し、破損・余剰・過大・型不一致・
Backend 由来の失敗を `DocumentErrorKind` に正直に分類する。

## Rules

- Rule 1 (typed round-trip): JSON/YAML/TOML それぞれで
  `to_writer` → `from_reader` が元値を再構成する（構造体・enum・
  入れ子・Unicode 文字列・非 ASCII キーを含む）。
- Rule 2 (Strict EOF): JSON の文書後の非空白 trailing byte は
  `TrailingData`。YAML の `---` 複数文書は `TrailingData`。
  TOML は全量解析で余剰/不正を `Syntax`/`DataModel` に分類する。
- Rule 3 (Byte Budget): `InputLimits::max_bytes` を 1 byte でも
  超過する入力は `LimitExceeded`。ちょうど `max_bytes` の入力は
  成功し得る（境界包含）。
- Rule 4 (kind 分類): 構文不正→`Syntax`、型不一致→`DataModel`、
  非 UTF-8/非対応 encoding→`InvalidEncoding` または `Syntax`
  （Backend が分離しない場合は文書化）、Reader の io 失敗→`Io`、
  backend trip→`LimitExceeded`。
- Rule 5 (Backend 非露出): 公開 API シグネチャ・`source()` chain の
  *型*として `serde_json`/`serde_saphyr`/`toml` の型は露出しない
  （`source()` の `dyn Error` 経由は許容）。`serde` trait のみ公開許可。
- Rule 6 (file feature): `write_file` は Serialize 失敗→`Encode`、
  Publish 失敗→`Publish`（`PublishFailure` の `effect`/`phase`/
  `info`/`take_recovery_handle` が失われない）。`read_file` は
  open 失敗→`Open`、parse 失敗→`Decode`。
- Rule 7 (feature matrix): `default=[]`・`json`/`yaml`/`toml` 単独・
  `file` 単独・`--all-features` でコンパイルが成立する。
- Rule 8 (不変の非破壊): 既存 `ruviform-io-file` 契約・
  Consumer Schema・byte-canonicalization を変更しない。

## Examples

- `json::from_reader` に `{"a":1}garbage` → `TrailingData`。
- `json::from_reader` に `[` 途中 EOF → `Syntax`。
- `yaml::from_reader` に `---\na: 1\n---\nb: 2` → `TrailingData`。
- `max_bytes=8` の `{"a":12345}`（>8 byte）→ `LimitExceeded`。
- `toml::from_reader` に `a = `（文法不正）→ `Syntax`、
  `a = "x"` を `struct{a:u8}` へ → `DataModel`。
- `toml::from_reader` に非 UTF-8 byte → `InvalidEncoding`。
- `json::write_file` で宛先既存 + `PublishMode::CreateNew` →
  `DocumentFileWriteError::Publish`（`effect=NotPublished`・
  `kind=DestinationExists`）。
- Serialize 不能な値（非有限 float を TOML へ等）を `write_file`
  へ → `Encode`（`Publish` でない）。

## Guarantees

- 成功時のみ `Ok`/`PublishReceipt` が返り、未達の保証を偽装しない。
- `Error::source()` は Backend 診断を保持するが公開型としては
  露出しない。Display に入力 Snippet・機密値を含めない。
- `write_file` の Encode 失敗時、staged temp は Drop による
  best-effort cleanup に委譲され、宛先には部分出力が届かない。
- `file` feature は `ruviform-io-file` への一方向依存のみ。

## Non-goals

- CSV/XML・複数文書 API・長時間 Stream・FormatRegistry・共通 DOM。
- Serializer 出力の byte-canonical Golden 検証（Consumer Gate へ）。
- CPU/Heap の一律保証・RT 経路・既存 `io-file` 契約の再検証。
- `cargo public-api` 等の機械的 API 監査ツール導入（本 Issue では
  手書き API 監査試験 + 目視で pin）。

## Risk

- serde-saphyr の `Error` が `#[non_exhaustive]` — wildcard 分類は
  `Syntax`。Backend upgrade で新 variant が増えた場合に kind が
  保守的既定へ落ち得る（version pin + upgrade 時レビューで緩和）。
- `BoundedRead` の trip と Backend lookahead の相互作用で
  `LimitExceeded`/`TrailingData` の境界が入力依存で揺れる —
  いずれも拒否であることを保証し、kind の期待は文書化した経路で pin。
