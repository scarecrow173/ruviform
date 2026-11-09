---
id: "TASK-7xQm2PvL9wR4tN8sK3jH5f"
type: "task"
status: "done"
title: "Issue #293 OBS-11 ProbeSource adapter + 統合 edge 強化スライス"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/293"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio/src/typed/probe/source.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/typed_probe_source.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio/src/typed/probe.rs"
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-observability/tests/integration.rs"
      - type: "file"
        path: "crates/ruviform-observability/src/integration/runner.rs"
      - type: "file"
        path: "crates/ruviform-observability/README.md"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-293-observability-integration-runner.md"
  implemented-by:
    - "docs/impl/ir/issue-293-probe-source-adapter.md"
  depends-on:
    - "docs/plans/implement-issue-293-observability-integration-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/implement-issue-293-observability-integration-runner.md"
  refines:
    - "docs/tasks/implement-issue-293-observability-integration-runner.md"
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-293-observability-integration-runner.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-293-observability-integration-runner-tests.md"
  verified-by:
    - "docs/test-specs/issue-293-observability-integration-runner-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/292"
    - "https://github.com/scarecrow173/ruviform/issues/309"
    - "https://github.com/scarecrow173/ruviform/issues/308"
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/223"
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

# Issue #293 OBS-11 ProbeSource adapter + 統合 edge 強化スライス

実装記録: [issue-293-probe-source-adapter.md](../impl/ir/issue-293-probe-source-adapter.md)

## Work

Issue #293（OBS-11）の残存 slice。`ruviform_observability::debug::
ProbeSource`（#292 で定義）の具象 adapter を `ruviform-audio` 側に
実装し、`ProbeDeck`/`ProbeCollector` の drain 状態を
`ProbeDeckSnapshot`/`ProbeReading` へ写像する。
`DebugInspector::add_probe_source` と runner `debug_mut()` 経由の
配線を fixture で検証する。併せて統合 edge を強化する:
filter-change/dynamic reconfigure の `NonMigrated` 報告、
secret non-exposure の contract test、GUI pull 境界
（push `Subscription`/fan-out は設計上非存在）の明文化、
`test-observability-msrv` lane（Rust 1.89）の追加と実走。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/issue-293-observability-integration-runner.md`
- `docs/designs/issue-293-observability-integration-runner.md`
- `docs/plans/implement-issue-293-observability-integration-runner.md`
- `docs/test-specs/issue-293-observability-integration-runner-tests.md`
- `docs/tasks/implement-issue-293-observability-integration-runner.md`

## Dependencies

- Issue #292（OBS-10 Debug — `ProbeSource`/`ProbeDeckSnapshot`/
  `ProbeReading`/`DebugInspector::add_probe_source`）
- Issue #185（Generic Probe — `ProbeBroker`/`ProbeDeck`/
  `ProbeCollector`/`pair`）
- Issue #309（probe time context — `ProbeClockMark`/
  `ProbeTimeContext`/`wire_clock_mark`/`bump_epoch`）
- Issue #308（probe rebind/capture boundary policy）
- Issue #293 前段 slice（`integration` feature/`ObservationRunner`）

## Acceptance Checks

- [x] `ruviform-audio` に `ProbeSource` adapter 実装 — drain→
  `ProbeSource` 写像・inspector/runner 配線 fixture green
- [x] edge 強化 — filter-change/reconfigure（`NonMigrated` 報告・
  source 再登録）・secret non-exposure contract・GUI pull 境界明文化
- [x] `test-observability-msrv` lane 追加・Rust 1.89 で実走 green
- [x] Feature DAG 不変（調査記録のみ — impl record 参照）
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` + MSRV
  lane（結果は impl record に記録）

## Deferred

- AudioLens #138・Synvelis #81 の consumer 統合・両 Consumer 受入 —
  NotRun（#281/#219 cross-repo gate）
- RT no-alloc/latency の実機計測・実 OS（Linux/macOS）検証 — NotRun
  （#219/#223 gate）
- cross-repo gate（#281/#219/#294）・外部 consumer 受入 — NotRun
- Feature DAG（`collector`/`diagnostics`/`debug-tools` feature 分割）
  — 調査の結果 always-on が意図の可能性が高く変更せず（impl record
  の調査記録参照）
