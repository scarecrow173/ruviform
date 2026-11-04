---
id: "PLAN-034RoY5EwSBQJwEF1k5arE"
type: "plan"
status: "approved"
title: "Issue #276 generic logging adapter implementation plan"
created: "2026-11-03"
updated: "2026-11-03"
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
    - "docs/specs/issue-276-generic-logging-adapter.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-276-generic-logging-adapter.md"
    - "docs/designs/issue-276-generic-logging-adapter.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-276-generic-logging-adapter.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #276 generic logging adapter implementation plan

## Goal

`ruviform-observability` に feature `logging` として、Host 所有の汎用 Logging Adapter
(tracing-subscriber 基盤・stderr/rolling file・sink 別 filter/format・有界 writer・
失敗分類・Health)を実装する。

## Scope

単一 plan・単一クレート・3 slice(T1 型+配線 → T2 writer+build → T3 試験+docs+検証)。
編集範囲は `crates/ruviform-observability/` と本 Issue の doc chain のみ。

## Upstream Documents

- `docs/specs/issue-276-generic-logging-adapter.md` (approved; AC1–AC8)
- `docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md` (accepted)
- Issue #275(Observability 統合設計正本)・Issue #276(受入正本)

## Design Inputs

- `docs/designs/issue-276-generic-logging-adapter.md` (approved) — モジュール構成・
  BoundedWriter・build 2 entry・失敗分類・Health・retention 委譲。

## File Map

- `crates/ruviform-observability/Cargo.toml` — `logging` feature + optional deps + dev-deps。
- `crates/ruviform-observability/src/lib.rs` — `#[cfg(feature = "logging")] pub mod logging`。
- `crates/ruviform-observability/src/logging/mod.rs` — モジュール契約 + re-export。
- `crates/ruviform-observability/src/logging/config.rs` — LoggingConfig 系型。
- `crates/ruviform-observability/src/logging/error.rs` — LoggingBuildError / SinkFailure。
- `crates/ruviform-observability/src/logging/health.rs` — LoggingHealth / snapshot 系。
- `crates/ruviform-observability/src/logging/writer.rs` — BoundedWriter / EventWriter /
  WriterGuard + 内部 unit test。
- `crates/ruviform-observability/src/logging/build.rs` — build_logging /
  build_logging_layers / BuiltSinks / SinkDegraded。
- `crates/ruviform-observability/tests/logging.rs` — 契約試験。
- `crates/ruviform-observability/examples/logging_host.rs` — Host サンプル。
- `crates/ruviform-observability/README.md` — logging feature 節を追加。
- `docs/impl/ir/issue-276-*.md` — implementation record(T3)。

## Implementation Sequence

- [x] T1: Cargo.toml feature/dep 配線 + `logging` mod 骨格 + config/error/health 型。
- [x] T2: writer.rs(BoundedWriter)+ build.rs(build_logging / build_logging_layers)。
- [x] T3: tests/logging.rs + examples + README + 全検証 + impl record。

## Dependencies and Risks

- tracing-appender の `max_log_files` は 0.2.3+。0.2 系なので下限 `0.2.3` を pin。
- 新規 worktree で依存未復元なら `mise install` → `mise run setup` が前提。
- 他 worker(#277)が同一クレートを編集中の可能性 — 最終検証前に `origin/main` を取り込む。
- Windows のみ検証。Linux/macOS の OS 差・readonly-dir 挙動差は未検証として明記。

## Task Breakdown

- T1 `task-276-t1-config-and-wiring.md`: feature 配線と config/error/health 型までで
  `cargo check -p ruviform-observability --features logging` が通る。
- T2 `task-276-t2-writer-sinks-build.md`: BoundedWriter と build entry を実装し
  writer unit test が通る。
- T3 `task-276-t3-tests-docs-verify.md`: 契約試験・example・README・全検証・IR。

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test`
- [x] `cargo check -p ruviform-observability --no-default-features`(core 無依存)
- [x] `cargo check -p ruviform-observability --features logging`(logging 単独で Builder)
- [x] `cargo test -p ruviform-observability --features logging`
- [x] `cargo +1.89 clippy -p ruviform-observability --all-targets --features logging -- -D warnings -A clippy::float_cmp -A clippy::similar_names`(MSRV informational。`mise exec rust@1.89.0` 経由で実行)
- [x] `cargo tree -p ruviform-observability --edges normal` で依存 DAG 確認
- [x] `cargo clippy -p ruviform-observability --features logging --all-targets -- -D warnings`(feature lane lint — 既定 `lint` は logging を含まないため)

## Open Questions / Gaps

- `tracing-subscriber` の feature set 最小幅は実装時にコンパイルで確定
  (`fmt`/`env-filter`/`json`/`std`)。
- Linux/macOS 未検証(本機 Windows のみ) — CI 3-OS matrix が後続で拾う。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(dispatched worker)。
- Independent work streams: none(T1→T2→T3 直列)。
- User approval required before dispatch: no(本 dispatch が承認済み作業指示)。

## Review Handoff

- `non_blocking` 不採用の根拠(観測不可能)と retention 委譲は ADR の焦点。
- stdout 非出力は構造的保証(SinkDestination に stdout 経路なし)。
