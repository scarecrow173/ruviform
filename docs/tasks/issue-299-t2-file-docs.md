---
id: "TASK-034S6MrEVaXtCjdoGQxnHP"
type: "task"
status: "done"
title: "Issue #299 T2 — file feature 合成・試験仕上げ・README"
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
        path: "crates/ruviform-io-structured/src/xml.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/README.md"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/issue-299-io-structured-xml.md"
  implements:
    - "docs/plans/issue-299-io-structured-xml.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-299-io-structured-xml.md"
    - "docs/designs/issue-299-io-structured-xml.md"
    - "docs/tasks/issue-299-t1-xml-module.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-299-io-structured-xml.md"
    - "docs/designs/issue-299-io-structured-xml.md"
    - "docs/tasks/issue-298-t2-file-docs.md"
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

# Issue #299 T2 — file feature 合成・試験仕上げ・README

## Work

T1 で実装した `xml` module の `file` feature 合成・残試験・
README/index 記述・impl record 作成を行う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-299-io-structured-xml.md`

## Dependencies

- `docs/tasks/issue-299-t1-xml-module.md`（xml module 本体）。

## Affected Files

- `src/xml.rs`: `read_file`（`file` feature — `open_document` helper
  上に owned event 収集）。
- `tests/structured.rs`: `xml+file` セクション・deterministic fuzz。
- `crates/ruviform-io-structured/README.md`: feature matrix・XML
  制約（DTD/entity 拒否・encoding・lookahead・non-RT）記述。
- `crates/README.md`: module 一覧に XML を反映。
- `ci/impact/map.mjs` 等の feature matrix 対象の確認（変更が必要
  なら map 更新、不要なら判断を記録）。
- `docs/impl/ir/issue-299-io-structured-xml.md`: command・exit
  code・未検証を記録。

## Done When

- [ ] `read_file` の `DocumentFileReadError` 分離 test（S5）・
  deterministic fuzz（S6）が test で pin 済み。
- [ ] feature matrix（default・xml・xml+file・all-features）の
  補助検証を実施・記録。
- [ ] README 2 箇所が XML feature を反映。
- [ ] impl record が command/exit code/未検証を記録。
- [ ] `mise run fmt`/`check`/`lint`/`test` 全緑。

## Verification

- [ ] `cargo test -p ruviform-io-structured --features xml,file` 緑。
- [ ] feature matrix 全組合せで check/test 緑（補助検証として明記）。
- [ ] MSRV 1.89 補助検証（環境に 1.89 があれば、なければ記録）。
