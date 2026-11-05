---
id: "TSPEC-034S6M7jOVNgIiAWwNm2J6"
type: "test-spec"
status: "approved"
title: "Issue #299 ruviform-io-structured — XML Module 試験仕様（IO-IMP-05）"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/299"
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
    - "docs/plans/issue-299-io-structured-xml.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-299-io-structured-xml.md"
    - "docs/designs/issue-299-io-structured-xml.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-299-io-structured-xml.md"
    - "docs/designs/issue-299-io-structured-xml.md"
    - "docs/test-specs/issue-298-io-structured-csv.md"
  refines:
    - "docs/plans/issue-299-io-structured-xml.md"
  refined-by: []
  derives-from:
    - "docs/plans/issue-299-io-structured-xml.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-299-io-structured-xml.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #299 ruviform-io-structured — XML Module 試験仕様（IO-IMP-05）

## 試験配置

既存 `tests/structured.rs` に XML セクションを追加
（`#[cfg(feature = "xml")]` ブロック、`mod xml_tests` 内）。
`file` 合成は `#[cfg(all(feature = "xml", feature = "file"))]`
で別ブロック。

## Golden / Positive

| ID | 内容 | 期待 |
| --- | --- | --- |
| G1 | 基本 XML（nested element + text） | Start/Text/End が順序どおり届く |
| G2 | namespace: `xmlns` default + `xmlns:p` prefix | element の `namespace()` が URI・`local_name()`/`prefix()`/`qualified_name()` が正しい |
| G3 | default ns は unprefixed **attribute** に適用されない | unprefixed attr の `namespace()` が `None`・prefixed attr が URI を持つ |
| G4 | namespaced attribute（`p:b`・`xml:lang`） | `p:b` が宣言 URI、`xml:lang` が `http://www.w3.org/XML/1998/namespace`（reserved pre-bind） |
| G5 | `xmlns=""` による default ns reset | 子 element が `Unbound`（`namespace() == None`） |
| G6 | borrowed `XmlEventRef` → `to_owned()` | `XmlEvent` が同構造・同内容を保持 |
| G7 | CDATA（`<`/`&`/`>` を含む生テキスト） | `CData` event が内容を保持・参照解釈されない |
| G8 | Text + entity/char ref（`a &lt; b &#65;`） | Text が分割で届き、`Reference` event が `Entity("<")`/`Char('A')` を持つ |
| G9 | Declaration（`version`/`encoding="UTF-8"`/`standalone`） | `XmlDeclarationRef` の各 accessor が正しい |
| G10 | PI（`<?target data?>`）・Comment | `ProcessingInstruction{target, content}`・`Comment` が届く |
| G11 | 零長入力・whitespace-only | 零長 → 即 `Ok(None)`。whitespace は `Text` として届く（backend 忠実） |
| G12 | `End` event の namespace 解決（pop 前 scope） | `</p:c>` で `p` が宣言 scope の URI を返す |
| G13 | attr `normalized_value()`（`x&amp;y`・tab/newline） | 正規化値が返る（predefined entity + whitespace 正規化） |
| G14 | `position()`/`depth()` の観測 | byte offset・open depth が正しく進む |

## Negative

| ID | 内容 | 期待 kind/limit |
| --- | --- | --- |
| N1 | `max_bytes` 超過入力 | `LimitExceeded{max_bytes}`（boundary exact/超過の 2 ケース） |
| N2 | `max_depth` 超過 nesting | `LimitExceeded{max_depth}` |
| N3 | `max_attributes` 超過要素 | `LimitExceeded{max_attributes}` |
| N4 | `max_event_bytes` 超過 event（巨大 text/CDATA） | `LimitExceeded{max_event_bytes}` |
| N5 | `max_events` 超過入力 | `LimitExceeded{max_events}` |
| N6 | `max_namespace_bindings` 超過 | `LimitExceeded{max_namespace_bindings}`（backend TooManyBindings → 同名 option に帰属） |
| N7 | DTD internal subset（`<!DOCTYPE d [<!ENTITY x "y">]>`） | `Unsupported` |
| N8 | DTD external ID（`<!DOCTYPE d SYSTEM "...">`） | `Unsupported` |
| N9 | 未定義 general entity（`&bogus;`） | `Unsupported` |
| N10 | 非 UTF-8 bytes（`\xFF\xFE`） | `InvalidEncoding` |
| N11 | mismatched end tag（`<a><b></a>`） | `Syntax` |
| N12 | 重複 attribute（`x="1" x="2"`） | `Syntax`（backend AttrError） |
| N13 | comment 内 `--`（`<!-- a -- b -->`） | `Syntax`（check_comments） |
| N14 | unclosed reference（`a & b`） | `Syntax` |
| N15 | undeclared prefix（`<p:x/>`） | `Syntax`（UnknownPrefix） |
| N16 | `xmlns:xml` の不正 bind | `Syntax`（InvalidXmlPrefixBind） |
| N17 | declared encoding が UTF-8 非互換 | `Unsupported` |
| N18 | `&#xZZ;` 等の malformed char ref | `Syntax` |
| N19 | 下層 `Read` error | `Io` |

## 構造・制約試験

| ID | 内容 | 期待 |
| --- | --- | --- |
| S1 | streaming — 入力を途中で切っても先頭 event が返る | 増分消費を pin（全入力一括読みでない・lookahead は BufReader chunk 内） |
| S2 | `XmlEventBuffer` 再利用 — 同一 buffer で N event 読み | 各 event が正しい値を返し、capacity が再利用される |
| S3 | `to_owned()` で event 保持後に buffer 再利用 | owned event が内容を保持（borrow 切り離し） |
| S4 | `position()` が event 保持中も callable | event が buffer のみ借用（reader freeze しない）を実測 |
| S5 | `read_file`（xml+file）の owned event 収集 | `Vec<XmlEvent>` が構造を保持・Open/Decode 失敗軸分離 |
| S6 | deterministic fuzz — seeded LCG で random/破断 XML | panic なし・規定 kind に分類される |
| S7 | 公開 API audit — xml.rs の pub signature に `quick_xml::` 型名がない | 目視/grep で backend 型非公開を pin |
| S8 | `Eof` 後の `read_event` 再呼出し | 引き続き `Ok(None)`（done latch） |
| S9 | `XmlEventBuffer::clear()` 後の再利用 | 正常に読み続けられる |

## 補助検証（canonical ではない）

- feature matrix: `cargo check/test --no-default-features`・
  `--features xml`・`--features xml,file`・`--all-features`。
- MSRV: `mise exec rust@1.89.0 -- cargo check -p ruviform-io-structured --all-features`。
- peak memory: `max_event_bytes` での公開前拒否 + buffer capacity
  再利用（S2）+ `BoundedRead` lookahead bound（S1）を観測。

## 未実施・制約

- 外部 fuzz harness（`cargo fuzz` 等）は repo 基盤にないため、
  deterministic seeded LCG fuzz で代替し、外部 fuzz 化は別 Issue。
- `max_event_bytes` は公開前拒否であり、backend buffer 伸長前の
  厳密な事前 cap ではない（文書化済み）。CPU 時間・全 event 累積
  コストは bound しない。
- 実 hardware/OS/GUI 依存の検証は対象外（全て headless）。
- `read_file` の `Vec<XmlEvent>` は `max_events`/`max_bytes` で
  bound されるが、owned event の heap 総量は advisory。
