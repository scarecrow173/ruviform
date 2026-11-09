---
id: "PLAN-034Ui3RAJGJJ1dJAlUW9N0"
type: "plan"
status: "approved"
title: "Issue #308 — Probe再Binding全件整合の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/308"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-308-probe-rebind-atomic.md"
    - "docs/designs/issue-308-probe-rebind-atomic.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-308-probe-rebind-atomic.md"
    - "docs/designs/issue-308-probe-rebind-atomic.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-308-probe-rebind-atomic.md"
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  deferred-by: []
---

# Issue #308 — Probe再Binding全件整合の実装計画 Implementation Plan

## Goal

`typed::probe` の `on_publish` を世代付き staged slots（`RebindBegin`/`RebindStage`/`RebindCommit`）による bounded all-or-nothing activation へ改め、`RebindReceipt`・`StatusKind::RebindDropped`・`StopReason::GenerationBoundary`・`CapturePolicy` を追加して、部分失敗の型付き報告と Capture 境界契約を実装する。

## Scope

1 本の実装ストリーム（probe.rs の契約変更 → テスト拡張 → 検証は同一 critical path・直列）。`runtime.rs`/`mod.rs`/`spsc.rs`/`Cargo.toml` は変更しない判断（design の代替案節を参照）。

## Upstream Documents

- `docs/specs/issue-308-probe-rebind-atomic.md`
- `docs/designs/issue-308-probe-rebind-atomic.md`
- `docs/test-specs/issue-308-probe-rebind-atomic.md`

## Design Inputs

- `docs/designs/issue-308-probe-rebind-atomic.md`（staged rebind・RebindReceipt・CapturePolicy の正本）
- `docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md`（既存 probe 契約）

## File Map

- `crates/ruviform-audio/src/typed/probe.rs` — `ProbeCommand` 変種差し替え（`ClearAll` 廃止、`RebindBegin`/`RebindStage`/`RebindCommit` 追加）、`ProbeDeck` へ `staged`/`staging` 追加と `commit_rebind`、`StatusKind::RebindDropped`・`StopReason::GenerationBoundary`・`CapturePolicy`・`RebindReceipt` 追加、`ProbeEntry.capture_policy`・`set_capture_policy`・`on_publish` 書き換えと doc 修正、`pair()` の staged arena 確保
- `crates/ruviform-audio/tests/typed_probe.rs` — queue 飽和点・連続切替・unresolved 明示・Capture boundary・Golden/Counter の新規テスト
- `docs/impl/ir/issue-308-probe-rebind-atomic.md` — 実装 record（完了時）

## Implementation Sequence

- [ ] T1: `probe.rs` に `RebindBegin`/`RebindStage`/`RebindCommit`・`staged`/`staging`・`commit_rebind`・新 status/policy/receipt 型を実装し、`on_publish` を staged 方式へ書き換え（doc/impl 不一致も同時修正）
- [ ] T2: `typed_probe.rs` に test-spec の Rule を RED→GREEN で実装（飽和点 4 パターン・retry・連続切替・unresolved・StopAtBoundary/Restart・Golden）
- [ ] T3: `mise run fmt` → `check` → `lint` → `test` → `test-rt` を通し、impl record と task status を更新

## Dependencies and Risks

- 依存: Issue #185 の `typed::probe` 実装（本 worktree に存在）。外部依存なし。
- リスク: queue 容量 `2+N` 化で既存テストが容量前提で失敗する可能性 → `ProbeConfig::default().command_capacity = 64` は max_probes=32+α で十分。
- リスク: `install(mode=Capture)` の `capture_remaining=0` quirk は対象外として維持（design に記録）。

## Task Breakdown

- T1–T3 は直列（同一ファイル・同一契約）。独立並行ストリームなし。

## Verification

- [ ] `mise run fmt-check` / `mise run check` / `mise run lint` / `mise run test` / `mise run test-rt` 全緑
- [ ] 新規テストが queue 飽和点・連続切替・Capture boundary を決定的にカバー
- [ ] `git merge origin/main` 後に再検証

## Open Questions / Gaps

- `RebindDropped` の status が telemetry overflow で欠落した場合の Consumer 側扱いは `lost_records` counter 依存（仕様として許容 — design に記録）。
- `ProbeCommand::ClearAll` を削除する破壊的 API 変更は pre-release のため許容（呼出し側はリポジトリ内テストのみ）。

## Self-Review

- [x] Every spec requirement maps to a task.（R1→T1、R2→T1+T2、R3→T1+T2、R4→T1+T2、R5→T2、R6→T1）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: yes（worker 契約により review-protocol の blocker 無しを承認として扱う）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `commit_rebind` の swap/status 順序と `staging` epoch の fail-closed 性、`on_publish` の receipt 内容と doc の正直さを重点レビュー。
