---
id: "TASK-034UjjbcehweeswcNaA3No"
type: "task"
status: "done"
title: "Issue #309 — PROBE-TIME時間コンテキスト・整列契約の実装タスク"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-309-probe-time-context-and-alignment.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-309-probe-time-context-and-alignment.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-309-probe-time-alignment.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #309 — PROBE-TIME時間コンテキスト・整列契約の実装タスク

## Work

`crates/ruviform-audio/src/typed/probe.rs` に `ProbeClockMark`（`RecordMeta` additive）・`ProbeCommand::SetClockMark`（series/deck スコープ lifetime command）・deck `current_mark`・非 RT `ProbeTimeContext` registry・`ObservationSpan`/`AlignPolicy`/`Alignment`/`align_observation_points`・`ObservationCoverage` 再構成を実装し、`runtime.rs` の `TapCtx`/`RtState` に `current_mark` を additive 拡張する。plan T1–T5 を直列実行（実装は後続 dispatch — 本 dispatch は docs のみ）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-309-probe-time-context-and-alignment.md`
- `docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md`
- `docs/designs/issue-309-probe-time-context-and-alignment.md`
- `docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md`
- `docs/test-specs/issue-309-probe-time-alignment.md`

## Dependencies

- Issue #185 `typed::probe` 基盤・Issue #308 rebind（worktree 内に存在）。外部 blocker なし。
- media-core `ClockDomainId`/`ClockEpochId`/`ClockAnchor`/`EngineFrame`/`TimeBase`・audio-core `same_clock_domain`（既存依存を再利用）。

## Affected Files

- `crates/ruviform-audio/src/typed/probe.rs`
- `crates/ruviform-audio/src/typed/runtime.rs`
- `crates/ruviform-audio/tests/`（typed_probe 系テスト）
- `docs/impl/ir/issue-309-probe-time-context-and-alignment.md`（実装 record）

## Done When

- [x] `ProbeClockMark` が `RecordMeta` に additive 追加され、mark 未配線は `UNKNOWN` domain で `Incomparable{UnknownClock}` に倒れる
- [x] `ProbeTimeContext` registry が domain/epoch/runtime_instance/generation 窓/latency/anchors を非 RT で保持する
- [x] `align_observation_points` が `Aligned{basis,offsets,error_budget}`/`Conditional`/`Incomparable{reason}` を返し、latency 不明・別 epoch・rate 不一致・generation 跨ぎを無条件 Aligned にしない
- [x] `ObservationCoverage` が `Unobserved`/`LostBlocks`/`LostRecords`/generation boundary/Probe OFF/`Unknown` を区別し、測定無音と未観測を混同しない
- [x] test-spec の Rule をカバーするテストが RED→GREEN、`assert_no_alloc` 系 RT 検証と PCM bit-identical が非退行

## Verification

- [x] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` / `mise run test-rt` 全緑
- [x] `git merge origin/main` 後の再検証
