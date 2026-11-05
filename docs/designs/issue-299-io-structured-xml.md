---
id: "DESIGN-034S6K6h23VjXK8NquLQMx"
type: "design"
status: "approved"
title: "Issue #299 ruviform-io-structured — XML Module 設計（IO-IMP-05）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/299"
    - "https://github.com/scarecrow173/ruviform/issues/284"
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
  implements:
    - "docs/specs/issue-299-io-structured-xml.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-299-io-structured-xml.md"
    - "docs/designs/issue-298-io-structured-csv.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-299-io-structured-xml.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/issue-298-io-structured-csv.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines:
    - "docs/specs/issue-299-io-structured-xml.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-299-io-structured-xml.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #299 ruviform-io-structured — XML Module 設計（IO-IMP-05）

## Context

既存 `ruviform-io-structured` crate に `xml` feature を追加し、
`xml` module（`src/xml.rs`）として XML Event Streaming を実装する。
Issue #297/#298 が確立した共通契約 — caller 借用 `BufRead`・
`InputLimits` byte budget・正規化 `DocumentError`・backend 型
非公開・`file` feature 合成・非 RT — をそのまま継承し、XML
固有の event 逐次読取・namespace 解決・secure default（DTD・
未定義 entity 拒否）を実現する。

### Backend 選定

| 役割 | 採用 crate | 理由 |
| --- | --- | --- |
| Parse engine + Namespace resolver | `quick-xml` `=0.42.0` | borrowed event（`Event<'b>` が caller 所有 `&mut Vec<u8>` を借用）・`NsReader` の namespace 解決（default ns・`xml`/`xmlns` reserved・binding stack を backend が管理）・`resolver_mut().set_max_namespace_bindings` の既存 limit・`NamespaceError::TooDeeplyNested`（u16 depth counter）・`GeneralRef` event で entity の見極めが可能。MSRV 1.86 < 1.89・MIT・2026-08-22 公開（7 日超）。|

代替候補を採らない理由: `xml-rs`/`roxmltree` 系は DOM/検証方向で
pull+borrowed buffer の組合せを満たさない。`quick-xml` の
`Reader`（ns 無し）は `xmlns` 追跡を自前実装にするため
`NsReader` を採用する。

## Scope

### In Scope

- spec の In Scope に同じ（`src/xml.rs` + 共有 cfg 拡張 + 試験 +
  README 2 箇所 + `Cargo.lock` 再解決）。

### Out of Scope

- spec の Out of Scope に同じ（writer・serde・schema・非 UTF-8
  decode・Consumer 意味論・新規 CI lane）。

## Components and Boundaries

### Feature・依存

```toml
[features]
xml = ["dep:quick-xml"]

[dependencies]
quick-xml = { version = "=0.42.0", default-features = false, optional = true }
```

`default-features = false` で最小構成（`encoding` 系 feature 不要 —
入力は UTF-8 のみ受付け、backend の `&str` 系 API がそのまま使える）。
`serde` 連携 feature も不要（typed deserialize は本 Issue の対象外）。

### 共有構造の拡張（変更は additive）

- `error.rs`: `DocumentFormat::Xml` variant + Display `"xml"`。
  `with_parts`/`with_limit` の cfg `any(...)` に `feature = "xml"` 追加。
- `input.rs`: `BoundedRead` は変更不要。module cfg `any(...)` に
  `xml` 追加（`read_to_string` の cfg は `toml` のみのまま — XML は
  streaming reader で使用しない）。
- `lib.rs`: `#[cfg(feature = "xml")] pub mod xml;` + module 表に
  `xml`/`quick-xml` 行。
- `file.rs`: `open_document`/`write_document` 周辺の cfg `any(...)`
  に `xml` 追加（`read_file` convenience 用・`Path`/`BufReader`/
  `StagedFile` import も同条件）。※ write 側は対象外なので
  `write_document` cfg には `xml` を含めない（dead code 回避 —
  `open_document`/`Path`/`BufReader` のみ xml を追加、
  `write_document`/`StagedFile`/`PublishOptions` は既存 format のみ）。

### `src/xml.rs` 公開型

```rust
pub struct XmlReadOptions {                 // #[non_exhaustive] Clone+Copy
    pub limits: InputLimits,                // default 16 MiB
    pub max_depth: Option<u64>,             // default Some(512)
    pub max_attributes: Option<u64>,        // default Some(512)
    pub max_event_bytes: Option<u64>,       // default Some(16 MiB) — event の source byte 幅
    pub max_events: Option<u64>,            // default Some(1_000_000)
    pub max_namespace_bindings: Option<usize>, // default None（backend DEFAULT 128 維持）
}

pub struct XmlEventBuffer { raw: Vec<u8> }  // #[non_exhaustive]

pub struct XmlEventReader<'a, R: BufRead> {
    inner: NsReader<BufReader<BoundedRead<&'a mut R>>>,
    options: XmlReadOptions,
    events: u64,
    done: bool,
    failed: bool,                          // error poison latch
    version: quick_xml::XmlVersion,        // private（attr/text normalize 用）
    last_byte: u64,                        // max_event_bytes の source-span 計測
}

pub enum XmlEventRef<'a> {                  // #[non_exhaustive]
    Start(XmlElementRef<'a>),
    End(XmlNameRef<'a>),
    Empty(XmlElementRef<'a>),
    Text(Cow<'a, str>),                  // xml10/xml11 行末正規化済み・借用
    CData(Cow<'a, str>),                  // raw・借用
    Comment(Cow<'a, str>),                // raw・借用
    Declaration(XmlDeclarationRef<'a>),
    ProcessingInstruction(XmlPiRef<'a>),
    Reference(XmlReference),              // value + position（owned）
}

pub enum XmlEvent { /* owned mirror — String/owned payload */ }

pub struct XmlNameRef<'a> {                  // qualified/prefix/local/namespace
    qualified: Cow<'a, str>,              // 実装では Owned（下記 Borrow 設計参照）
    prefix: Option<Cow<'a, str>>,
    local: Cow<'a, str>,
    namespace: Option<Cow<'a, str>>,       // Bound → URI コピー
}
pub struct XmlElementRef<'a> { name: XmlNameRef<'a>, attributes: Vec<XmlAttributeRef<'a>> }
pub struct XmlAttributeRef<'a> { name: XmlNameRef<'a>, value: Cow<'a, str>, version: XmlVersion }
pub struct XmlDeclarationRef<'a> { version: Cow<'a,str>, encoding: Option<Cow<'a,str>>, standalone: Option<bool> }
pub struct XmlPiRef<'a> { target: Cow<'a, str>, content: Cow<'a, str> }
pub enum XmlReferenceValue { Entity(&'static str), Char(char) }  // 'static 共有
// owned 側: XmlName/XmlElement/XmlAttribute/XmlDeclaration/XmlPi（String payload）
```

### Borrow 設計（核心）

`read_event<'b>(&mut self, buffer: &'b mut XmlEventBuffer)
-> Result<Option<XmlEventRef<'b>>, DocumentError>`:

- `self.inner.read_event_into(&mut buffer.raw)` → `Event<'b>` は
  `buffer.raw` のみを借用し、reader 本体は借用しない
  （`read_resolved_event_into` は resolver を借用するため reader を
  freeze する — 採用しない）。
- element/attribute 名は `self.inner.resolver()`（shared borrow）で
  `resolve_element`/`resolve_attribute` に渡す。返る
  `ResolveResult::Bound(Namespace<'x>)` の URI は resolver 所有の
  ため **所有コピー**（`Cow::Owned`）して event に持たせる。
- **`'b` で取れるものと取れないもの**: backend の element API
  （`BytesStart::name()`/`attributes()` 等）は `&self` 由来の寿命しか
  返さず `buf`/`name_len` は `pub(crate)` — `'b` を取り出す公開経路が
  ない。したがって **element/attribute 名・属性値・namespace URI・
  宣言/PI 文字列は `Cow::Owned` コピー** とする（event byte 幅で
  bound 済みの小文字列）。一方 `Text`/`CData`/`Comment` は
  `into_inner()`/`xml10_content()`/`xml11_content()` が `Cow<'b>` を
  返すため **大きい payload は真の借用** を維持する。`Cow<'a>` 型
  署名のため API 契約（借用 event）は変わらない。
- backend は End event でも scope の pop を次回 read まで遅延させる
  （`pending_pop`）ため、`End` 名は閉じる element 自身の scope で
  解決できる。
- `Vec<XmlAttributeRef<'b>>` + 各小文字列コピーの確保は
  `max_attributes`/`max_event_bytes` までに bound。
- `XmlNameRef` の `namespace()`/`prefix()`/`local_name()`/
  `qualified_name()` は `&str`/`Option<&str>` を返し backend 型を
  露出しない。

### Event マッピング

| backend `Event<'b>` | 公開 `XmlEventRef<'b>` | 付加処理 |
| --- | --- | --- |
| `Start(e)` | `Start(XmlElementRef)` | depth = `resolver().level()` が `max_depth` 超 → `LimitExceeded`。attributes を eager 列挙（backend AttrError → 分類）し `max_attributes` 超 → `LimitExceeded`。各 attr を `resolve_attribute`（default ns は付かない）。 |
| `Empty(e)` | `Empty(XmlElementRef)` | Start と同じ（depth は push 済み level で判定）。 |
| `End(e)` | `End(XmlNameRef)` | `resolve_element`（scope pop 前）。 |
| `Text(e)` | `Text(Cow<'a,str>)` | `xml10_content`/`xml11_content` で宣言 version の行末正規化（entity は GeneralRef event として別途届く）。 |
| `CData(e)` | `CData(Cow<'a,str>)` | raw。 |
| `Comment(e)` | `Comment(Cow<'a,str>)` | raw。`check_comments = true` で `--` を ill-formed 扱い。 |
| `Decl(e)` | `Declaration(XmlDeclarationRef)` | `version()`/`encoding()`/`standalone()` を解釈。`encoding` が UTF-8 非互換なら `Unsupported`。version "1.1" → `XmlVersion::Explicit1_1` を reader に保持。 |
| `PI(e)` | `ProcessingInstruction(XmlPiRef)` | `target()`/`content()`（content は raw・先頭空白を含む）。 |
| `DocType(_)` | — | **常に `Unsupported`**（DTD secure default）。 |
| `GeneralRef(e)` | `Reference(XmlReference)` | `resolve_char_ref()` → `Char(c)`（Err → `Syntax`）、`resolve_xml_entity()` → `Entity(&'static str)`、両方 None → **`Unsupported`**（未定義 entity — DTD 拒否のため定義不能）。 |
| `Eof` | `Ok(None)` | `done` を latch。 |

### Limit 強制表

| limit | 強制点 | kind / limit.name |
| --- | --- | --- |
| `limits.max_bytes` | `BoundedRead`（pull 課金・probe 付き） | `LimitExceeded{max_bytes}` |
| `max_depth` | Start/Empty 後 `resolver().level()` | `LimitExceeded{max_depth}`（実効値 = min(option, u16::MAX）） |
| `max_attributes` | attribute eager 列挙数 | `LimitExceeded{max_attributes}` |
| `max_event_bytes` | 連続 `buffer_position()` 差分（event 毎の source byte 幅・**公開前**検査。`buffer.raw` は event が借用中のため len() 不可 — position delta で同義の span を計測） | `LimitExceeded{max_event_bytes}` |
| `max_events` | 正常 yield 前カウンタ | `LimitExceeded{max_events}` |
| `max_namespace_bindings` | `resolver_mut().set_max_namespace_bindings`（backend push 時） | `LimitExceeded{max_namespace_bindings}` |

強制不能な点の文書化: 単一 event の heap 肥大は backend buffer
伸長のタイミングでしか止められないため `max_event_bytes` は
「公開前検査」であり transient 確保の厳密な事前 cap ではない。
CPU 時間・全 event の cumulative コストも bound しない。

### Error 正規化

| backend | kind |
| --- | --- |
| `Error::Io`（BoundedRead trip） | `LimitExceeded{max_bytes}` |
| `Error::Io`（その他） | `Io` |
| `Error::Syntax` / `IllFormed` / `InvalidAttr` | `Syntax` |
| `Error::Encoding`（UTF-8） | `InvalidEncoding` |
| `Error::Escape`（`UnrecognizedEntity`） | `Unsupported` |
| `Error::Escape`（その他） | `Syntax` |
| `Error::Namespace::TooManyBindings/TooDeeplyNested` | `LimitExceeded` |
| `Error::Namespace`（UnknownPrefix・Invalid*Bind） | `Syntax` |
| DocType event / 未定義 GeneralRef / 非 UTF-8 declared encoding | `Unsupported`（`BackendMessage` 診断） |

`error_position()` → `DocumentPosition.byte`。`source()` に backend
診断を保持。`DocumentError` の `Display` は backend message を含めない
（既存契約）。

## Data and Control Flow

```
caller &mut BufRead
  → BoundedRead<&mut R>            （pull 課金 + limit probe）
  → BufReader<BoundedRead<..>>     （lookahead ≤ chunk サイズ）
  → NsReader                       （parse + scope push/pop 遅延）
  → Event<'b> → buffer.raw         （借用）
  → resolver().resolve_*           （Bound URI は Owned コピー）
  → XmlEventRef<'b>                （payload 借用 + metadata 所有）
```

## Non-Functional Constraints

- 非 RT: I/O block・確保あり（buffer・attr Vec・小文字列コピー・backend 内部）。
- `unsafe` 不使用・公開 enum/struct `#[non_exhaustive]`。
- UTF-8 のみ受理（backend `&str` API がそのまま検査）。BOM は
  現状 passthrough（strip は deferred — Consumer 実需なし。UTF-16
  BOM は backend の Encoding error で自然に `InvalidEncoding`）。

## Risks and Trade-offs

- **metadata の Owned コピー**: 名・属性値・URI・宣言/PI 文字列は
  backend 都合（`&self` 束縛 API）で `Cow::Owned` コピーとなる。
  大きい payload（Text/CData/Comment）は真の借用を維持し、
  コピー対象は `max_event_bytes` で bound される小文字列のみ。
  resolver 借用を event に持たせる代替は reader freeze を招くため
  不採用。
- **`max_event_bytes` の確保後検査**: backend が event 全体を
  buffer へ書くまで止められない。事前 cap を偽装せず「公開前
  拒否」として文書化する。
- **`Vec<XmlAttributeRef>` の per-element 確保**: bound 付きの
  通常確保として許容（record buffer 再利用の精神に反しない —
  buffer 自体の heap は再利用される）。
- **`quick-xml` の非検証 parse**: 複数 root・トップレベル Text は
  backend が受理するため event として忠実に流す。document 構造の
  整形式保証は consumer 責務として文書化。

## Deferred Design Concerns

- `XmlEventWriter`/serialize | 実需後に別 Issue | read 側契約を先に固定 |
- `from_reader<T>` serde 接着 | Consumer 型確定後 | event→serde は別責務 |
- UTF-8 BOM strip / 非 UTF-8 decode | 実需後 | silent 誤 decode を防ぐため拒否を維持 |

## Verification Notes

- [ ] spec AC1–AC5 を tests/structured.rs の `xml` セクションで pin。
- [ ] feature matrix（default・xml・xml+file・all-features）を
  補助検証として実施・記録。
- [ ] backend 型非露出を `cargo doc` 出力/grep で目視確認。
