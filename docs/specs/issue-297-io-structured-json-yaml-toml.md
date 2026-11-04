---
id: "SPEC-034RTbxCxbL7YmxxkxrMmt"
type: "spec"
status: "approved"
title: "Issue #297 ruviform-io-structured — JSON/YAML/TOML Serde型付き Reader/Writer・Strict EOF (IO-IMP-03)"
created: "2026-11-04"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/297"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/219"
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
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/specs/issue-296-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #297 ruviform-io-structured — JSON/YAML/TOML Serde型付き Reader/Writer・Strict EOF (IO-IMP-03)

## What Should Be Built

IO-02（Issue #284）の **v0.1/v0.2 合意済み設計**を実装基準として、
`ruviform-io-structured` クレートを新設し、構造化テキスト Format の
Serde 型付き入出力 API を確立する。本 Issue は同時に当該クレートの
**基底 scaffold Owner** であり、後続の IO-IMP-04 CSV（Issue #298）・
IO-IMP-05 XML（Issue #299）が乗る共通契約（`DocumentError` 族・
`InputLimits`・feature matrix 構成）を汎用な形で確定する。

公開面（Format 別モジュール `json` / `yaml` / `toml`）:

- `from_reader<T: DeserializeOwned>(reader: &mut impl Read, options: &XxxReadOptions) -> Result<T, DocumentError>`
- `to_writer<T: Serialize + ?Sized>(writer: &mut impl Write, value: &T, options: &XxxWriteOptions) -> Result<(), DocumentError>`
- 単一文書の **Strict EOF**: 文書終端後の非空白の余剰データは
  `DocumentErrorKind::TrailingData`。YAML の複数文書入力も単一文書
  API では `TrailingData` として扱う。
- 入力 Byte Budget: `InputLimits::max_bytes` は Reader から実際に読む
  Byte 数の hard limit。超過時は `DocumentErrorKind::LimitExceeded` を
  返し、確保前・解析継続前に拒否する。
- `DocumentError` は `kind()`（`Io`/`InvalidEncoding`/`Syntax`/
  `DataModel`/`TrailingData`/`LimitExceeded`/`Unsupported`）・
  `format()`・判明した場合の位置情報（byte offset / line / column）を
  持つ。Backend の具体 Error 型・Error 列挙は一切公開せず、原因は
  `std::error::Error::source()` で保持する。入力 Snippet や機密値を
  Display に無条件含めない。
- Serde の `Serialize` / `DeserializeOwned` trait だけは公開 API の
  型制約として許可する（設計 v0.1-A の明示合意）。
- TOML は Backend（`toml`）が `&str` 駆動のため「サイズ制限付き
  Buffer 経由」を設計どおりの明示契約とする（Streaming 実装と誤称しない）。
- YAML は `serde-saphyr` の実証可能な Budget（入力 Byte・深さ・
  alias 展開・複数文書）のみを公開制限として採用し、Backend で
  実証できない制限は `Unsupported`／文書化された制約とする。

`file` feature（`ruviform-io-file` への opt-in 依存）:

- Format 別 `read_file<T>` / `write_file<T>` Convenience API。
  `write_file` は `StagedFile::create` → Format Serialize → `seal()` →
  `publish(PublishOptions)` の直列化のみを行い、Format 完了と File
  Publish を別契約として保持する。
- `DocumentFileReadError::{Open(std::io::Error), Decode(DocumentError)}` と
  `DocumentFileWriteError::{Create(FileError), Encode(DocumentError),
  Publish(PublishFailure)}` を分離し、`PublishFailure` の
  `PublishEffect` / `RecoveryInfo` / `RecoveryHandle` を失わない。
- Encode 失敗時は `StagedFile` の Drop による best-effort cleanup に
  委ねる（自動 rollback・追記再開は行わない）。

## Why Now

共通 I/O 基盤チェーン（Issue #282）の構造化データ層。AudioLens の
Manifest・Synvelis の Project Snapshot がいずれも Serde derive 済み
Schema を持ち、IO-01（#295/#296）の staged publish 基盤が既に
利用可能になったため、型付き Read/Write と File Convenience を
確定しないと Consumer 側の I/O Issue（AL #139・SYN #82）が着手
できない。また本クレートの公開 Error/Options/feature 構成は
IO-IMP-04/05 がそのまま利用するため、先に汎用契約を固定する。

## Users and Value

- AudioLens: `Manifest`・`schema.json`・`index.json` 等の読み書き
  （AL-IO-01 / audio-lens #139）。既存 Serde Schema と Golden Byte
  差異は Consumer 側で検証する。
- Synvelis: Project Snapshot・HEAD 保存（SYN-IO-01 / synvelis #82）。
- Ruviform 内部: `ruviform-cli-sample` 等が JSON 出力に利用可能な
  汎用 typed I/O。

## Scope

### In Scope

- `crates/ruviform-io-structured` 新設・Workspace 登録・
  `crates/README.md` のクレート台帳更新。
- 公開契約: `DocumentError` / `DocumentErrorKind` / `DocumentFormat` /
  `DocumentPosition` / `InputLimits` / Format 別 `*ReadOptions`・
  `*WriteOptions` / `DocumentFileReadError` / `DocumentFileWriteError`
  （file feature）。
- `json` / `yaml` / `toml` 各 feature モジュールの
  `from_reader` / `to_writer` /（`file` 併用時）`read_file` /
  `write_file` と rustdoc（典型例・制限・non-RT 明記）。
- Backend: JSON=`serde_json`、YAML=`serde-saphyr`、TOML=`toml`
  （MSRV・ライセンス・メンテナンス・公開 API 非露出を検証済みとする）。
- typed round-trip・否定試験（trailing / invalid / over-budget /
  Unicode / Backend error / 複数文書）・feature matrix
  （`default=[]`・個別 Format・`file` opt-in・all-features）の
  コンパイル検証。

### Out of Scope

- CSV（IO-IMP-04 / Issue #298）・XML（IO-IMP-05 / Issue #299）の
  実装。本 Issue は共通型を汎用に保つ責務のみ。
- 複数文書・長時間接続 Stream の API（別 API として将来分離）。
- Schema versioning / migration / canonical signed bytes / manifest
  semantics（Consumer 責務）。既存 Manifest/Project の
  byte-canonicalization を暗黙に変更しない。
- `Format A → 共通 DOM → B` 変換、FormatRegistry、巨大
  `DocumentValue`、YAML `!include` 実行・外部参照解決・独自 Tag の
  自動処理、TOML コメント・空白レイアウトの Lossless 保持。
- CPU 時間・Consumer 側 Deserialize が確保する Heap の一律厳密保証
  （Advisory のみ）。
- 既存 CI（#219/#294）の再実装、Consumer 独自 Schema の Ruviform
  への移管。

## Requirements

- `default = []`（feature 最小）。feature: `json` / `yaml` / `toml` /
  `file`（`file` は `ruviform-io-file` への path 依存 opt-in）。
- MSRV 1.89・edition 2024・`workspace.lints` 継承・`unsafe` 不使用。
- クレート外から到達可能な全公開 enum に `#[non_exhaustive]`。
- 全 API 非 RT 専用（crate-level doc に明記）。
- 依存追加はメンテナンス状況・ライセンス（MIT OR Apache-2.0）・
  依存木サイズ・公開から7日以上経過のバージョンを選ぶ。
- Backend 具体型・Error・feature 名を公開 API に露出しない
  （Serde trait のみ例外）。公開 API 監査をテストで pin する。
- 制限を強制できない場合は黙って成功扱いせず `Unsupported` /
  `LimitExceeded` の契約とする。`max_bytes` だけで Heap/CPU が
  bounded とは主張しない。
- `ruviform-io-file` → `ruviform-io-structured` 方向の逆依存を作らない
  （依存は structured → io-file のみ、`file` feature 内に閉じる）。

## Acceptance Criteria

- [ ] AC1: `json`/`yaml`/`toml` 各 feature で
      `from_reader`/`to_writer` の typed round-trip が通り、
      trailing / 構文不正 / over-budget / Unicode / 型不一致
      （DataModel）/ Backend error の否定試験が失敗を分類する。
- [ ] AC2: Strict EOF — JSON 後続非空白・YAML 複数文書・TOML は
      `from_str` 全量解析で余剰を検出し、期待どおり
      `TrailingData`/`Syntax` に分類される。
- [ ] AC3: 公開 API（シグネチャ・Error・Options）に Backend 具体型が
      漏出しない（`serde` trait のみ例外）。feature matrix が
      `cargo check` で検証される（`default=[]` / 個別 Format /
      `file` / all-features）。
- [ ] AC4: `file` feature の `write_file` で Serialize 失敗 →
      Encode 分類、Publish 失敗 → `PublishFailure` の
      effect/recovery が保持されて届く。`read_file` の Open 失敗と
      Decode 失敗が分離される。
- [ ] AC5: MSRV 1.89・non-RT・依存 DAG（io-file への一方向）・
      `mise run fmt`/`check`/`lint`/`test` 全緑。既存
      Manifest/Project の byte-canonicalization は変更しない
      （Serializer byte 互換の Golden 検証は Consumer 側責務で
      本 Issue では実施しない）。

## Deferred Design Concerns

- CSV `CsvReader`/`CsvRecord`・XML `EventReader`/`XmlEvent` |
  IO-IMP-04/05 の実装 Issue で確定 | 本 Issue の共通型が実際に
  汎用かはそれらの接続で初めて検証される。|
  共通型が Format 固有に寄りすぎた場合の API 修正リスク。
- JSON/YAML/TOML 以外の入力 Encoding（UTF-16 BOM 等）の明示契約 |
  serde-saphyr は内部で encoding 対応するが JSON/TOML は UTF-8
  前提 | Consumer 実需が固まってから。| 非 UTF-8 入力を
  InvalidEncoding と Syntax のどちらに届けるかの揺れ。
- Byte 互換 Golden 検証（Serializer 出力 byte の回帰）|
  Consumer 側 schema 確定後の Gate（IO-06 #288）|
  Synvelis/AudioLens の移行検証時。| 暗黙の byte 非互換。

## Sources and Relations

- 正本: Issue #297 本文（作業範囲・Acceptance・Done/非対象）。
- 設計基準: Issue #284（IO-02）v0.1/v0.2 合意済み追補・最終整合性
  レビュー追補、Issue #282（統合設計正本）。
- 依存実装: Issue #295/#296（IO-IMP-01/02、`ruviform-io-file` の
  `StagedFile`/`SealedFile`/`PublishReceipt`/`PublishFailure`/
  `RecoveryHandle`）。
- 横断 Gate: Issue #288（IO-06）、Issue #219（Cross-Repo Consumer 影響分類）。
