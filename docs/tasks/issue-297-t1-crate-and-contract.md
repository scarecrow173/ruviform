---
id: "TASK-034RTggRy4UgfgaFSfQY4f"
type: "task"
status: "done"
title: "T1 ruviform-io-structured scaffold・共通契約・JSON Format (Issue #297)"
created: "2026-11-04"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/297"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-297-io-structured-json-yaml-toml.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-297-io-structured-json-yaml-toml.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-297-io-structured-json-yaml-toml.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 ruviform-io-structured scaffold・共通契約・JSON Format (Issue #297)

## Work

`crates/ruviform-io-structured` を新設する。`Cargo.toml` に feature
matrix（`default=[]`・`json`/`yaml`/`toml`/`file`）と依存
（`serde`・optional `serde_json`/`serde-saphyr`/`toml`・optional
`ruviform-io-file`）を定義し、workspace `members` と
`crates/README.md` に登録する。共通契約 `error.rs`
（`DocumentError`/`DocumentErrorKind`/`DocumentFormat`/
`DocumentPosition`）・`limits.rs`（`InputLimits`）・`input.rs`
（private `BoundedRead`）を書き、`json.rs` の
`from_reader`/`to_writer`/`Json*Options` と JSON の契約・否定試験を
実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-297-io-structured-json-yaml-toml.md`
- `docs/designs/issue-297-io-structured-json-yaml-toml.md`
- `docs/specs/issue-297-io-structured-json-yaml-toml.md`

## Dependencies

- `serde 1`・`serde_json 1`（optional、`feature=json`）
- MSRV 1.89・edition 2024・`workspace.lints`

## Affected Files

- `crates/ruviform-io-structured/`（新設: Cargo.toml・README.md・
  src/{lib,error,limits,input,json}.rs）
- `Cargo.toml`（`members` 追加）・`crates/README.md`・`Cargo.lock`
- `crates/ruviform-io-structured/tests/structured.rs`（JSON 部分）

## Done When

- [ ] `cargo check -p ruviform-io-structured` が default 全組合せで通る
- [ ] `json::from_reader`/`to_writer` の round-trip・trailing・
      over-budget・invalid・Unicode・型不一致試験が通る
- [ ] 公開 API に `serde_json` の型が漏出しない

## Verification

- [ ] `mise run check` / `mise run lint` / `mise run test`（JSON 部分が緑）
