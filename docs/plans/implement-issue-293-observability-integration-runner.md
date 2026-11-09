---
id: "PLAN-BMe1olaQdn0e0jQ5JgPxiT"
type: "plan"
status: "approved"
title: "Issue #293 OBS-11 ObservabilityConfig/ObservationRunner 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/293"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-293-observability-integration-runner.md"
    - "docs/designs/issue-293-observability-integration-runner.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-293-observability-integration-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-292-observability-debug-inspection.md"
    - "docs/plans/implement-issue-291-observability-diagnostics.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-293-observability-integration-runner.md"
    - "docs/designs/issue-293-observability-integration-runner.md"
    - "docs/test-specs/issue-293-observability-integration-runner-tests.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-293-observability-integration-runner-tests.md"
  verified-by:
    - "docs/test-specs/issue-293-observability-integration-runner-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/synvelis/issues/81"
  deferred-by: []
---

# Issue #293 OBS-11 ObservabilityConfig/ObservationRunner 実装計画

## Goal

`crates/ruviform-observability/src/integration/` に
`ObservabilityConfig`（subsystem 独立設定・`LoggingConfig` 非統合）
と `ObservationRunner`（`Unconfigured→Prepared→Ready→Running→
Draining→Stopped`・非 RT `poll_once` 駆動・段階的 drain・
`reconfigure`・Host 注入 dispatch・Logger 非依存 `RunnerHealth`）
を実装し、Issue #293 の observability 内完結 slice の受入条件を
満たす。

## Scope

単一実装 stream。`src/integration/` 新設 + `lib.rs` への
feature-gated module 登録 + `Cargo.toml` の `integration = []`
feature 追加 + crate README/README crate doc + `tests/integration.rs`
のみ。既存 collector/metrics/source/record/rt_bridge/diagnostics/
debug/profiling/tracing_export/logging の公開 API は消費のみで
変更しない。他 crate・CI・生成物への変更なし。

## Upstream Documents

- `docs/specs/issue-293-observability-integration-runner.md`

## Design Inputs

- `docs/designs/issue-293-observability-integration-runner.md`
- `docs/test-specs/issue-293-observability-integration-runner-tests.md`

## File Map

- `crates/ruviform-observability/Cargo.toml` — `integration = []`
  feature 追加（依存追加なし）。`tests/integration.rs` 用に
  `[[test]] required-features = ["integration"]` 追加。
- `crates/ruviform-observability/src/lib.rs` — `#[cfg(feature =
  "integration")] pub mod integration;` 追加と crate doc の module
  一覧・feature 表更新（additive のみ）。
- `crates/ruviform-observability/src/integration/mod.rs` — module
  doc（責務・非 RT 契約・drain 順序・feature 構成）+ re-export。
- `crates/ruviform-observability/src/integration/config.rs` —
  `ObservabilityConfig`/`DebugSettings`/`ConfigError`/`ExportPolicy`
  （`#[cfg(feature = "tracing-export")]`）。
- `crates/ruviform-observability/src/integration/runner.rs` —
  `ObservationRunner`/`RunnerState`/`PollOutcome`/`RunnerError`/
  `ShutdownReport`/`DrainCompleteness`/`ReconfigureReport`/
  `attach_logging`（`#[cfg(feature = "logging")]`）。
- `crates/ruviform-observability/src/integration/health.rs` —
  `RunnerHealth`/`ExportTotals`。
- `crates/ruviform-observability/src/integration/tests.rs` —
  module 内 unit test。
- `crates/ruviform-observability/tests/integration.rs` —
  contract test（global/scoped dispatch・logging attach・Logging
  OFF・Debug Session health・feature 組合せ）。
- `crates/ruviform-observability/README.md` — 起動・通常停止・
  異常停止・Logging OFF・Debug Session の使用例と制約。

## Implementation Steps

1. `Cargo.toml` に `integration = []` feature と `[[test]]` を追加。
2. `config.rs`: `ObservabilityConfig`/`DebugSettings`/`ConfigError`
   を実装。`validate()` は `CollectorConfig::validate` /
   `DiagnosticsConfig::validate` に委譲。
3. `health.rs`: `ExportTotals`（`EmitStats` 累積）と `RunnerHealth`
   合成型を実装。
4. `runner.rs`: `Components`（collector/diagnostics/debug/exporter
   の所有）、`RunnerState`、全 lifecycle API、`poll_once` 合成、
   `begin_shutdown`/`finish_shutdown`/`shutdown` の drain 順序、
   `reconfigure` の Validate→Prepare→Activate→Retire、
   `attach_logging`（cfg）、accessors、`Drop` best-effort drain。
5. `mod.rs`: module doc + re-export。
6. `lib.rs`: module 登録・crate doc 更新。
7. `tests.rs`（unit）+ `tests/integration.rs`（contract）。
8. README 例と制約を追記。

## Verification Matrix

| 検証 | コマンド | 期待 |
| --- | --- | --- |
| fmt | `mise run fmt-check` | pass |
| check | `mise run check` | pass |
| lint | `mise run lint` | pass（clippy `-D warnings`） |
| test | `mise run test` | pass |
| RT lane | `mise run test-rt` | pass（integration は RT 非対象だが回帰確認） |
| feature 組合せ | `mise exec -- cargo check -p ruviform-observability --no-default-features --features integration` 等 | pass（default / integration / +tracing-export / +logging / +allocation-probe / --all-features / no-default+integration） |

## Risks and Mitigations

- **reconfigure で source/diagnostics/debug 状態が非継承になる**:
  設計で `ReconfigureReport` に記録する契約とし、「完全保存」を
  主張しない。CollectorConfig 不変のため新 Collector 構築が唯一の
  正直な経路。
- **`Arc<DebugAuthority>` の共有**: Host が permit 発行用
  authority を保持できるよう `Arc` で受け取る設計。`DebugAuthority`
  は `Send` でない可能性があるため runner の auto trait に注意
  （`ObservationRunner` は `Send` を要求しない — Host thread 所有）。
- **`finish_shutdown` の途中失敗**: collector shutdown は infallible、
  exporter emit は infallible（none dispatch 含む）、guard drop は
  infallible — 失敗経路は poll のみで `poll_errors_total` に計上
  して継続。report は常に生成される。
- **feature 組合せの cfg 複雑化**: `export` field・`attach_logging`・
  exporter 保持を `#[cfg]` で局所化し、feature 無し側は型ごと
  存在しない構成にする（fake type を置かない）。

## Rollback

`src/integration/`・`tests/integration.rs`・`Cargo.toml` の
feature/test 追加・`lib.rs` の module 登録・README 節を revert
すれば完全に除去できる。既存 module への変更はないため影響は
integration feature 利用者のみ。

## Deferred

spec の Deferred Design Concerns と同一（consumer 受入・RT 実機・
他 OS/MSRV・probe adapter・外部 exporter・LoggingConfig 統合・
GUI/IPC）。
