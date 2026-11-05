---
id: "SPEC-034S6JgQsu8LENsvnVQR6J"
type: "spec"
status: "approved"
title: "Issue #299 ruviform-io-structured — XML Event Streaming・Namespace・Secure Default (IO-IMP-05)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/299"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/219"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-structured/src/xml.rs"
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
    - "docs/specs/issue-298-io-structured-csv.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/specs/issue-298-io-structured-csv.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/issue-298-io-structured-csv.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/issue-298-io-structured-csv.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #299 ruviform-io-structured — XML Event Streaming・Namespace・Secure Default (IO-IMP-05)

## What Should Be Built

IO-IMP-03（Issue #297）の scaffold・IO-IMP-04（Issue #298）の CSV
streaming 契約を継承し、IO-02（Issue #284）合意済み設計の
**XML Event Streaming** を `xml` feature/module として既存
`crates/ruviform-io-structured` に追加する。新規クレートは作らない。

公開面（feature `xml` の `xml` module）:

- `XmlEventReader<'a, R: BufRead>` — caller が所有する `&mut R` を
  借用する Event 逐次読取器。`read_event(&mut self, &mut XmlEventBuffer)
  -> Result<Option<XmlEventRef<'_>>, DocumentError>`（EOF = `Ok(None)`）。
  backend の lookahead/prefetch は `BufReader` chunk 境界に限定される。
- `XmlEventBuffer` — 再利用可能な caller 所有 buffer。Event はこの
  buffer に借用され、次の `read_event`/`clear` まで有効。
- `XmlEventRef<'a>` — borrowed event enum: `Start`/`End`/`Empty`/
  `Text`/`CData`/`Comment`/`Declaration`/`ProcessingInstruction`/
  `Reference` を保持。`to_owned()` で `'static` な `XmlEvent` を得る。
- Namespace/QName/Attribute 表現: `XmlNameRef`/`XmlName`（qualified
  name・prefix・local name・namespace URI）、`XmlElementRef`/
  `XmlElement`（name + attributes）、`XmlAttributeRef`/`XmlAttribute`
  （name + raw value + normalized value accessor）。
- Namespace 意味: default namespace は prefix 無し **element** 名に
  適用し、prefix 無し **attribute** には適用しない（XML Namespaces
  仕様どおり）。`xml`/`xmlns` の reserved prefix は backend が
  規則を強制する。
- Secure Default: DTD（`DOCTYPE` 宣言・内部/外部 subset）は
  `Unsupported` で拒否。general entity reference は定義不能なため
  `Unsupported` で拒否し、XML 定義済み entity（`lt`/`gt`/`amp`/
  `apos`/`quot`）と数値 character reference のみ `Reference` event
  として解決値付きで受理する。
- `XmlReadOptions` — `limits`（`InputLimits` byte budget）に加え
  `max_depth`（要素 nesting）・`max_attributes`（要素当たり）・
  `max_event_bytes`（単一 event の buffer サイズ）・`max_events`
  （総 event 数）・`max_namespace_bindings`（スコープ内 xmlns
  宣言数）を有限既定値の hard limit として公開する。
- `feature = "file"` 併用時の `read_file` — 既存 `open_document`
  helper 上に合成し、全 event を owned `Vec<XmlEvent>` として収集
  して返す。失敗軸分離は `DocumentFileReadError` を再利用。

設計上の責務境界（IO-02 合意 + Issue #299 本文）:

- Depth/Attribute/Event/Bytes 上限を backend が許す限り確保前に
  実施する。`max_bytes` は `BoundedRead`（pull 単位で課金）が強制、
  `max_depth`/`max_namespace_bindings` は backend の scope 機構が
  push 時に強制、`max_attributes`/`max_event_bytes`/`max_events`
  は event 組立て中・公開前に強制。強制不能なものは `Unsupported`
  として文書化し、黙って成功扱いしない。
- Text は entity/character reference 周辺で分割されて届くため、
  `Text` event は生の（unescape されていない）文字列、`Reference`
  event が解決済みの replacement を保持する。連結は consumer 責務。
- backend（`quick-xml`）の型・Error・feature 名は公開 API に露出
  しない。診断は `Error::source()` の `dyn Error` 経由のみ。
- Writer・serde typed deserialize・Schema 検証（XSD/DTD）は本
  Issue の対象外。Event streaming の意味保存のみを責務とする。

## Why Now

IO-IMP-04 で streaming format（record 逐次）の共通契約は実証済み。
XML は IO-02 が列挙する必須 format の残りで、event streaming と
namespace/entity の security 契約（DTD・external entity の secure
default 拒否）をこの Issue で固定する。AudioLens/Synvelis の
manifest・metadata 系で XML 実需がある。

## Users and Value

- AudioLens / Synvelis: XML manifest・metadata の逐次読取を、
  namespace 解決・limit 強制・secure default 付きで利用。
- Ruviform 内部: 大きな XML を DOM 無しで逐次処理する用途に、
  borrowed buffer 再利用と resource limit の安全な入口を提供。

## Scope

### In Scope

- `crates/ruviform-io-structured` への `xml` feature/module 追加:
  `XmlEventReader`/`XmlEventBuffer`/`XmlEventRef`/`XmlEvent`/
  `XmlNameRef`/`XmlName`/`XmlElementRef`/`XmlElement`/
  `XmlAttributeRef`/`XmlAttribute`/`XmlDeclarationRef`/
  `XmlDeclaration`/`XmlPiRef`/`XmlPi`/`XmlReferenceValue`/
  `XmlReadOptions` と rustdoc（典型例・制限・non-RT 明記）。
- `DocumentFormat::Xml` variant（`#[non_exhaustive]` enum の
  additive 拡張・Display `"xml"`）と `with_parts`/`with_limit`/
  `input`/`file` の cfg への `xml` 追加。
- Backend: `quick-xml`（`NsReader` による namespace 解決と
  borrowed event）。private 実装詳細・公開面に出さない。
- 試験: namespace（default/prefix/attribute）・borrowed/owned
  等価・CDATA/Text・Declaration/PI/Comment・EOF/零長・DTD 拒否・
  external/general entity 拒否・各 limit 分類・不正 UTF-8・
  depth/attr/event/bytes 上限・buffer 再利用・lookahead 境界・
  streaming 増分消費・`file` 合成・feature matrix・deterministic
  fuzz。
- crate README・`crates/README.md` の記述更新。

### Out of Scope

- XML Writer/serialize 側（`XmlEventWriter` 等）。
- serde typed deserialize（`from_reader<T>` 型 API）。
- Schema 検証（XSD・DTD・RelaxNG）・entity 展開・XInclude。
- 既存 `json`/`yaml`/`toml`/`csv` feature・`file` feature・
  `ruviform-io-file` の契約変更。
- 非 UTF-8 入力の decode（Shift_JIS・UTF-16 等）。宣言 encoding
  が UTF-8 非互換の文書は `Unsupported` で拒否。
- Consumer 独自 Schema/意味論。
- 新規 CI job/lane の追加（feature matrix 検証は #298 と同様
  auxiliary evidence として実施）。

## Requirements

- `default = []` を維持し、`xml` は他 format feature と独立に
  組合せ可能。`xml` + `file` で `read_file` が有効化。
- MSRV 1.89・edition 2024・`workspace.lints` 継承・unsafe 不使用・
  全公開 enum `#[non_exhaustive]`・全 API 非 RT（crate doc 明記）。
- 依存追加は公開から 7 日以上経過した版・メンテナンス状況・
  ライセンス・依存木サイズを確認して記録する（`quick-xml` は
  `=0.42.0` で pin・MIT・MSRV 1.86）。
- Backend 型・Error・feature 名を公開 API に露出しない。
  公開 API 監査を試験/目視で pin する。
- 実現できない制限を黙って成功扱いせず、`Unsupported` /
  `LimitExceeded` / 文書化された制約のいずれかに分類する。
- `DocumentError::limit()` に超過した公開 option 名と上限を保持
  する（`max_bytes`/`max_depth`/`max_attributes`/
  `max_event_bytes`/`max_events`/`max_namespace_bindings`）。
- `ruviform-io-file` → `ruviform-io-structured` の逆依存を作らない。

## Acceptance Criteria

- [ ] AC1: `xml` feature で `XmlEventReader::read_event` が
  Start/End/Empty/Text/CData/Comment/Declaration/PI/Reference を
  意味保存で返し、namespace（default・prefix・namespaced
  attribute・`xml` reserved）が正しく解決される。零長入力・
  EOF・borrowed/owned event 等価を試験で pin。
- [ ] AC2: DTD（internal subset・external ID）・未定義 general
  entity が `Unsupported` で拒否される。定義済み entity・数値
  char ref は `Reference` event として受理される。
- [ ] AC3: `max_bytes`/`max_depth`/`max_attributes`/
  `max_event_bytes`/`max_events`/`max_namespace_bindings` が
  `LimitExceeded`（`limit()` に名と値）として強制される。不正
  UTF-8 は `InvalidEncoding`。ill-formed XML（mismatched end・
  重複 attribute・comment 内 `--`）は `Syntax`。
- [ ] AC4: `XmlEventBuffer` 再利用・borrow 契約（event が次の
  read まで有効・`to_owned` で永続化）・backend lookahead が
  `BufReader` chunk に収まることを試験で pin。公開 API に
  backend 型が露出しない。
- [ ] AC5: MSRV 1.89・non-RT・依存 DAG（io-file への一方向）・
  `mise run fmt`/`check`/`lint`/`test` 全緑・feature matrix
  （default・xml 単独・xml+file・all-features）成立。

## Deferred Design Concerns

- XML Writer/serialize | 実需確定後に別 Issue | 本 Issue は read
  streaming のみを確定させる。|
- serde typed deserialize（`from_reader<T>` 型） | Consumer の
  manifest 型が確定してから | event → DOM/serde 接着は別責務。|
- 非 UTF-8 declared encoding の decode 層 | 実需が出てから | 現状は
  `Unsupported` で明示拒否し、silent な誤 decode を防ぐ。|

## Sources and Relations

- 正本: Issue #299 本文（作業範囲・Acceptance・Done/非対象）。
- 設計基準: Issue #284（IO-02）— XML event streaming・namespace
  保持・DTD/external entity secure default 拒否・確保前上限。
- 依存実装: Issue #297（scaffold・`DocumentError` 族・
  `BoundedRead`・`file` feature helper）、Issue #298（streaming
  format の共通契約・reusable buffer・borrowed reader パターン）。
- 横断 Gate: Issue #288（IO-06）、Issue #219（Cross-Repo）。
