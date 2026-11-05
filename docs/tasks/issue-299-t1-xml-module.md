---
id: "TASK-034S6MooKs5lp0V92HyYuw"
type: "task"
status: "done"
title: "Issue #299 T1 — xml module 本体と共有拡張"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/299"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-structured/src/xml.rs"
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
        path: "crates/ruviform-io-structured/src/file.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/plans/issue-299-io-structured-xml.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-299-io-structured-xml.md"
    - "docs/designs/issue-299-io-structured-xml.md"
  blocks:
    - "docs/tasks/issue-299-t2-file-docs.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-299-io-structured-xml.md"
    - "docs/designs/issue-299-io-structured-xml.md"
    - "docs/tasks/issue-298-t1-csv-module.md"
  refines:
    - "docs/plans/issue-299-io-structured-xml.md"
  refined-by: []
  derives-from:
    - "docs/plans/issue-299-io-structured-xml.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-299-io-structured-xml.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #299 T1 — xml module 本体と共有拡張

## Work

`xml` feature で公開される `xml` module（`src/xml.rs`）と、それを
可能にする共有構造の additive 拡張を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-299-io-structured-xml.md`
- `docs/designs/issue-299-io-structured-xml.md`

## Dependencies

- なし（先頭 task）。T2 が本 task に依存。

## Affected Files

- `Cargo.toml`: `xml = ["dep:quick-xml"]` feature・optional 依存
  `quick-xml = "=0.42.0"`（`default-features = false`）。
- `src/error.rs`: `DocumentFormat::Xml` variant + Display。
  `with_parts`/`with_limit` の cfg `any(...)` に `feature = "xml"`。
- `src/lib.rs`: `#[cfg(feature = "xml")] pub mod xml;` + module 表。
- `src/input.rs`・`src/file.rs`: cfg `any(...)` に `xml` 追加
  （`open_document`/`Path`/`BufReader` のみ — `write_document`/
  `StagedFile`/`PublishOptions` には xml を含めない）。
- `src/xml.rs`: 新規。`XmlReadOptions`/`XmlEventBuffer`/
  `XmlNameRef`/`XmlName`/`XmlElementRef`/`XmlElement`/
  `XmlAttributeRef`/`XmlAttribute`/`XmlDeclarationRef`/
  `XmlDeclaration`/`XmlPiRef`/`XmlPi`/`XmlReferenceValue`/
  `XmlEventRef`/`XmlEvent`/`XmlEventReader`/error 正規化/
  `read_file`（`file` feature）。
- `tests/structured.rs`: `xml` セクション追加。

## Done When

- [ ] `xml` feature で `cargo check`/`test` が通る（AC1–AC4 の
  unit test 含む）。
- [ ] spec の Error 正規化表・Limit 強制表どおりに kind/limit が
  分類される。
- [ ] 公開 API に `quick_xml::` 型名が露出しない（目視/grep）。
- [ ] `mise run fmt`/`check`/`lint`/`test` 全緑。

## Verification

- [ ] test-spec G1–G14・N1–N19・S1–S4・S7–S9 が test で pin 済み。
- [ ] `cargo test -p ruviform-io-structured --features xml` 緑。
