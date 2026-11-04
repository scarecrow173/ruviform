---
id: "TASK-034RTghO8nOg3ZiYGau2pe"
type: "task"
status: "done"
title: "T2 YAML/TOML Format module・Budget 写像・2相分類 (Issue #297)"
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
    - "docs/tasks/issue-297-t1-crate-and-contract.md"
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

# T2 YAML/TOML Format module・Budget 写像・2相分類 (Issue #297)

## Work

`yaml.rs` に `serde-saphyr` 駆動の `from_reader`/`to_writer` を実装し、
`YamlReadOptions`（`limits` + `max_depth` + `max_aliases`）を Backend
`Options::budget` へ写像する。`Error::Budget`/alias 系→`LimitExceeded`、
`MultipleDocuments`→`TrailingData`、型不一致系→`DataModel`、
parser 系→`Syntax` の分類表を実装する。`toml.rs` は
`BoundedRead::read_to_string`（UTF-8→`InvalidEncoding`）+ 2 相分類
（`toml::from_str::<toml::Table>`→`Syntax`、`<T>`→`DataModel`）+
`toml::to_string`/`to_string_pretty`（`pretty`）を実装する。
各 Format の round-trip・否定試験を追加する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-297-io-structured-json-yaml-toml.md`
- `docs/designs/issue-297-io-structured-json-yaml-toml.md`
- `docs/specs/issue-297-io-structured-json-yaml-toml.md`

## Dependencies

- `serde-saphyr 2.0.0`（`default-features=false` + `serialize,deserialize`）
- `toml 1.1`（`std,serde,parse,display`）
- T1 の共通契約・`BoundedRead`

## Affected Files

- `crates/ruviform-io-structured/src/{yaml,toml}.rs`（新設）
- `crates/ruviform-io-structured/src/lib.rs`（mod/re-export）
- `crates/ruviform-io-structured/Cargo.toml`・`Cargo.lock`
- `crates/ruviform-io-structured/tests/structured.rs`（YAML/TOML 部分）

## Done When

- [ ] yaml/toml の round-trip・trailing・over-budget・invalid・
      Unicode・型不一致・複数文書（YAML）試験が通る
- [ ] `max_depth`/`max_aliases` が `LimitExceeded` として実効する
- [ ] 公開 API に `serde_saphyr`/`toml` の型が漏出しない

## Verification

- [ ] `mise run check` / `mise run lint` / `mise run test`（YAML/TOML 部分が緑）
