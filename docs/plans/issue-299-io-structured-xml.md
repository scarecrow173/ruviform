---
id: "PLAN-034S6LiVSpxGAXkZWYagWB"
type: "plan"
status: "approved"
title: "Issue #299 ruviform-io-structured — XML Module 実装計画（IO-IMP-05）"
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
    - "docs/designs/issue-299-io-structured-xml.md"
    - "docs/specs/issue-299-io-structured-xml.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-299-io-structured-xml.md"
    - "docs/designs/issue-299-io-structured-xml.md"
    - "docs/plans/issue-298-io-structured-csv.md"
  refines:
    - "docs/designs/issue-299-io-structured-xml.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-299-io-structured-xml.md"
    - "docs/designs/issue-299-io-structured-xml.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-299-io-structured-xml.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #299 ruviform-io-structured — XML Module 実装計画（IO-IMP-05）

## Goal

設計書 `DESIGN-034S6K6h23VjXK8NquLQMx` に基づき、`xml` feature/
module を既存 scaffold に載せ、XML Event Streaming・namespace
解決・secure default（DTD/未定義 entity 拒否）・limit 強制を
実装する。

## Scope

単一の実装 stream。実装は 2 task に分け、各 task で検証可能な
単位にする（csv と同じ分解規約）。

## Upstream Documents

- `docs/specs/issue-299-io-structured-xml.md`

## Design Inputs

- `docs/designs/issue-299-io-structured-xml.md`

## File Map

- `crates/ruviform-io-structured/src/xml.rs` — 新規。Options・
  `XmlEventBuffer`・`XmlEventReader`・event/name/attribute 型・
  error 正規化・`read_file`。
- `crates/ruviform-io-structured/Cargo.toml` — `xml` feature +
  `quick-xml = "=0.42.0"` optional dep。
- `crates/ruviform-io-structured/src/{lib,error,input,file}.rs` —
  cfg/variant の additive 拡張。
- `crates/ruviform-io-structured/tests/structured.rs` — `xml`
  セクション追加。
- `crates/ruviform-io-structured/README.md`・`crates/README.md` —
  feature 表・制約の記述。

## Implementation Sequence

- [ ] `Cargo.toml` + `error.rs` + `lib.rs`/`input.rs`/`file.rs` cfg で土台。
- [ ] `src/xml.rs` を Options → `XmlEventBuffer` → name/attr 型 →
  `XmlEventReader` → `read_file` の順で実装。
- [ ] `tests/structured.rs` に XML セクション追加。
- [ ] `README.md`/`crates/README.md` 更新・impl record 作成。

## Task Breakdown

### T1: xml module 本体（`src/xml.rs` + 共有拡張）

`docs/tasks/issue-299-t1-xml-module.md`

- `Cargo.toml`: `xml = ["dep:quick-xml"]`・`quick-xml = "=0.42.0"`
  optional・`default-features = false`。
- `src/error.rs`: `DocumentFormat::Xml`・`with_parts`/`with_limit`
  cfg へ `xml` 追加。
- `src/lib.rs`・`src/input.rs`・`src/file.rs`: cfg 条件へ `xml` 追加。
- `src/xml.rs`: `XmlReadOptions`・`XmlEventBuffer`・
  `XmlNameRef`/`XmlName`・`XmlElementRef`/`XmlElement`・
  `XmlAttributeRef`/`XmlAttribute`・`XmlDeclarationRef`/
  `XmlDeclaration`・`XmlPiRef`/`XmlPi`・`XmlReferenceValue`・
  `XmlEventRef`/`XmlEvent`・`XmlEventReader`・error 正規化・
  `read_file`（`file` feature）。
- `tests/structured.rs` への unit test 追加（AC1–AC4 系）。

### T2: file feature 合成・試験仕上げ・README

`docs/tasks/issue-299-t2-file-docs.md`

- `read_file` の `file` feature テスト（owned event 収集）。
- deterministic fuzz（seeded LCG による破断 XML・random 構造）。
- `crates/ruviform-io-structured/README.md` feature matrix・XML
  制約（DTD/entity 拒否・encoding・lookahead・non-RT）記述。
- `crates/README.md` の module 一覧に XML を反映。
- `ci/impact/map.mjs` 等の feature matrix 対象確認。
- feature matrix・MSRV 補助検証の実施と記録。

## 検証計画

| 検証 | 方法 |
| --- | --- |
| fmt/check/lint/test | `mise run fmt`/`check`/`lint`/`test` |
| feature matrix | `cargo check/test` を `mise exec -- cargo ...` で feature 組合せ（default・xml・xml+file・all-features）— canonical でない補助検証として明記 |
| MSRV 1.89 | `mise exec rust@1.89.0 -- cargo check -p ruviform-io-structured --all-features`（環境に 1.89 がある前提、なければ記録） |
| 公開 API 監査 | rustdoc/目視で backend 型非露出を確認 |
| lookahead/streaming | 途中切断 `Read` で逐次消費を観測する test・buffer 再利用 test |
| fuzz | deterministic seeded LCG fuzz test（再現可能な内部生成） |
| `file` 合成 | `read_file` の `DocumentFileReadError` 分離 test |

## リスクと対策

- **resolver 借用と event 借用の競合**: `read_resolved_event_into`
  は reader を freeze するため、`read_event_into` + 手動
  `resolver().resolve_*` + aux buffer コピーで event を buffer のみ
  借用にする（design の核心）。
- **`max_event_bytes` の確保後検査**: 公開前拒否として正直に
  文書化し、事前 cap 相当を偽装しない。
- **namespace scope の pop 遅延**: End event 名は pop 前 scope で
  解決する（backend `pending_pop` 仕様に依存 — unit test で pin）。
- **`quick-xml` の非検証 parse**: 複数 root・top-level Text は
  忠実に流し、document 構造整形式は consumer 責務として明記。

## Done 判定

- spec AC1–AC5 を test・実測・記録で全て満たす。
- docs/tasks/issue-299-t1-xml-module.md・issue-299-t2-file-docs.md
  を done として index 再生成。
- impl record（`docs/impl/ir/issue-299-...md`）に command・exit
  code・未検証を記録。

## Open Questions / Gaps

- `ci/impact/map.mjs` に io-structured feature lane が存在するか —
  #298 と同様、なければ補助検証で代替する方針で確認済み。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker が直接実装）。
- Independent work streams: T1 → T2（依存順）。
- User approval required before dispatch: no（dispatched task として
  受入条件確定済み）。

## Review Handoff

- reviewer focus: borrow 契約（event が buffer のみ借用）・
  `max_event_bytes` の確保後検査の正直な文書化・DTD/entity 拒否の
  kind 分類（`Unsupported` vs `Syntax`）・`End` 名の pop 前解決。
