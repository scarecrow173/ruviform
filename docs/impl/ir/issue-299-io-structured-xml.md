---
id: "IMPL-034S7hWIC3uXInNg52Jtzp"
type: "impl"
status: "completed"
title: "issue-299-io-structured-xml"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "TASK-034S6MooKs5lp0V92HyYuw"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# issue-299-io-structured-xml

## Summary

Issue #299 の XML structured streaming を既存 `ruviform-io-structured`
crate の `xml` feature として実装。`XmlEventReader<R: BufRead>` が
caller 所有 `XmlEventBuffer` へ借用 event（`XmlEventRef<'_>`）を
供給し、`to_owned()` で所有 `XmlEvent` へ切り離す。namespace 解決・
DTD/entity 拒否の secure defaults・resource limits・正規化
`DocumentError` を備え、backend（quick-xml =0.42.0）は非公開依存。

## Context

docs/specs → docs/designs → docs/plans → docs/test-specs の
issue-299-io-structured-xml チェーンと TASK-034S6MooKs5lp0V92HyYuw /
T2 タスクに従う。

## Implementation

### Changes

- `src/xml.rs` 新規: `XmlReadOptions`/`XmlEventBuffer`/`XmlEventReader`/
  `XmlEventRef`/`XmlEvent` + Name/Element/Attribute/Declaration/PI/
  Reference の借用・所有型。
- `Cargo.toml`: `xml = ["dep:quick-xml"]` + `quick-xml =0.42.0`
  （optional・private・default-features=false）。
- `src/error.rs`: `DocumentFormat::Xml` + `with_parts` cfg 拡張。
- `src/lib.rs`/`src/input.rs`/`src/file.rs`: cfg 拡張（xml は read のみ、
  write 系 machinery には含めない）。
- `tests/structured.rs`: xml_tests（G1–G14・N1–N19・S1–S9 相当の
  40 件）+ xml_file_tests。
- `mise.toml`: `check/lint/test-io-structured` タスク追加（feature
  matrix の正規入口。io-compression lane と同型）。
- README 2 件更新。

### Decisions During Implementation

- **aux buffer を廃止**: backend の `BytesStart::name()`/
  `attributes()` が `&self` 束縛で `'b` を取り出せない（`buf`/
  `name_len` は `pub(crate)`）と判明。名・属性値・URI・宣言/PI
  文字列は `Cow<'a>` 型のまま `Owned` コピーとし、大きい payload
  （`Text`/`CData`/`Comment`）のみ `into_inner`/`xml1x_content` で
  真の借用を維持する構成に変更（design doc に反映済み）。
- **`max_event_bytes` は `buffer_position()` 差分計測**: event が
  `buffer.raw` を `&'b mut` 借用するため公開後に `raw.len()` を
  読めない。source span delta で同義の幅を計測し「公開前検査」契約
  を維持。
- **`new` は infallible**: BOM probe 等の先読みを行わないため
  `Result` ではなく `Self` を返す（csv の eager probe との差異）。
- **`failed` poison latch**: 一度の Err 後も reader を再使用させ
  ないため `Syntax` を継続返却する設計を追加。
- **CI lane**: `ruviform-io-structured` の CI lane 不在は #297 由来の
  既存 gap（json/yaml/toml/csv も未カバー）。mise task は追加したが
  新規 CI job/map 追加は本 Issue スコープ外として別 Issue 起票
  予定（follow-up）。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise run check-io-structured` / `lint-io-structured` /
  `test-io-structured`: feature matrix（default・xml・xml,file・
  all-features）全緑（117 tests + doctests）。
- `mise exec -- cargo clippy -p ruviform-io-structured --all-features
  --all-targets -- -D warnings`: 0 warnings。
- 負荷試験: DTD（internal/external）・未定義 entity・非 UTF-8
  encoding/byte・各 limit 分類・seeded fuzz（200 doc）を
  tests/structured.rs で pin。

## Risks

- `max_event_bytes` は backend の内部 buffering 後の公開時検査で、
  確保前の厳密 cap ではない（design/公開 docs に明記済み）。
- 複数 root / top-level text は backend 忠実に受理（document 構造
  検査は責務外・design で確定済み）。

## Follow-ups

- io-structured 用 CI lane（job + impact map エントリ）の整備 —
  #297 以来の format 共通 gap。別 Issue を起票する。
