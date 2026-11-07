---
id: "TASK-nHtJqA3vzK6uhK4D7A0FhO"
type: "task"
status: "done"
title: "T1 ruviform-io-e2e クレート scaffold (Issue #306)"
created: "2026-11-07"
updated: "2026-11-07"
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
    - "docs/plans/issue-306-io-composition-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-306-io-composition-e2e.md"
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
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  references: []
  defers: []
  deferred-by: []
---

# T1 ruviform-io-e2e クレート scaffold (Issue #306)

## Work

`crates/ruviform-io-e2e/` を新設する。`Cargo.toml`（dev-deps:
io-file / io-structured[json,csv,file] / io-compression[gzip,zstd] /
io-archive / io-wav / io-symphonia / media-io / media-core /
audio-core / serde[derive]、workspace lints）、`src/lib.rs`
（crate doc のみ・公開 API なし・非 RT 宣言）、`README.md`、
`tests/common/mod.rs`（RAII `TestDir` + staged 補助、archive
tests/common と同型）。workspace `members`、`crates/README.md`
台帳、`Cargo.lock` を更新。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-306-io-composition-e2e.md`
- `docs/designs/issue-306-io-composition-e2e.md`
