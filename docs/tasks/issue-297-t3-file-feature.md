---
id: "TASK-034RTgiDjrTlcheIw55nAn"
type: "task"
status: "done"
title: "T3 file feature・workspace 台帳・総合検証 (Issue #297)"
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
    - "docs/tasks/issue-297-t2-yaml-toml.md"
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

# T3 file feature・workspace 台帳・総合検証 (Issue #297)

## Work

`file.rs` に `DocumentFileReadError::{Open,Decode}` /
`DocumentFileWriteError::{Create,Encode,Publish}` を実装し、各 Format
module に `read_file`/`write_file`（`StagedFile::create` → serialize
→ `seal()` → `publish`）を追加する。Encode 失敗時は `Encode` を返し
staged を Drop の best-effort cleanup に委譲、Publish 失敗時は
`PublishFailure` をそのまま `Publish` variant で保持する。crate
README・台帳記述を仕上げ、feature matrix・否定試験を含む総合検証を
`mise run fmt`/`check`/`lint`/`test` で緑にする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-297-io-structured-json-yaml-toml.md`
- `docs/designs/issue-297-io-structured-json-yaml-toml.md`
- `docs/specs/issue-297-io-structured-json-yaml-toml.md`

## Dependencies

- `ruviform-io-file`（optional path 依存、`feature=file`）
- T1/T2 の Format module

## Affected Files

- `crates/ruviform-io-structured/src/file.rs`（新設）
- `crates/ruviform-io-structured/src/{lib,json,yaml,toml}.rs`
- `crates/ruviform-io-structured/Cargo.toml`・`README.md`・`Cargo.lock`
- `crates/ruviform-io-structured/tests/structured.rs`（file 部分）
- `docs/impl/ir/issue-297-*.md`（実装記録）

## Done When

- [ ] `write_file` の Serialize 失敗→`Encode`・Publish 失敗→
      `Publish`（effect/recovery 保持）・`read_file` の
      Open/Decode 分離が試験で pin される
- [ ] `file` feature 単独・Format×`file`・all-features が compile
- [ ] `mise run fmt`/`check`/`lint`/`test` 全緑・merge 後再検証

## Verification

- [ ] `mise run fmt` / `mise run check` / `mise run lint` / `mise run test`
- [ ] feature matrix の `cargo check -p`（補助証拠）
