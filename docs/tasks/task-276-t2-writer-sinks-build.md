---
id: "TASK-034RoYu0iSpPIN9pnjyhDU"
type: "task"
status: "done"
title: "Issue 276 T2: bounded writer + sinks + build result"
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
    - "docs/tasks/task-276-t1-config-and-wiring.md"
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

# Issue 276 T2: bounded writer + sinks + build result

## Work

`src/logging/writer.rs` に BoundedWriter(`sync_channel` 有界化・EventWriter・worker thread・
`WriterGuard` + drain/flush/join/5s timeout)を実装し mod 内 unit test を書く。
`src/logging/build.rs` に `build_sinks` 内部共通 + `build_logging` /
`build_logging_layers` を実装し、`LoggingBuildResult` / `LoggingLayersResult` /
`SinkDegraded` を公開。sink ごとの EnvFilter・Text/JsonLines fmt layer・RollingFileAppender
(rotation + `max_log_files`)+ Required/BestEffort/全 sink 失敗の分類を配線する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-276-generic-logging-adapter.md`
- `docs/designs/issue-276-generic-logging-adapter.md`
- `docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md`
- `docs/tasks/task-276-t1-config-and-wiring.md`

## Dependencies

- T1(config/error/health 型と feature 配線が前提)。

## Affected Files

- `crates/ruviform-observability/src/logging/writer.rs`(新規)
- `crates/ruviform-observability/src/logging/build.rs`(新規)
- `crates/ruviform-observability/src/logging/mod.rs`(re-export 完成)

## Done When

- [x] BoundedWriter が queue-full/WriteZero/io/flush/disconnect を別 counter に分類する。
- [x] `WriterGuard` が drain→flush→join(5s timeout・超過で `shutdown_abandoned`)する。
- [x] `build_logging` が `Dispatch`・guards・degraded・health を返し global を触らない。
- [x] `build_logging_layers` が `Box<dyn Layer<Registry>>` を返す。
- [x] writer.rs mod 内 unit test(E7–E10・join/flush)が通る。

## Verification

- [x] `mise run fmt-check` / `check` / `lint`
- [x] `cargo test -p ruviform-observability --features logging --lib`
- [x] `cargo clippy -p ruviform-observability --features logging --all-targets -- -D warnings`
