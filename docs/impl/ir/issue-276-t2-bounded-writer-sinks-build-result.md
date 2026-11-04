---
id: "IMPL-034RwLt9pOcXsRFEHwaQA8"
type: "impl"
status: "completed"
title: "Issue 276 T2: bounded writer + sinks + build result"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/276"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/src/logging/writer.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/logging/build.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/src/logging/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/logging/health.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/task-276-t2-writer-sinks-build.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/task-276-t1-config-and-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-276-generic-logging-adapter.md"
    - "docs/designs/issue-276-generic-logging-adapter.md"
    - "docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md"
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

# Issue 276 T2: bounded writer + sinks + build result

## Summary

`src/logging/writer.rs` に有界非ブロック writer(sync_channel +
sink 専用 worker thread + `WriterGuard`)を、`src/logging/build.rs` に
`build_logging` / `build_logging_layers` の 2 entry と sink 初期化
(EnvFilter 検証・RollingFileAppender・per-sink fmt layer)を実装した。
`tracing_appender::non_blocking` は queue-full/partial write/flush 失敗/
disconnect の区別計測を提供しないため ADR どおり自前実装。

## Context

- Task: `docs/tasks/task-276-t2-writer-sinks-build.md`(前提 T1)
- ADR: `docs/adr/issue-276-logging-adapter-bounded-writer-ownership-and-build-time-sink-resilienc.md`
- Spec / Design / Plan: 同名 issue-276 文書。

## Implementation

### Changes

- `writer.rs`: `WorkerMsg{Event(Vec<u8>), Shutdown}`、`BoundedSender`
  (MakeWriter 用)、`EventWriter`(fmt が複数 `write` を呼んでも Drop で
  1 event = 1 message にバッファ)、`WriterGuard`(drop で in-band
  Shutdown 送信→done wait→join、5s timeout で abandon+`shutdown_abandoned`)。
  worker は `write_all` 結果を `events_written`/`partial_writes`(WriteZero)/
  `write_failures` に分類し、Shutdown では marker 以後の滞留 event を
  `channel_disconnected` として計数してから flush+exit。sender 全 drop でも
  flush+exit。
- `build.rs`: `build_sinks` 内部共通(filter は全 sink で先に
  `EnvFilter::try_new` 検証 — failure_mode に依らず `InvalidFilter`)。
  Required → `RequiredSinkFailed` で即中止(構築済み guard は drop で
  正常 shutdown)。BestEffort → `SinkDegraded` 記録して継続。
  `layers.is_empty()` → `AllSinksFailed`(全滅を成功扱いしない)。
  `LoggingBuildResult{dispatch,guards,degraded,health}` /
  `LoggingLayersResult{layers,guards,degraded,health}`。
  `init_destination` で RollingFile は `create_dir_all` を自前で先に行い
  `RollingFileAppender::builder().rotation(..).filename_prefix(..)
  .max_log_files(..).build(..)` に渡す。
- `health.rs`: `SinkMetrics::snapshot` を `pub(crate)` 化(writer unit test
  から利用)。
- `mod.rs`: rotation 時ファイル作成失敗が上游で stderr 報告+旧ファイル継続
  される caveat を失敗モデルに追記。

### Decisions During Implementation

- Shutdown を in-band marker(`WorkerMsg::Shutdown`)にしたのは、guard が先に
  drop されても layer 側の `BoundedSender` が channel を生かし続けるケースを
  確実に終了させるため。marker 以前の event は FIFO で drain され、
  以後の straggler は `channel_disconnected` に計数する。
- `tracing-appender` の `build()` は `prune_old_logs` が `create_dir_all`
  より先に走るため、存在しない directory では `read_dir` 失敗を eprintln する
  上游の癖がある。自前の `create_dir_all` を先に入れて解消
  (invalid dir は io::Error として typed error 側に集約)。
- `build_logging` / `build_logging_layers` は `&LoggingConfig` を取る
  (clippy needless_pass_by_value 解消 + config を caller が保持できる)。
- `init_sink` の戻り値は `BuiltSink` 構造体化(type_complexity 解消)。

## Related Experiments

### Adopted

なし。

### Rejected

- `tracing_appender::non_blocking`: drop/failure observability が皆無のため
  不採用(ADR 記載どおり)。

## Validation

- `cargo test -p ruviform-observability --features logging` — 13 unit test
  pass(queue-full/WriteZero/write fail/flush fail/disconnect/drain+join/
  straggler 計数/shutdown timeout abandon)。
- `cargo clippy -p ruviform-observability --features logging --all-targets
  -- -D warnings` — pass。
- `mise run fmt-check` / `check` / `lint` — pass。

## Risks

- `tracing-appender` の rotation 時 `create_writer` 失敗は upstream が
  eprintln して旧ファイルへの書き込みを継続する。本 adapter の counter に
  到達しない経路であり、契約として mod.rs/README に明記済み。

## Follow-ups

- T3(契約試験 + example + README + 全検証)は後続 IR 参照。
