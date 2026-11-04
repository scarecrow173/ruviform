---
id: "DESIGN-034RTd0bYqWgcxXWa7uVPE"
type: "design"
status: "approved"
title: "Issue #297 ruviform-io-structured — 共通契約・Format別API・Backend選定設計 (IO-IMP-03)"
created: "2026-11-04"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/297"
    - "https://github.com/scarecrow173/ruviform/issues/284"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-structured/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-structured/README.md"
      - type: "file"
        path: "crates/ruviform-io-structured/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/limits.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/input.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/json.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/yaml.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/toml.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/file.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-structured/../README.md"
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
    - "docs/designs/issue-295-io-file-staged-publish.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-297-io-structured-json-yaml-toml.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #297 ruviform-io-structured — 共通契約・Format別API・Backend選定設計 (IO-IMP-03)

## Context

IO-02（Issue #284）の合意済み追補 v0.1/v0.2 が本設計の上位正本。
本 Issue（IO-IMP-03 / #297）は JSON/YAML/TOML の typed I/O と
`ruviform-io-structured` クレートの基底 scaffold を担当し、
CSV（IO-IMP-04 / #298）・XML（IO-IMP-05 / #299）が後から乗る
共通契約（Error 族・InputLimits・feature matrix）を汎用に固定する。

依存は `ruviform-io-structured` → `ruviform-io-file` の一方向のみ
（`file` feature 内に閉じ込め）。逆依存は作らない。全 API 非 RT。

## Scope

### In Scope

- クレート scaffold: `Cargo.toml` feature matrix・crate-level doc・
  `crates/README.md`・workspace `members` 登録。
- 共通契約: `DocumentError` / `DocumentErrorKind` / `DocumentFormat` /
  `DocumentPosition` / `InputLimits` / `DocumentFileReadError` /
  `DocumentFileWriteError`。
- `json` / `yaml` / `toml` Format モジュール:
  `from_reader` / `to_writer` / `*ReadOptions` / `*WriteOptions` /
  （`file` 併用時）`read_file` / `write_file`。
- 入力 Byte Budget の強制機構（private `BoundedRead`）。

### Out of Scope

- CSV / XML モジュール本体・`CsvRecord` / `XmlEvent` 等の型。
- 複数文書 API・長時間 Stream・FormatRegistry・共通 DOM。
- YAML `!include` 解決・外部参照・独自 Tag の自動処理・
  TOML コメント保持・Schema/versioning/migration。

## Components and Boundaries

### クレート内部構造

| モジュール | 責務 | 公開 |
| --- | --- | --- |
| `error` | `DocumentError`/`DocumentErrorKind`/`DocumentFormat`/`DocumentPosition` | 常時 |
| `limits` | `InputLimits { max_bytes }` | 常時 |
| `input` | private `BoundedRead<R>`（byte 計数 + trip フラグ） | 非公開 |
| `json` | serde_json 駆動 typed I/O + `Json*Options` | `feature=json` |
| `yaml` | serde-saphyr 駆動 typed I/O + `Yaml*Options` | `feature=yaml` |
| `toml` | toml 駆動 typed I/O + `Toml*Options` | `feature=toml` |
| `file` | `DocumentFileReadError`/`DocumentFileWriteError` + Format module から共有される staged publish helper | `feature=file` |

### 公開署名（論理形）

```rust
pub fn from_reader<T: serde::de::DeserializeOwned>(
    reader: &mut impl std::io::Read,
    options: &JsonReadOptions, // Format 別
) -> Result<T, DocumentError>;

pub fn to_writer<T: serde::Serialize + ?Sized>(
    writer: &mut impl std::io::Write,
    value: &T,
    options: &JsonWriteOptions,
) -> Result<(), DocumentError>;

#[cfg(feature = "file")]
pub fn read_file<T: serde::de::DeserializeOwned>(
    path: impl AsRef<std::path::Path>,
    options: &JsonReadOptions,
) -> Result<T, DocumentFileReadError>;

#[cfg(feature = "file")]
pub fn write_file<T: serde::Serialize + ?Sized>(
    path: impl AsRef<std::path::Path>,
    value: &T,
    options: &JsonWriteOptions,
    publish: ruviform_io_file::PublishOptions,
) -> Result<ruviform_io_file::PublishReceipt, DocumentFileWriteError>;
```

所有権は Consumer に残る（`&mut` 借用）。`to_writer` 成功は
Serializer が要求する出力を Writer へ渡したことのみを意味し、
`Write::flush`・file sync・publish を含まない。失敗時 Writer は
部分出力を含み得る。

### Error モデル

```rust
#[non_exhaustive]
pub enum DocumentErrorKind {
    Io, InvalidEncoding, Syntax, DataModel,
    TrailingData, LimitExceeded, Unsupported,
}

#[non_exhaustive]
pub enum DocumentFormat { Json, Yaml, Toml } // Csv/Xml は後続 Issue で追加

#[non_exhaustive]
pub struct DocumentPosition { // 判明した成分のみ Some
    pub byte: Option<u64>, pub line: Option<u64>, pub column: Option<u64>,
}

pub struct DocumentError { kind, format, position, source: Option<Box<dyn Error+Send+Sync>> }
```

`Error::source()` に Backend エラーを保持するが、公開型としては
露出しない。Display は kind/format/位置と source メッセージのみで
入力 Snippet を含めない。

### Format 別 Backend → kind 分類

- **JSON** (`serde_json`): `Deserializer::from_reader(&mut bounded)` →
  `T::deserialize` → `de.end()`。
  `Category::Io`→`Io`（trip 時は `LimitExceeded` に再分類）、
  `Category::Syntax`→`Syntax`、`Category::Data`→`DataModel`、
  `Category::Eof`→`Syntax`。`de.end()` の失敗は `TrailingCharacters`
  のみを返す契約のため決定的に `TrailingData`。位置は
  `line()`/`column()` から取得。
- **YAML** (`serde-saphyr`): `from_reader_with_options(&mut bounded,
  Options { budget: Some(budget), ..Default::default() })`。
  `Error::IOError`→`Io`（trip 時 `LimitExceeded`）、
  `Error::Budget`/alias 系→`LimitExceeded`、
  `Error::MultipleDocuments`→`TrailingData`、`Error::Eof`→`Syntax`、
  `Error::SerdeInvalid*`/`Unexpected` 等の型不一致系→`DataModel`、
  `Error::InvalidUtf8Input`→`InvalidEncoding`、
  `Error::ExternalMessage`/`Message` 等のパーサ系→`Syntax`、
  未分類の新 variant（non_exhaustive）→`Syntax`（保守的既定）。
  位置は `location()` の line/column から取得。
- **TOML** (`toml`): `BoundedRead::read_to_string`（UTF-8 検証
  →`InvalidEncoding`）→ 2 相分類: 第 1 相 `toml::from_str::<
  toml::Table>` で文法のみ検証（失敗→`Syntax`、span→byte offset）、
  第 2 相 `toml::from_str::<T>`（失敗→`DataModel`）。「Streaming
  実装」と誤称しないため Buffer 経由であることを rustdoc で明示。
  Writer: `toml::to_string`/`to_string_pretty` → `write_all`。

### Options 型

```rust
#[non_exhaustive] pub struct InputLimits { pub max_bytes: u64 } // Default 16 MiB

#[non_exhaustive] pub struct JsonReadOptions { pub limits: InputLimits }
#[non_exhaustive] pub struct JsonWriteOptions { pub pretty: bool }
#[non_exhaustive] pub struct YamlReadOptions {
    pub limits: InputLimits,
    pub max_depth: Option<usize>,    // → Backend Budget.max_depth
    pub max_aliases: Option<usize>,  // → Backend Budget.max_aliases
}
#[non_exhaustive] pub struct YamlWriteOptions { /* 将来拡張予定の空 struct */ }
#[non_exhaustive] pub struct TomlReadOptions { pub limits: InputLimits }
#[non_exhaustive] pub struct TomlWriteOptions { pub pretty: bool }
```

`max_bytes` 既定値は 16 MiB の有限値。無制限 Default は置かない。
`InputLimits::max_bytes` は `BoundedRead` が強制する唯一の共通
hard limit。それ以外の制限は Backend が実際に強制できる場合のみ
Format 別 Options として公開する（設計 E の「実証できない制限を
黙って無視しない」）。

### `BoundedRead<R>`（private, input.rs）

`Read` を包み、読取 byte を計数。`max` 到達後に underlying が
まだ byte を持つ場合のみ `io::Error`（内部 marker 型
`BudgetExceeded` + trip フラグ）を返す。`max` ちょうどで
underlying が EOF に達した場合は正常 EOF（`Ok(0)`）を返すため、
「文書がちょうど budget サイズ」は成功し、「1 byte でも超過」は
`LimitExceeded` になる。Format module は呼出後
`bounded.was_limit_exceeded()` で trip を検出し、Backend が
`Io` に分類したエラーを `LimitExceeded` へ再分類する。
これを全 Format 共通の単一 authoritative 実装とする。

### `file` feature

```rust
#[non_exhaustive]
pub enum DocumentFileReadError { Open(std::io::Error), Decode(DocumentError) }
#[non_exhaustive]
pub enum DocumentFileWriteError {
    Create(ruviform_io_file::FileError),
    Encode(DocumentError),
    Publish(ruviform_io_file::PublishFailure),
}
```

`write_file` 内部: `StagedFile::create`（失敗→`Create`）→
`to_writer(&mut staged, ..)`（失敗→`Encode`、staged を drop して
best-effort cleanup に委譲）→ `seal()` → `publish(publish)`
（失敗→`Publish`）。`PublishFailure` はそのまま保持し、
`PublishEffect`・`RecoveryInfo`・`RecoveryHandle` を失わない。
`read_file` 内部: `File::open`（失敗→`Open`）→
`from_reader(&mut file, ..)`（失敗→`Decode`）。

## Data and Control Flow

- 読取: `reader` → `BoundedRead`（max_bytes 計数）→ Backend
  parser → `T`。Backend が追加読取を要求して trip → `LimitExceeded`。
- 書込: `T` → Backend serializer → `writer`（JSON/YAML は直接
  Write、TOML は String 経由 `write_all`）。
- file 書込: `value` → staged temp → seal → atomic publish →
  `PublishReceipt`。Encode 失敗時の部分出力は temp のみで
  宛先へ届かない（staged publish の隔離効果）。

## Non-Functional Constraints

- MSRV 1.89・edition 2024・`workspace.lints`・unsafe 禁止・
  全公開 enum `#[non_exhaustive]`・全 API 非 RT。
- `default = []`。`file` は `ruviform-io-file` への optional path
  依存のみで Format feature と独立（`file` 単独でもコンパイル可能）。
- Backend 公開型の漏出禁止（Serde trait のみ例外）。

## Backend 選定

| Format | 採用 | 版 | 公開日 | MSRV | License | 根拠 |
| --- | --- | --- | --- | --- | --- | --- |
| JSON | `serde_json` | `1`（解決: 1.0.154） | 2026-10-11 | 1.71 | MIT OR Apache-2.0 | 標準。streaming `from_reader` + `end()` による trailing 検出 |
| YAML | `serde-saphyr` | `2.0.0` | 2026-10-13 | 1.89 | MIT OR Apache-2.0 | pure Rust・unsafe 禁止・parser Budget が hard limit を実証可能。`serde_yaml`（archived, unsafe-libyaml C 依存）は保守面で不採用 |
| TOML | `toml` | `1.1`（解決: 1.1.8） | 2026-10-09 | 1.85 | MIT OR Apache-2.0 | 標準。`&str` 駆動のためサイズ制限付き Buffer 契約 |

選定は全て公開から 7 日以上経過した版。`serde-saphyr` は
`default-features = false` + `["serialize","deserialize"]` で
comments/include 等の optional 面を切る。`toml` は
`["std","serde","parse","display"]` のみ。`serde` は公開 bound
として必須依存（`default` 常時）。

依存木: `serde-saphyr` は granit-parser・encoding_rs_io 等の
独自 subtree を持つが `file`/`json`/`toml` とは無関係に分離される
（feature 独立）。

## Risks and Trade-offs

- **serde-saphyr `Error` が `#[non_exhaustive]`**: 新 variant は
  wildcard で `Syntax` へ保守的分類。upgrade 時に kind 分類の
  レビューが必要 — 版は `Cargo.lock` で pin し、upgrade は PR で
  明示的に行う。
- **YAML 多文書の検出経路が 2 つ**: `Budget::max_documents` と
  `Error::MultipleDocuments` の双方が機能し得る。どちらが先に
  発火しても `TrailingData`/`LimitExceeded` で受理する試験を
  記述し、分類の揺れを許容せず `MultipleDocuments`→`TrailingData`
  を優先的に確認する。
- **TOML 2 相 parse の二重コスト**: `Table` 経由の文法検証 + typed
  parse で入力を 2 回走査する。文法エラーと型不一致を Backend
  の公開 API で分離する手段が `toml` には存在しないため、正直な
  分類のために許容する（単一文書・budget 済み入力でコストは有界）。
- **`BoundedRead` の byte 計数と Backend の内部 buffer**: serde_json
  IoRead は 1 byte lookahead を行うため「doc ちょうど max + 末尾
  非空白 1 byte」は `LimitExceeded` になり得る（`TrailingData` で
  なく）。入力が budget 超過である事実は正しいため受理する。
- **Write 側の byte budget は提供しない**: 設計では入力 budget のみ
  が要件。出力側の肥大は Serialize 実装と Consumer 責務。

## Deferred Design Concerns

- `CsvReadOptions`/`XmlReadOptions`・`DocumentFormat::{Csv,Xml}`
  variant | IO-IMP-04/05 で確定 | 後続 Issue 着手時 |
  共通型が Format 固有に寄っていた場合の再設計コスト。
- JSON depth の公開制限 | serde_json の固定 recursion limit
  （128）が Backend の実質値であり独自 Budget を実装しない |
  Consumer が深い JSON を扱う実需が発生した場合 |
  `Syntax` と `LimitExceeded` の境界の揺れ。
- YAML `reject_unsupported_tags` 等の tag ポリシー公開 |
  初期既定は backend default（未知 tag の自動処理なし）で様子見 |
  Consumer の tag 利用実需 | 未定義 tag の取り扱い差異。

## Verification Notes

- [ ] `cargo check -p ruviform-io-structured` を
      `default`/`json`/`yaml`/`toml`/`file`/`--all-features` で実行。
- [ ] 公開 API シグネチャに `serde_json`/`serde_saphyr`/`toml`
      の型が現れないことを `cargo public-api` 相当として
      `cargo doc`/`grep` で目視監査（補助証拠。canonical = CI ではない）。
- [ ] 否定試験: trailing / invalid / over-budget / 複数文書 /
      Unicode / 型不一致 / Backend error が期待 kind に分類される。
