---
id: "IMPL-uQYRObw7mzwY2AGJRMgIkm"
type: "impl"
status: "completed"
title: "Issue #293 OBS-11 ObservabilityConfig + ObservationRunner 実装記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/293"
  changes:
    added:
      - "crates/ruviform-observability/src/integration/"
      - "crates/ruviform-observability/tests/integration.rs"
    modified:
      - "crates/ruviform-observability/Cargo.toml"
      - "crates/ruviform-observability/src/lib.rs"
      - "crates/ruviform-observability/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-issue-293-observability-integration-runner.md"
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

# Issue #293 OBS-11 ObservabilityConfig + ObservationRunner 実装記録

## Summary

Issue #293（OBS-11）の `integration` feature を実装する。
`ObservabilityConfig`（Collector/Metrics/Diagnostics/Debug/
tracing-export の独立設定 — `LoggingConfig` は無条件統合しない）と
`ObservationRunner`（非 RT poll 駆動・Source 登録・Snapshot 照会・
停止/Drain レポート）を、既存公開 API の消費のみで構成する。
ライフサイクル `Unconfigured → Prepared → Ready → Running →
Draining → Stopped`、動的設定 `Validate → Prepare → Activate →
Retire`、logging 非依存・非再帰の `RunnerHealth` snapshot、
Host 注入 `tracing::Dispatch`（global install なし）を提供する。

## Context

`docs/tasks/implement-issue-293-observability-integration-runner.md`
の実装スライス。上流: `docs/specs/issue-293-observability-integration-runner.md`、
`docs/designs/issue-293-observability-integration-runner.md`、
`docs/test-specs/issue-293-observability-integration-runner-tests.md`。
消費する既存契約: OBS-07 `collector`（`Collector`/`CollectorRegistrar`/
`CollectorHandle`/`ObservationBatch`/`CollectorHealth`）、OBS-09
`diagnostics`（`Diagnostics`/`DiagnosticsConfig`/`DiagnosticSnapshot`/
`TracingFindingSink` 連携）、OBS-10 `debug`（`DebugInspector`/
`DebugConfig`/`DebugAuthority`/`SessionPermit`）、OBS-03
`tracing_export`（`TracingExporter`/`EmitStats`）、OBS-01 `logging`
（`build_logging`/`WriterGuard`/`LoggingHealth` — Host 注入のみ）。

## Implementation

### Changes

- `crates/ruviform-observability/src/integration/` 新設
  （`config.rs`/`runner.rs`/`health.rs`/`mod.rs`/`tests.rs`）。
  - `config.rs`: `ObservabilityConfig`（`collection` は
    `CollectorConfig` をそのまま権威とし Metrics 予算を内包、
    `diagnostics`/`debug` は `Option` で無効化可能、
    `export` は `tracing-export` feature 時のみ存在）、
    `ExportPolicy`（Host 注入 `Option<tracing::Dispatch>`・
    per-poll emission cap・diagnostics event policy）、
    `DebugSettings`（`DebugConfig` + optional `Arc<DebugAuthority>`）、
    `ConfigError`。`validate()` は各 subsystem の既存 validator へ
    委譲し新ルールを発明しない。
  - `runner.rs`: `ObservationRunner` — `prepare`（validate +
    Components 完全構築、live state 非接触）→ `activate`
    （Prepared→Ready）→ `poll_once`（Ready→Running 遷移を兼ね、
    `Collector::poll` → optional `Diagnostics::evaluate` →
    optional `TracingExporter::emit` を非 RT で駆動）→
    `begin_shutdown`（Running→Draining）→ `finish_shutdown`
    （Collector 最終 poll → `Collector::shutdown` terminal drain
    → terminal batch export → logging guard drop/flush/close の
    固定順序）/ one-shot `shutdown`。
    `reconfigure` は新 config を validate + 完全構築してから
    live state を swap し、旧 component set を retire/drain。
    source 登録・diagnostics retention・debug session・exporter
    baseline は移行せず `ReconfigureReport.non_migrated` で明示。
    `attach_logging` は Prepared/Ready/Running のみ受理し
    `WriterGuard` + `LoggingHealth` を Host から注入する。
  - `health.rs`: `RunnerHealth` — lifecycle/poll/shutdown/
    reconfigure 集計 + collector/diagnostics/debug/debug-session/
    logging 各 snapshot への参照 + tracing-export `ExportTotals`
    （`EmitStats` の runner 生涯累積）。atomic/値コピーのみで
    tracing emit・sink アクセスを行わない非再帰 snapshot。
  - `tests.rs`: ライフサイクル・poll・drain 順序・reconfigure・
    health の unit test。
- `Cargo.toml`: `integration = []` feature 追加（依存ゼロ）、
  `[[test]] name = "integration" required-features = ["integration"]`。
- `src/lib.rs`: module doc + `#[cfg(feature = "integration")] pub mod integration;`。
- `tests/integration.rs`: contract test — lifecycle、source poll、
  injected dispatch scoped/global 分離、Logging OFF/attach、
  unquiesced partial drain、debug session health、feature 組合せ。
- `README.md`: `integration` feature 節（起動・通常停止・異常停止・
  Logging OFF・Debug Session の使用例）。

### Decisions

- Runner はスレッド/タイマーを所有せず Host が `poll_once` を呼ぶ
  — RT/非 RT 境界を Host に残すため。
- 停止時の logging health は guard drop 後も attachment として保持
  （`health().logging` が shutdown 後も読める）。guard のみ drain
  して health handle は残す設計に修正済み。
- Dispatch test の source は `ReportingHint::OnIncrease` を使用 —
  `SnapshotOnly` は export policy で抑制され event が出ないため。

## Verification

- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt`: すべて
  exit 0（workspace default features。`integration` は default off の
  ため feature-gated code は下記補助証拠で検証）。
- 補助証拠（canonical = mise、直接 cargo）:
  - `cargo clippy -p ruviform-observability --all-targets
    --all-features -- -D warnings`: exit 0
  - `cargo test -p ruviform-observability --all-features`: lib 250 +
    integration contract 11 + 他 feature test binary すべて passed
  - feature 組合せ `integration` / `integration,logging` /
    `integration,tracing-export` の clippy + test: すべて passed
- review-protocol（subagent 不使用の分離パスで実施）: Reviewer A
  （協調）/ Reviewer B（敵対）→ Validator → Judge。CONFIRMED P2 を
  1 件修正 — `Stopped → prepare` restart 後に drain 済み logging
  attachment が残り `attach_logging` が `AlreadyAttached` で永久
  拒否される問題（`prepare` で attachment をクリア +
  `restart_reattaches_fresh_logging` 回帰試験追加）。その他の候補は
  P2/P3 non-blocking（runner 生涯 counter・`last_shutdown` の restart
  越え保持、`Draining` 中の one-shot `shutdown` 不可は documented
  契約、reconfigure drain の `Instant::now()` 使用）。判定:
  ACCEPT WITH NON-BLOCKING NOTES。
