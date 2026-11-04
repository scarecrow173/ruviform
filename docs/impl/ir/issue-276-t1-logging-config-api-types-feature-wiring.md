---
id: "IMPL-034RwLrHlik3I1I7ljPS1y"
type: "impl"
status: "completed"
title: "Issue 276 T1: logging config/API types + feature wiring"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/276"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-observability/src/logging/mod.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/logging/config.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/logging/error.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/logging/health.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-observability/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/src/lib.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/task-276-t1-config-and-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/task-276-t1-config-and-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-276-generic-logging-adapter.md"
    - "docs/designs/issue-276-generic-logging-adapter.md"
    - "docs/plans/issue-276-generic-logging-adapter.md"
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

# Issue 276 T1: logging config/API types + feature wiring

## Summary

`ruviform-observability` に feature `logging`(既定 off)と optional 依存
`tracing` / `tracing-subscriber`(env-filter・fmt・json・std) /
`tracing-appender`(>= 0.2.3,`max_log_files` retention のため)を配線し、
`src/logging/` モジュール骨格と公開型(`LoggingConfig`・`SinkConfig`・
`SinkDestination`・`LogFormat`・`RotationPeriod`・`SinkFailureMode`・
`LoggingBuildError`・`SinkFailure`・Health 系)を実装した。
core counter pair の型・挙動は不変で、`--no-default-features` では
外部依存ゼロを維持する。

## Context

- Task: `docs/tasks/task-276-t1-config-and-wiring.md`
- Plan: `docs/plans/issue-276-generic-logging-adapter.md`
- Spec / Design: `docs/specs/issue-276-generic-logging-adapter.md` /
  `docs/designs/issue-276-generic-logging-adapter.md`
- Issue #276(受入正本)・#275(親設計)。

## Implementation

### Changes

- `Cargo.toml`: `logging` feature を追加し tracing 系 3 依存を optional 化。
  dev-dep として `serde_json`・`tempfile` を追加(契約試験用)。
  `[[test]] logging`・`[[example]] logging_host` を `required-features =
  ["logging"]` で宣言。
- `src/lib.rs`: `#[cfg(feature = "logging")] pub mod logging;` を追加。
  counter pair 側は無変更。
- `src/logging/mod.rs`: 所有権契約(Host が subscriber を所有・
  set_global_default/init を呼ばない・stdout 非出力・製品固有設定を読まない)、
  RT 契約(RT callback から通常 tracing を発行しない)、失敗モデル、
  センシティブデータ方針をモジュール文書として明記し re-export。
- `src/logging/config.rs`: `LoggingConfig`/`SinkConfig`/`SinkDestination`
  (Stderr/RollingFile{directory,filename_prefix,rotation,max_files,
  queue_capacity})/`LogFormat`/`RotationPeriod`/`SinkFailureMode` と
  `DEFAULT_STDERR_QUEUE_CAPACITY`。全 enum に `#[non_exhaustive]`。
- `src/logging/error.rs`: `LoggingBuildError`(NoSinks/InvalidFilter/
  RequiredSinkFailed/AllSinksFailed)+ `SinkFailure`、`Error::source` 実装。
- `src/logging/health.rs`: `LoggingHealth`/`HealthSnapshot`/
  `SinkHealthSnapshot`/`SinkState`/`SinkKind` と内部 `SinkMetrics`
  (atomic counter + Mutex 状態フラグ)/`HealthBuilder`。

### Decisions During Implementation

- `Stderr` sink には `queue_capacity` フィールドが Issue の API スケッチに
  無いため、公開面を増やさず `DEFAULT_STDERR_QUEUE_CAPACITY = 4096` の
  固定値を採用。
- dev-dep の `tracing`/`tracing-subscriber` 二重宣言は削除 — optional な
  通常依存は `required-features` 経由で test/example target に既に見える。
- `SinkMetrics` の hot counter は atomic のみ、`state`/`shutdown_abandoned`
  は Mutex 側に寄せて snapshot のコストを最小化。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo check -p ruviform-observability --no-default-features` — pass。
- `cargo check -p ruviform-observability --features logging --all-targets` — pass。
- `cargo tree -p ruviform-observability --no-default-features --edges normal` —
  normal dep ゼロを確認。
- `mise run fmt-check` / `mise run check` / `mise run lint` — pass
  (feature lane として `cargo clippy -p ruviform-observability --features
  logging --all-targets -- -D warnings` も pass)。

## Risks

- `tracing-subscriber` feature set(env-filter/fmt/json/std)は現在の使用面に
  最小化済み。将来の sink 種別追加で広げる場合は理由記録が必要。

## Follow-ups

- T2(writer + build)、T3(契約試験 + example + README + 全検証)は本 IR と
  同一ブランチの後続コミットとして実施済み(各自の IR 参照)。
