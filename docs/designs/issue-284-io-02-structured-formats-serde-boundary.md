---
id: "DESIGN-034UydimCfynTXvl5UJT6x"
type: "design"
status: "approved"
title: "Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/plans/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/tasks/issue-284-io-02-structured-formats-serde-boundary.md"
      - type: "doc"
        path: "docs/test-specs/issue-284-io-02-structured-formats-serde-boundary.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/issue-298-io-structured-csv.md"
    - "docs/designs/issue-299-io-structured-xml.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-409-scoped-enumeration.md"
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-284-io-02-structured-formats-serde-boundary.md"
    - "https://github.com/scarecrow173/ruviform/issues/284"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/282"
    - "https://github.com/scarecrow173/ruviform/issues/283"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/285"
    - "https://github.com/scarecrow173/ruviform/issues/286"
    - "https://github.com/scarecrow173/ruviform/issues/287"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
    - "https://github.com/scarecrow173/ruviform/issues/297"
    - "https://github.com/scarecrow173/ruviform/issues/298"
    - "https://github.com/scarecrow173/ruviform/issues/299"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/409"
    - "https://github.com/scarecrow173/audio-lens/issues/139"
    - "https://github.com/scarecrow173/audio-lens/issues/143"
    - "https://github.com/scarecrow173/synvelis/issues/82"
    - "https://github.com/scarecrow173/synvelis/issues/84"
    - "https://github.com/scarecrow173/synvelis/issues/25"
  defers: []
  deferred-by: []
---

# Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計

## Context

Issue #284 は、独立クレート `ruviform-io-structured` の
Format 別 API・Serde 公開境界・Backend 隠蔽・resource bounds・
error mapping・Feature/DAG を実装前に確定する IO-02 の設計
Issue である（親 #282）。Issue 本文の「合意済み設計 v0.1・
v0.2（2026-11-04）」と「最終整合性レビュー — 実装 Issue への
引継ぎ」が要求の正本であり、本書はそれを docs チェーンの
正本設計書として文書化する。

**本書が固定するもの / 固定しないもの**

- 固定する: Format 横断の Serde 公開境界・driver/backend
  中立契約、Format/Version 契約の骨格（API 族・単一文書
  Strict EOF・Consumer 所有の schema/version/canonical
  bytes）、Resource Budget の 3 層モデル、Error 契約の語彙、
  Cargo feature/Backend 選定の確定値、Consumer 利用面、
  証拠項目（OWN/CSV/XML/DOC/ERR/COMP）と他 IO gate への
  対応索引。
- 固定しない: per-format の詳細 API・内部 buffer 戦略・
  test 規則の実装詳細（#297/#298/#299 の設計・test-spec が
  正本）、実走テスト結果、Consumer repo の実測、backend の
  将来更新。

**実装の現状（2026-11-09 時点の snapshot）**

- `crates/ruviform-io-structured` は main に存在し、
  #297（JSON/YAML/TOML・PR #354）・#298（CSV・PR #361）・
  #299（XML・PR #363）で merge 済み。`json`/`yaml`/`toml`/
  `csv`/`xml`/`file` の feature matrix が `Cargo.toml` に
  定義され、`mise run test-io-structured` が format feature
  matrix の検証入口である。
- 合成 E2E クレート `ruviform-io-e2e`（#306）が Structured→
  StagedFile 境界（serialize→seal→publish→再 Open→strict
  EOF 拒否・失敗軸分離）を実 OS temp dir で検証する。
- IO-06 cross-consumer gate 設計（#288）が IO-02 を
  matrix 行 S-N1〜S-X6 として受入対象に含めている。
- Consumer 側: AudioLens #139/#143 は CLOSED、Synvelis
  #82 は CLOSED・#84 は OPEN・#25 は CLOSED（Issue 状態は
  変動し得る snapshot として記録）。

文書チェーン: [spec](../specs/issue-284-io-02-structured-formats-serde-boundary.md)・
[plan](../plans/issue-284-io-02-structured-formats-serde-boundary.md)・
[test-spec](../test-specs/issue-284-io-02-structured-formats-serde-boundary.md)・
[task](../tasks/issue-284-io-02-structured-formats-serde-boundary.md)。

## Scope

### In Scope

- Serde 公開境界・driver/backend 中立契約の正本記述。
- Format/Version 契約（API 族・単一文書・Strict EOF・
  Consumer 所有の schema/version/canonical bytes）。
- Resource Budget の 3 層モデル（hard limit / parser
  budget / advisory）と「保証できない制限を偽装しない」
  規則。
- `DocumentError` 系の Error 契約と File Convenience の
  失敗軸分離。
- Cargo feature・Backend 選定の確定値（採用版・MSRV・
  ライセンス・feature 構成）。
- AudioLens/Synvelis の利用面と責務分界。
- 証拠項目（OWN/CSV/XML/DOC/ERR/COMP・Design Acceptance
  Gate）から既存 test-spec・#288 matrix・実装 Issue/PR
  への対応索引。
- 設計合意と実装/検証の分離の明記。

### Out of Scope

- per-format 詳細設計の再定義（#297/#298/#299 が正本）。
- コード変更・テスト追加・CI 変更。
- Consumer repo の変更・実測（#219/#294 consumer-verify、
  AL #139/#143、SV #82/#84/#25 が Owner）。
- Format A→共通 DOM→Format B の自動変換、Schema/
  version/migration/canonical signed bytes の Ruviform 側
  実装。
- 新規 CI Workflow・証拠スキーマ・Dashboard。

## Components and Boundaries — 責務分担

| 領域 | Owner | 本書との関係 |
| --- | --- | --- |
| IO-02 横断契約（Serde 境界・Format/Version 骨格・Budget 3 層・Error 語彙・Feature/DAG・Consumer 面・証拠索引） | **Issue #284（本書）** | 正本 |
| JSON/YAML/TOML の typed I/O・共通 scaffold（Error 族・InputLimits・BoundedRead・file 合成） | **#297**（`docs/designs/issue-297-…`） | 詳細正本。本書は横断契約のみ所有 |
| CSV Record streaming・Dialect・typed Serde | **#298**（`docs/designs/issue-298-…`） | 同上 |
| XML Event streaming・Namespace・secure default | **#299**（`docs/designs/issue-299-…`） | 同上 |
| File 公開契約（StagedFile type-state・PublishOptions・Receipt/Failure・PlatformFileOps） | **#295**（`docs/designs/issue-295-…`） | `file` feature が依存する下位契約 |
| 実 OS backend（renameat2/renamex_np/MoveFileExW 等）・OS/FS capability | **#296**（`docs/designs/issue-296-…`） | publish の OS 保証範囲の正本 |
| 旧 `StagedOutput`/旧 `PublishOptions` 撤去・WAV→StagedFile 移行 | **#305** | IO-02 以前の Media I/O 境界。本書は触れない |
| 合成 E2E（層横断境界試験） | **#306**（`docs/designs/issue-306-…`・`ruviform-io-e2e`） | Structured→StagedFile 境界の実 OS 証拠 |
| Rooted capability（`RootedDir`/`open_scoped_root`/bounded 列挙） | **#409**（`docs/designs/issue-409-…`） | path 規則の下位機構。本書は scoped I/O を所有しない |
| Cross-consumer/OS-FS/Security/性能/互換性 Gate | **#288**（`docs/designs/issue-288-…`） | 本書の証拠項目が同 gate の IO-02 matrix 行の根拠 |
| Cross-Repo CI 運用 | **#219/#294** | Consumer tuple 検証の実行機構 |
| Consumer 固有の Schema/Project/Dataset 意味論・Golden・復旧 | **AL #139/#143、SV #82/#84/#25** | Consumer 側の受入 Owner |

原則: 本書は **横断契約の定義** を所有し、**per-format 詳細**
は #297/#298/#299、**File/OS 保証**は #295/#296、**実 OS
境界証拠**は #306、**採否判定**は #288、**実行機構**は
#219/#294 に委ねる。責務の重複定義をしない。

## Serde 公開境界と Driver/Backend 中立契約

Issue #284「採用済み設計」A・v0.2-6 を正本として固定する。

### 許可される公開外部型

- `serde::Serialize` / `serde::de::DeserializeOwned` — 公開
  API の型制約（bound）としてのみ使用。Consumer の既存
  derive/schema をそのまま再利用できる。
- `std::io::{Read, Write, BufRead}`、`std::path::Path` —
  標準 trait/型。
- Ruviform 独自型: `DocumentError`/`DocumentErrorKind`/
  `DocumentFormat`/`DocumentPosition`/`DocumentLimit`/
  `InputLimits`/Format 別 `*Options`/`CsvRecord`/`CsvReader`/
  `CsvWriter`/`XmlEventReader`/`XmlEventBuffer`/`XmlEventRef`/
  `XmlEvent` 等の `*Ref`/`*Owned` ペア/`DocumentFileReadError`/
  `DocumentFileWriteError`。

### 非公開とするもの

- Backend の公開型・Error・feature 名・version:
  `serde_json::Value`/`serde_json::Error`、serde-saphyr の
  `Error`/`Yaml` 型、`toml::Value`/`toml::de::Error`、
  `csv::ByteRecord`/`csv::StringRecord`/`csv::Error`、
  `quick_xml::` の Reader/Event/encoding 型等は公開 API に
  原則露出させない。`Cargo.toml` の comment がこの契約を
  明記している。
- Backend 診断は `DocumentError::source()`（
  `Box<dyn Error + Send + Sync>`）経由の `dyn Error` として
  のみ到達可能。`DocumentError` の `Display` は kind/format/
  position/limit のみを出し、backend レンダリング文字列や
  入力 snippet を埋め込まない（`error.rs` の module doc が
  正本）。
- 自作の Serializer trait・derive macro・JSON DOM・
  `DocumentValue`・FormatRegistry・同一 Reader trait への
  強制は行わない（Issue #284 明記）。Format 間で API 形が
  異なることを許容する。

### Driver 中立性

公開 API は特定 backend の「driver」（parser/serializer の
具象実装）に依存しない。backend 差し替えは `Cargo.toml` の
optional 依存と module 内部に閉じ、公開シグネチャ・Error
語彙・Options 型は Ruviform 型のみで構成される。Serde trait
は唯一の例外として意図的に公開する（Consumer の derive
資産を再利用するため — Issue #284「Serde の公開依存は
明示的に許可」）。

### 検証根拠

backend 型非露出は per-format test-spec の audit 規則で
pin されている: `issue-297` Rule 5、`issue-298` S8、
`issue-299` S7（公開 signature に backend 型名がないことの
目視/grep 確認）。#288 matrix 行 S-X6 が同契約を gate
項目として参照する。

## Format/Version 契約

### API 族（実装済みの確定形）

| Format | feature | API 族 | 正本 |
| --- | --- | --- | --- |
| JSON | `json` | `json::from_reader<T>`/`to_writer<T>`（単一文書・strict EOF）+ `read_file`/`write_file`（`file` 併用時） | `docs/designs/issue-297-…`・`crates/ruviform-io-structured/src/json.rs` |
| YAML | `yaml` | `yaml::from_reader<T>`/`to_writer<T>`（単一文書・複数文書は `TrailingData`）+ file convenience | 同上 `yaml.rs` |
| TOML | `toml` | `toml::from_reader<T>`/`to_writer<T>`（有限 buffer parse・2 相分類）+ file convenience | 同上 `toml.rs` |
| CSV | `csv` | `CsvReader`/`CsvRecord`/`CsvWriter` の record streaming + `read_typed`/`write_typed` + `from_reader`/`to_writer` + file convenience | `docs/designs/issue-298-…`・`src/csv.rs` |
| XML | `xml` | `XmlEventReader<R: BufRead>` + 再利用 `XmlEventBuffer` + 借用 `XmlEventRef<'a>`（`to_owned()` で所有 event）+ file convenience の owned event 収集 | `docs/designs/issue-299-…`・`src/xml.rs` |

- Borrow により入力/出力 stream の所有権は Consumer に残る
  （`&mut impl Read`/`&mut impl Write`/`&'a mut R`）。
- `to_writer`/`write_record`/`finish` の成功は「serializer
  が要求する出力を writer へ送った」ことのみを意味し、
  `Write::flush`・file sync・publish を含まない。失敗時
  writer は部分出力を含み得る。自動 rollback/追記再開は
  保証しない（Issue #284 v0.2-1）。

### 単一文書契約（Strict EOF）

- JSON: 文書後の非空白 byte は `TrailingData`（serde_json の
  `Deserializer::end()` で決定的に検出）。
- YAML: 複数文書は `TrailingData`（`Error::MultipleDocuments`
  を優先分類）。
- TOML: 有限文書全体を parse するため「文書後の余剰」と
  「文法違反」を区別できず `Syntax` に分類する — 正直な
  分類として文書化済み（crate README）。
- CSV/XML は有限文書ではなく record/event 列であり、
  EOF は `Ok(false)`/`Ok(None)` の正常終端。XML は EOF 後
  の再呼出しも `Ok(None)`（done latch）、エラー後の
  reader は poison される。
- 「文書完成・文法的終端・余剰データ拒否・入力 budget 内
  での終了」が単一文書 API の基本契約（Issue #284 v0.2-4）。
  長時間接続 stream・複数文書 API は別 API に分離する
  （初期 API では提供しない）。

### Format/Version・Schema の所有

- schema/version/migration/canonical signed bytes/manifest
  semantics は **各 Consumer が所有**する。Ruviform は
  型無しの構造 I/O プリミティブのみを提供し、既存
  Manifest/Project schema を移動しない（crate README
  Non-goals・Issue #284 G）。
- `Format A→共通 DOM→Format B` の自動変換を提供しない
  （意味が失われる変換を基盤が肩代わりしない）。
- Serde deserialize できても既存 Serializer の byte 列と
  同一とは限らない — byte 互換が必要なら Consumer 側の
  golden test で担保する（crate README Non-goals）。
- これは #219 consumer-compat-contract の影響分類
  （`impacted`/`no impact`/`unknown`）と整合する:
  Ruviform 側の typed I/O 追加は Consumer の schema 互換を
  変更しないが、Consumer が採用する場合の byte 差異は
  Consumer 側 golden で判定する。

### Format 別の正直な制約（実装済みの確定値）

| Format | 保証するもの | 保証しないもの（正直な記述） |
| --- | --- | --- |
| JSON | strict EOF・`max_bytes`・serde 型付き round-trip | depth は serde_json 固定内部上限（超過は `Syntax` であり `LimitExceeded` に偽装しない）・pretty 出力の byte 互換は Consumer 責務 |
| YAML | `max_bytes`・`max_depth`（既定 64）・`max_aliases`（既定 50,000）等 backend 実 budget・複数文書拒否・未対応 tag は `Unsupported` | include/property 等の外部 resolver は既定で無効・公開しない。CPU/deserializer heap は非保証 |
| TOML | `max_bytes`・UTF-8 必須・2 相分類（文法 `Syntax` / 型 `DataModel`）・pretty 出力 | streaming ではない（有限 buffer parse — 「Streaming 実装」と誤称しない）。コメント・空白レイアウトの lossless 維持は非対象 |
| CSV | record 単位 streaming・`CsvRecord` 再利用・`max_bytes`/`max_record_bytes`/`max_field_bytes`/`max_fields`/`max_records` の確保前 hard limit・`CsvDialect` 検証（矛盾設定は `Unsupported`）・BOM policy・UTF-16 系拒否 | grammar は permissive（未閉鎖 quote 等は backend 契約どおり受理 — module 文書に明記）。formula injection の sanitize は行わない（Consumer の Export Policy） |
| XML | event 単位 streaming・namespace 即時解決（default ns は unprefixed attribute に非適用）・`Start`/`Empty`/`End`/`Text`/`CData`/`Comment`/`Declaration`/`ProcessingInstruction`/`Reference` の区別保持・borrowed event + `to_owned()`・DTD/外部 entity/未定義 entity/非 UTF-8 encoding を既定拒否・`max_depth`/`max_attributes`/`max_event_bytes`/`max_events`/`max_namespace_bindings` | `max_event_bytes` は event 公開時検査であり backend 内部 buffer の事前確保 cap ではない（正直に文書化）。writer/serde typed/XSD 等 schema validation は非対象。document 構造検査（単一 root 等）は backend 忠実の範囲外 |

## Resource Budget 契約（3 層モデル）

Issue #284 v0.2-4「Resource Budget と Single Document 契約」
を正本として固定する。

### Hard Limit（実際に強制される上限）

- `InputLimits::max_bytes`（既定 `DEFAULT_MAX_INPUT_BYTES` =
  16 MiB・有限・無制限 Default なし）— private
  `BoundedRead<R>` が全 format の `from_reader` で共有する
  唯一の authoritative 実装。上限ちょうどは成功し得、
  +1 byte は `LimitExceeded`（`DocumentError::limit()` に
  `DocumentLimit{name:"max_bytes", max}` を保持）。
  `metadata().len()` の事前確認のみを hard limit としない。
- Format 別: YAML `max_depth`/`max_aliases`、CSV
  `max_record_bytes`/`max_field_bytes`/`max_fields`/
  `max_records`、XML `max_depth`/`max_attributes`/
  `max_event_bytes`/`max_events`/`max_namespace_bindings` —
  いずれも backend が実際に強制できる範囲で公開し、超過は
  `LimitExceeded`。CSV の record/field 上限は `csv-core`
  の pull 型 parser により **バッファ肥大化前**に適用する
  （`csv` crate 単体の `expand_fields` では保証できない
  ため — #298 設計）。
- XML `max_event_bytes` は「event 公開時の検査」であり、
  backend がその byte 数を内部 buffer 済みの可能性を
  正直に文書化する（`src/xml.rs`・crate README）。

### Parser Budget（backend 保証範囲の検証済み公開）

- YAML の alias/node/scalar/depth budget、XML の depth/
  attribute/event、JSON depth（backend 固定値）等は、
  backend の実際の保証範囲を検証したうえで公開する。
  未対応の制限を黙って無視しない — 実装できない制限は
  Options に出さず、必要なら `Unsupported` で報告する
  （例: `delimiter == quote` の矛盾 dialect）。

### Advisory（厳密保証を主張しない）

- CPU 時間、Consumer 独自 `Deserialize` 実装が確保する任意
  heap、無限 stream の終了は一律の厳密保証として主張しない
  （crate-level doc・`limits.rs` module doc が正本）。
  byte 上限は「入力サイズの上限」であり解析コストの保証
  ではない。
- `LimitNotGuaranteed` 相当の設計意図: 実装が強制できない
  budget を公開 Options として約束しない。公開された
  Option は全て実強制または `Unsupported`/`LimitExceeded`
  のいずれかで振る舞う。

## Error 契約

### `DocumentError` 系（format 横断の正規化）

- `DocumentErrorKind`: `Io`/`InvalidEncoding`/`Syntax`/
  `DataModel`/`TrailingData`/`LimitExceeded`/`Unsupported`
  （`#[non_exhaustive]`・match 可能な安定語彙）。
- `DocumentFormat`: `Json`/`Yaml`/`Toml`/`Csv`/`Xml`。
- `DocumentPosition`: `byte`/`line`/`column`/`record` —
  backend が実際に報告した成分のみ `Some`。根拠のない
  位置・分類の推測はしない。
- `DocumentLimit`: `LimitExceeded` 時の上限名（公開 option
  名）と実際の上限値。帰属できない backend 内部制限は
  `None`。
- `source()`: backend 診断を `Box<dyn Error + Send + Sync>`
  として保持。`Display`/`Debug` は backend 文字列・入力
  snippet を含めない（機密データの診断漏出防止 — #288
  否定系判定表・安全性行と整合）。

### File Convenience の失敗軸分離

`feature = "file"`（`ruviform-io-file` への optional path
依存）で `read_file`/`write_file` を提供:

- `DocumentFileReadError`: `Open(io::Error)` /
  `Decode(DocumentError)`。
- `DocumentFileWriteError`: `Create(FileError)` /
  `Encode(DocumentError)` / `Publish(PublishFailure)`。
- `Publish` variant は `PublishFailure` 全体 —
  `PublishEffect`・phase・durability evidence・
  `RecoveryInfo`・`RecoveryHandle` — を失わず保持する
  （`src/file.rs` が正本）。Encode 失敗時は staged temp を
  drop して best-effort cleanup し、destination は一切
  publish されない（部分文書が宛先へ届かない）。
- 「Format 書込み失敗」と「Publish 失敗」を混同しない —
  Issue #284 v0.2-5・ERR-01/02 の要件。

## Cargo Feature・Backend 選定（確定値）

### Feature matrix

`default = []`。format feature と `file` は独立に組合せ可能
（`file` 単体でも共通エラー型群は利用可。format module の
`read_file`/`write_file` は各 format feature + `file` の
両方が有効なときのみ存在）。

| feature | module | backend（private・`=` pin） |
| --- | --- | --- |
| `json` | `json` | `serde_json =1.0.154` |
| `yaml` | `yaml` | `serde-saphyr =2.0.0`（`default-features=false` + `serialize`/`deserialize`） |
| `toml` | `toml` | `toml =1.1.8+spec-1.1.0`（`std`/`serde`/`parse`/`display`） |
| `csv` | `csv` | `csv =1.4.0` + `csv-core =0.1.13`（共に `default-features=false`） |
| `xml` | `xml` | `quick-xml =0.42.0`（`default-features=false`） |
| `file` | `file` | `ruviform-io-file`（path・optional） |

`serde = "1"` は非 optional（公開 bound として必須）。
backend は全て `=` pin — 7 日 release-age ポリシーと
`Cargo.lock` 追跡による固定（`Cargo.toml` comment が正本）。

### Backend 選定表（Issue #284 Design Acceptance Gate 対応）

| Format | 採用 | 解決版 | 公開日 | MSRV | License | 選定根拠 |
| --- | --- | --- | --- | --- | --- | --- |
| JSON | `serde_json` | 1.0.154 | 2026-10-11 | 1.71 | MIT OR Apache-2.0 | 標準。streaming `from_reader` + `end()` による trailing 検出（#297 設計） |
| YAML | `serde-saphyr` | 2.0.0 | 2026-10-13 | 1.89 | MIT OR Apache-2.0 | pure Rust・unsafe なし・parser Budget が hard limit を実証可能。`serde_yaml`（archived・unsafe-libyaml C 依存）は保守面で不採用（#297 設計） |
| TOML | `toml` | 1.1.8+spec-1.1.0 | 2026-10-09 | 1.85 | MIT OR Apache-2.0 | 標準。`&str` 駆動のためサイズ制限付き Buffer 契約（#297 設計） |
| CSV | `csv` + `csv-core` | 1.4.0 / 0.1.13 | — | 1.73 / 未宣言 | Unlicense/MIT | `csv-core` の pull 型 parser で確保前 hard limit を実現。`csv` が record 組立て・Serde 接着・Writer を担当（#298 設計） |
| XML | `quick-xml` | 0.42.0 | 2026-08-22 | 1.86 | MIT | borrowed event + `NsReader` namespace 解決 + `GeneralRef` event・`set_max_namespace_bindings` 等の既存 limit（#299 設計） |

全 backend の MSRV は workspace MSRV 1.89 以下で、workspace
toolchain で検証済み。保守状況・ライセンス・依存木サイズ・
7 日 release-age は `Cargo.toml` の pin comment と各 per-format
設計の「Backend 選定」節が正本。security advisory の継続監視は
dependency audit lane（#288 gate の Cargo feature/OS/MSRV/
license/security audit 項目）に接続する。

### 依存 DAG・RT 制約

- `ruviform-io-structured` → `ruviform-io-file` は `file`
  feature 内の optional path 依存のみ。`ruviform-io-file`
  から Structured への逆依存は作らない（循環防止）。
- media-io / domain / engine / DSP / RT core への依存は
  持たない。Parser/Compression/Archive・ブロッキング I/O を
  RT コアへ混入しない（DAG 正本:
  `docs/designs/ruviform-architecture-design-baseline.md`・
  各 crate README）。
- 全 API 非 RT — I/O にブロックし alloc も行う。RT
  オーディオコールバックからは絶対に呼ばない
  （crate-level doc が正本）。

## Consumer 利用面（AudioLens / Synvelis）

Issue #284 G「Cross-Consumer」・v0.1/v0.2-6 を正本として
固定する。Consumer の schema/version/migration/canonical
signed bytes/manifest semantics は各 Consumer が所有し、
Ruviform はそれらを移動しない。

### AudioLens（AL-IO-01 #139・AL-IO-02 #143 — ともに CLOSED）

- 利用面: `Manifest`・`schema.json`・`index.json` 等の
  Serde 型付き JSON 読み書き。Consumer 側の既存 derive
  schema を `json::from_reader`/`to_writer`（必要なら
  `file` feature の `read_file`/`write_file`）で再利用する。
- 検証観点: Job 再開・Artifact hash・数値精度（f64 等の
  serde 表現差異）・既存 Serializer の Golden Byte 差異は
  AL 側 golden test で担保。Dataset の独自 binary chunk
  意味論は IO-02 の移管対象外。
- `write_file` の `Publish` 失敗は `PublishFailure` の
  effect/recovery を保持したまま AL の job error へ写像
  できる（`DocumentFileWriteError::Publish` が情報を失わ
  ない — `file.rs` が正本）。

### Synvelis（SYN-IO-01 #82 — CLOSED・SYN-IO-02 #84 — OPEN・#25 — CLOSED）

- 利用面: Project Snapshot の Serde 型の読み書き。旧
  ファイルの読み込みと新 Serializer の互換性は SV 側の
  schema/version 責務として扱い、Ruviform は typed I/O の
  みを提供する。
- 検証観点: Revision/HEAD/Recovery の責務分離 — Ruviform
  単一 File Publish は Project 全体の Transaction や HEAD
  の CAS とみなさない（#288 設計の Consumer 要件と同一
  原則）。Snapshot schema の version/migration は SV が所有。
- WAV PCM/Riff finalize/再 Open（#25 CLOSED の既存 E2E）
  は Media I/O（#305）側の責務であり IO-02 の対象外。

### Consumer 独立判定

#219/#288 の契約どおり、同一候補 Ruviform Revision でも
AudioLens と Synvelis は独立して判定する。IO-02 の採用は
consumer-verify.yml（#294）の候補 tuple lane で実測され、
`impacted`/`no impact`/`unknown` の影響分類と
`pass`/`fail`/`blocked`/`not run`/`unsupported` の Gate
結果で記録する。本書は要件の定義のみで、実施は各
Consumer repo が Owner。

## File Convenience と IO-01/#409 境界

- Low-level API（`from_reader`/`to_writer`/`CsvReader`/
  `XmlEventReader`）は `std::io` のみで完結し、IO-01 未
  実装でも Codec/Parser の設計・テストが可能な構成
  （Issue #284 採用済み設計）。`file` feature はその上に
  `StagedFile`→`seal()`→`publish(PublishOptions)` を合成
  する任意依存であり、依存方向は `io-structured` →
  `io-file` の一方向のみ。
- `read_file`/`write_file` の `path` は `impl AsRef<Path>`
  の caller 指定パス。**scoped/rooted な path 規則は
  IO-02 の責務ではない** — `ruviform-io-file` の
  `RootedDir`/`open_scoped_root`/bounded 列挙（#409）や
  Consumer 側の path policy が所有する。IO-02 は rooted
  capability を再実装せず、`StagedFile` の公開契約経由で
  publish の OS 保証（#295/#296）をそのまま利用する。
- Archive/Compression の finish 成功も Consumer/組合せ側が
  確認する — `write_file` が保証するのは Format encode →
  seal → publish の完了であり、上位層の finish を代替
  しない（#306 合成 E2E の層責務と整合）。

## 証拠項目対応（Evidence Mapping）

Issue #284 の OWN/CSV/XML/DOC/ERR/COMP 証拠項目と Design
Acceptance Gate を、既存の正本（per-format test-spec・
合成 E2E・#288 matrix・実装 Issue/PR）へ対応付ける索引。
本書は参照のみを持ち、証拠内容を複写しない。

### 証拠項目 → 正本対応表

| 証拠項目（Issue #284 v0.2-6） | 内容 | 正本（test-spec / gate / 実装） |
| --- | --- | --- |
| OWN-01/02 | Borrowed Reader/Writer・Streaming 終了と部分出力・所有権/メモリ解放 | `issue-297` Rule 1/6、`issue-298` S1–S4/S5、`issue-299` S1–S5、#288 matrix S-N4 |
| CSV-01/02/03 | Record 再利用・確保前 Hard Limit・Dialect/Serde 列対応と Fuzz | `issue-298` G1–G13/N1–N15/S1–S8（deterministic fuzz S6） |
| XML-01/02/03 | Namespace/属性/QName・Borrowed Event/Buffer 安全性・DTD 拒否/Depth/巨大 Event Fuzz | `issue-299` G1–G14/N1–N19/S1–S9（deterministic fuzz S6） |
| DOC-01/02 | 単一文書終端・YAML Alias/Tag Budget・TOML 有限 buffer | `issue-297` Rule 2/3/4（Strict EOF・Byte Budget・kind 分類） |
| ERR-01/02 | Encode・Publish 失敗の直交・RecoveryHandle 非喪失・機密データ診断漏出防止 | `issue-297` Rule 6、`issue-298` S5/N14/N15、`issue-299` S5/N19、`issue-306` R3（実 OS CreateNew 競合で `PublishFailure` effect/recovery 保持を実演） |
| COMP-01/02 | AudioLens/Synvelis の Golden・互換性（精度/速度/Feature）を別々に合格 | #288 matrix S-N1〜S-X6・Consumer 非回帰判定表・#219/#294 consumer-verify・AL #139/#143・SV #82/#84/#25 |

### Design Acceptance Gate → 対応

| Gate 項目（Issue #284） | 対応する正本 |
| --- | --- |
| JSON/YAML/TOML/CSV/XML ごとの Consumer コード・Read/Write/Streaming API 案 | 実装済み API（`src/{json,yaml,toml,csv,xml}.rs`）+ per-format 設計（#297/#298/#299） |
| Public Rust API に具体 Parser 型を露出させない Audit（Serde trait のみ例外） | `issue-297` Rule 5・`issue-298` S8・`issue-299` S7・#288 matrix S-X6・本書「Serde 公開境界」節 |
| バックエンド選定表（MSRV 1.89/ライセンス/メンテ/セキュリティ/Feature） | 本書「Backend 選定表」+ 各 per-format 設計「Backend 選定」節 |
| Low-level と File Convenience を分離した compile/feature/DAG 検証計画 | 本書「Cargo Feature・Backend 選定」DAG 節・`mise run check-io-structured`/`lint-io-structured`/`test-io-structured`（format feature matrix）・`issue-297` Rule 7 |
| 破損/過大/複雑/非対応/unknown field/end-of-input・Round-trip・精度・文字コード試験 | `issue-297` Rule 1–4、`issue-298` G/N/S 系、`issue-299` G/N/S 系、#288 matrix S-X1〜S-X5 |
| 両 Consumer の Manifest/Project schema を Ruviform に移動せず利用可能 | 本書「Consumer 利用面」節・crate README Non-goals・AL #139/#143・SV #82/#84 |
| Format ごとの設計成果と実装 Issue 分解 | #297/#298/#299 の docs チェーン（spec/design/plan/test-spec）+ 本書の対応索引 |

### #288 gate との接続

#288（IO-06）の受入 matrix は IO-02 を次の行で参照する
（`docs/designs/issue-288-…` が正本）:

- S-N1〜S-N4（正常系: typed round-trip・CSV/XML streaming・
  file 合成）→ `issue-297`/`298`/`299` test-spec の G/Rule 系。
- S-X1〜S-X6（否定系: 構文/TrailingData/型不一致・過大
  budget・CSV/XML 個別否定・backend 型非露出）→ 同 test-spec
  の N/S 系と本書の Error/Budget/Serde 境界節。

IO-02 単体の実走証拠は `mise run test-io-structured`
（format feature matrix: default・`xml`・`xml,file`・
`--all-features`）と `cargo test --workspace`（doc test 含む）
が入口。Consumer tuple の採否判定は #288 gate と
consumer-verify（#219/#294）が Owner。

## Design Done / Implementation Done の分離

- **本 slice（docs）**: 横断契約・Format/Version 骨格・
  Budget 3 層・Error 語彙・Feature/Backend 確定値・Consumer
  面・証拠索引の **設計正本の文書化**。実装は #297/#298/#299
  で merge 済みだが、本書の存在は「設計が文書化された」
  ことのみを意味し、Consumer 実測・OS 別実機・性能比較の
  完了を意味しない。
- **残る検証（実装/受入段階）**: Consumer 側の Golden・
  byte 互換・性能の独立検証（AL/SV Issue・consumer-verify
  tuple）、macOS/Linux 実機での file feature 経路、CI lane
  の継続監視。未実施は `not run`/`NotRun`/`Unsupported` と
  記録する。
- Issue #284 の受入条件のうち「API 署名/所有権が実際に
  利用可能」「Public API 監査」「Limits/security の実強制」
  「Error/Publish 連携」「feature/OS/MSRV/license/security
  audit と nonRT DAG」は #297/#298/#299 の実装と test-spec
  で既に検証済みの範囲を指し、本書はその正本への索引を
  提供する。「両 Consumer 回帰」は cross-consumer gate
  （#288・#219/#294）の実走段階の要件であり、本 slice の
  対象外。

## Risks and Trade-offs

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | 横断契約と per-format 正本の記述が drift する | med | med | 本書は骨格と索引のみ所有し、詳細は #297/#298/#299 へ参照。test-spec の audit 規則が backend 非露出・budget・error 分類を pin |
| 2 | 「byte 上限があるから heap/CPU も bounded」と誤読される | med | high | Advisory 節で CPU/deserializer heap/無限 stream を非保証と明記。crate README・`limits.rs` doc と同一文言 |
| 3 | Consumer が schema/version を Ruviform に移そうとする | low | med | Consumer 利用面節で schema/version/migration/canonical bytes の Consumer 所有を明記。crate README Non-goals と一致 |
| 4 | `file` feature の publish 失敗情報（RecoveryHandle 等）が上位で失われる | low | high | `DocumentFileWriteError::Publish(PublishFailure)` が全情報を保持する契約を Error 節と証拠索引（ERR-01/02→`issue-306` R3）で pin |
| 5 | backend 更新で公開境界・budget 保証・kind 分類が壊れる | med | med | `=` pin + `Cargo.lock` + release-age ポリシー。upgrade は PR で明示的に行い per-format test-spec の audit 規則で再検証 |
| 6 | docs のみの変更が「Consumer 受入完了」と誤解される | low | med | Design/Implementation Done 分離節で明示。Consumer 実測は #288/#219/#294 の段階 |

## Deferred Design Concerns

- Consumer 側の実測・移行（AL Manifest/Dataset Golden・
  byte 互換、SV Snapshot/HEAD 旧ファイル互換） | cross-repo
  gate（#219/#294）と各 Consumer Issue が Owner | consumer
  候補 tuple の実走時 | 設計どおりの互換性が実測で
  裏付けられないまま採用されるリスク。
- macOS/Linux 実機での `file` feature 経路の実測 | 本 slice
  の作業環境は Windows のみ | CI 既存 OS lane または実機
  環境が利用可能になった時点 | OS/FS 差異が Structured file
  publish で未検証のまま残る。
- 新 Format 追加・backend メジャー更新時の契約維持 | 新
  Format/backend 更新の要求時 | 公開 API 境界・budget
  保証・error 分類の再監査が必要になるリスク。
- `LimitNotGuaranteed` 相当の明示的診断面 | 現行は
  `Unsupported`/backend 固定分類で十分 | Consumer が
  「この limit は保証外」を programmatic に判別する実需が
  出た時点 | 保証範囲の誤認リスク。

## Verification Notes

- [ ] Serde 公開境界節が「許可: serde traits・std::io・
      Ruviform 独自型」「非公開: backend 型/Error/feature/
      version」「source() 経由のみ」「自作 Serializer/
      DOM/Registry なし」を全て含み、`issue-297` Rule 5・
      `issue-298` S8・`issue-299` S7 への参照を持つこと。
- [ ] Format/Version 契約節が 5 Format の API 族・単一文書
      Strict EOF・Consumer 所有の schema/version/canonical
      bytes・Format A→DOM→B 非提供を含むこと。
- [ ] Resource Budget 節が hard limit/parser budget/
      advisory の 3 層と「保証できない制限を偽装しない」
      規則を含み、`BoundedRead`・`max_bytes` 既定 16 MiB・
      Format 別 budget の実装済み値と一致すること。
- [ ] Error 契約節が `DocumentErrorKind` 7 variant・
      `DocumentFileReadError`/`DocumentFileWriteError` の
      variant と `PublishFailure` 保存を含むこと。
- [ ] Backend 選定表の版・MSRV・ライセンスが `Cargo.toml`
      の pin と per-format 設計の記述に一致すること
      （serde_json 1.0.154/serde-saphyr 2.0.0/toml
      1.1.8+spec-1.1.0/csv 1.4.0+csv-core 0.1.13/
      quick-xml 0.42.0）。
- [ ] 証拠項目対応表の OWN/CSV/XML/DOC/ERR/COMP 各行が
      実在する test-spec rule/節・#288 matrix 行・Issue/PR
      番号へ解決できること。
- [ ] Consumer 利用面節が AL #139/#143・SV #82/#84/#25 の
      Issue 番号と責務分界（schema/version は Consumer 所有・
      Ruviform 単一 Publish ≠ Project Transaction/HEAD CAS）
      を含むこと。
- [ ] 「設計合意と実装/検証の分離」節が存在し、未実施項目を
      `not run`/`NotRun`/`Unsupported` と記録する方針を
      明記していること。
