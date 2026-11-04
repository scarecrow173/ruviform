---
id: "TASK-034RoYsOdygNXYa3BanrU1"
type: "task"
status: "done"
title: "Issue 276 T1: logging config/API types + feature wiring"
created: "2026-11-03"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/276"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-276-generic-logging-adapter.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-276-generic-logging-adapter.md"
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
    - "docs/test-specs/issue-276-generic-logging-adapter.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 276 T1: logging config/API types + feature wiring

## Work

`Cargo.toml` に feature `logging` と optional 依存(`tracing`・`tracing-subscriber`
(env-filter/fmt/json)・`tracing-appender`)と dev-dep(`tempfile`・`serde_json`・`tracing`)を
配線し、`src/lib.rs` に `#[cfg(feature = "logging")] pub mod logging` を追加。
`src/logging/` に `mod.rs`(契約文書 + re-export)・`config.rs`・`error.rs`・`health.rs` を作り、
公開型(全 enum `#[non_exhaustive]`・struct は pub field plain)と rustdoc を書く。
writer/build は後続 T2 のため mod で宣言のみ or 先行 stub。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-276-generic-logging-adapter.md`
- `docs/specs/issue-276-generic-logging-adapter.md`
- `docs/designs/issue-276-generic-logging-adapter.md`

## Dependencies

- なし(最初の slice)。

## Affected Files

- `crates/ruviform-observability/Cargo.toml`
- `crates/ruviform-observability/src/lib.rs`
- `crates/ruviform-observability/src/logging/{mod,config,error,health}.rs`(新規)

## Done When

- [x] `cargo check -p ruviform-observability --no-default-features` が通る(core 無依存)。
- [x] `cargo check -p ruviform-observability --features logging` が通る。
- [x] config/error/health の全公開型が rustdoc 付きで存在し、enum に `#[non_exhaustive]`。
- [x] `cargo tree -p ruviform-observability --edges normal` で logging 配下の
      依存が feature ゲート内に閉じている。

## Verification

- [x] `mise run fmt-check`
- [x] `mise run check`
- [x] `mise run lint`
- [x] `cargo check -p ruviform-observability --no-default-features`
- [x] `cargo check -p ruviform-observability --features logging`
