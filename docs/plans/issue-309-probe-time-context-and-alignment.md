---
id: "PLAN-034UjiuskEaR4norBLPJa8"
type: "plan"
status: "approved"
title: "Issue #309 — PROBE-TIME時間コンテキスト・整列契約の実装計画"
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
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md"
    - "docs/test-specs/issue-309-probe-time-alignment.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md"
    - "docs/designs/issue-309-probe-time-context-and-alignment.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-309-probe-time-alignment.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/280"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/280"
  deferred-by: []
---

# Issue #309 — PROBE-TIME時間コンテキスト・整列契約の実装計画 Implementation Plan

## Goal

`typed::probe` に `ProbeClockMark`（`RecordMeta` additive）・非 RT `ProbeTimeContext` registry・`align_observation_points` 三値契約・`ObservationCoverage` 再構成を実装し、複数 probe 観測の Frame/Clock/Epoch/Latency 整列可能性を型で判定可能にする（設計 dispatch で確定した契約の実装 — 本 plan は後続実装 dispatch 向け）。

## Scope

1 本の実装ストリーム（`probe.rs` 契約追加 → `runtime.rs`/`TapCtx` additive → 非 RT registry/align/coverage → テスト）。device session への clock timestamp 配線・`EngineFrame` anchor 生成は対象外（design の Deferred Concerns）。

## Upstream Documents

- `docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md`
- `docs/designs/issue-309-probe-time-context-and-alignment.md`
- `docs/adr/issue-309-probe-time-context-placement-and-alignment-contract.md`
- `docs/test-specs/issue-309-probe-time-alignment.md`

## Design Inputs

- `docs/designs/issue-309-probe-time-context-and-alignment.md`（D1–D8 の正本: hybrid mark 注入・media-core 語彙再利用・非 RT mint+`SetClockMark`・三値 alignment・coverage 再構成・#308 境界接続）
- `docs/designs/issue-308-probe-rebind-atomic.md`（世代境界・`RebindReceipt`/`CapturePolicy` の消費側）
- `docs/designs/issue-280-rt-record-bridge-clock-correlation.md`（OBS-05 Clock 語彙・RT bridge の再利用境界）

## File Map

- `crates/ruviform-audio/src/typed/probe.rs` — `ProbeClockMark`/`RecordMeta` additive 追加、`ProbeCommand::SetClockMark`（series/deck スコープ lifetime command）、deck `current_mark`、非 RT `ProbeTimeContext` registry、`ObservationSpan`/`Alignment`/`AlignmentBasis`/`IncomparableReason`/`AlignPolicy`/`ObservationCoverage`/`align_observation_points`
- `crates/ruviform-audio/src/typed/runtime.rs` — `TapCtx`/`RtState` への `current_mark` additive 拡張（`frame_cursor` は不変）
- `crates/ruviform-audio/tests/typed_probe.rs` or 新規 test — test-spec Rules の RED→GREEN
- `docs/impl/ir/issue-309-probe-time-context-and-alignment.md` — 実装 record（完了時）

## Implementation Sequence

- [ ] T1: `probe.rs` に `ProbeClockMark`・`RecordMeta` への additive field・`ProbeCommand::SetClockMark`・deck `current_mark`・`TapCtx` 経由の stamp を実装（mark 未配線は `UNKNOWN`）
- [ ] T2: 非 RT `ProbeTimeContext` registry（domain/epoch/runtime_instance 採番・generation 窓・`LatencyKnowledge`・anchors 受け皿）を実装し、`SetClockMark` 発行経路を broker/pair に接続
- [ ] T3: `ObservationSpan`/`AlignPolicy`/`Alignment`/`align_observation_points`・`ObservationCoverage` 再構成を実装（`Aligned`/`Conditional`/`Incomparable`・basis・error_budget・coverage 区間分類）
- [ ] T4: test-spec の Rule を RED→GREEN で実装（UNKNOWN/epoch/domain/rate/latency/coverage/generation 境界・RT no-alloc 非退行・PCM bit-identical）
- [ ] T5: `mise run fmt` → `check` → `lint` → `test` → `test-rt` を通し、impl record と task status を更新

## Dependencies and Risks

- 依存: #185 probe 基盤・#308 rebind（本 worktree に存在）・media-core `ClockDomainId`/`ClockEpochId`/`ClockAnchor`/`EngineFrame`/`TimeBase`・audio-core `same_clock_domain`。外部依存なし。
- リスク: `RecordMeta` サイズ増による telemetry 帯域影響 → 実装時に計測（design Risks）。
- リスク: `ClockDomainId` 採番 namespace が well-known 定数/device domain と衝突 → registry 採番規則を検証。
- リスク: device timestamp 未配線のため live device では epoch が `UNKNOWN` のまま → `Incomparable{UnknownClock}` で保守的（設計どおり）。

## Task Breakdown

- T1–T5 は直列（同一ファイル・同一契約）。独立並行ストリームなし。

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` / `mise run test-rt` 全緑
- [ ] test-spec の Rule（clock identity・basis/error_budget・generation 境界・coverage/無音区別・RT 契約）をカバーするテストが緑
- [ ] `assert_no_alloc` 系 RT 検証が非退行、probe ON/OFF/Capture で sink PCM bit-identical
- [ ] `git merge origin/main` 後に再検証

## Open Questions / Gaps

- `ProbeClockMark` の `RecordMeta` での正確な field 名・`Option` 包み方は実装時に確定（`UNKNOWN` domain を sentinel とするか `Option` にするかは型設計の詳細）。
- `align_observation_points` の公開 API 名・module 配置（`probe::time` 等）は実装 dispatch で確定。
- device clock anchor 供給（`OutputCallbackInfo`/`InputCallbackInfo` 配線）は別 Issue — 本 plan では `anchors` の受け皿のみ。

## Self-Review

- [x] Every spec requirement maps to a task.（AC-1→T1/T2、AC-2→T3、AC-3→T1、AC-4→T3、AC-5→T3、downstream AC→T4/T5）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（後続 dispatch）
- Independent work streams: none
- User approval required before dispatch: yes（worker 契約により review-protocol の blocker 無しを承認として扱う）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `ProbeClockMark` の `UNKNOWN` sentinel 設計と `SetClockMark` の series スコープ性、`align_observation_points` の `Incomparable` reason 網羅性と `error_budget` 算出根拠を重点レビュー。
