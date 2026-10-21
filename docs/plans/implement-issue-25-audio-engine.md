---
id: "PLAN-034JRQoIQSe5Sh13CkKfjN"
type: "plan"
status: "approved"
title: "Issue #25 音声エンジン実装"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/audio-engine.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/audio-engine-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #25 音声エンジン実装 Implementation Plan

## Goal

`ruviform-audio` エンジン新設（Issue #25 / spec `docs/specs/audio-engine.md`）。pull グラフ・ミキサー・スナップショット差替・デバイスセッション統合を実装する。

## Scope

新クレート `crates/ruviform-audio`。依存: `ruviform-audio-core`（device ポート・バッファ型）・`ruviform-rt`。dev-dep で `ruviform-audio-device` は使わない（モックで代替 — 公開 API に adapter 型を出さない）。

## Upstream Documents

- spec: `docs/specs/audio-engine.md`（approved）
- design: `docs/designs/audio-engine.md`（approved）
- ADR: `docs/adr/audio-engine-snapshot-swap.md`（accepted）
- test-spec: `docs/test-specs/audio-engine-tests.md`

## Design Inputs

- `RtProcessor`/`RtContext::mint`/`MasterClock`/`SPSC`（#21 `ruviform-rt`）
- `OutputStream`/`OutputStreamConfig`/`OutputDataCallback`（#23 `ruviform-audio-core::device`）
- `AudioBuffer`/`planar` バッファ型（#18 audio-core）

## File Map

- `crates/ruviform-audio/Cargo.toml` — `ruviform-rt`/`ruviform-audio-core` path dep
- `src/lib.rs` — crate doc + `pub mod graph`/`session`/`plan`/`mixer`
- `src/graph.rs` — `AudioNode` trait + `Graph`（pull 駆動）
- `src/mixer.rs` — `Mixer`（planar f32 和算）
- `src/plan.rs` — `GraphPlan`（不変・Send+Sync・`prepare` 済み）+ `PlanHandle`（`RwLock<Arc<_>>` + `try_read` + キャッシュ）
- `src/session.rs` — `Session::open_output(device, config, plan)` — callback → `RtContext::mint` → `plan.graph.process(out, ctx)`

## Implementation Sequence

1. T1 `audio-engine-graph-snapshot`: クレート骨格 + `graph`/`mixer`/`plan`（スナップショット交換）+ 単体テスト
2. T2 `audio-engine-session`: `Session` + callback 配線 + RT 契約テスト（assert_no_alloc/thread-check）
3. T3 `audio-engine-closeout`: 文書確定・検証列

## Dependencies and Risks

- `OutputStream` callback の正確なシグネチャは `device.rs` 実装を再確認してから合わせる
- `AudioBuffer` の RT 借用形状は `&mut` スライスビュー — buffer.rs の可変ビュー API を確認
- `PlanHandle` の `try_read` 競合は 1 ブロック遅延で吸収（ADR 採否済み）

## Task Breakdown

- `docs/tasks/audio-engine-graph-snapshot.md`（T1）
- `docs/tasks/audio-engine-session.md`（T2、depends-on T1）
- `docs/tasks/audio-engine-closeout.md`（T3、depends-on T2）

## Verification

- [ ] pull グラフがモックノード列を正しい順序で駆動
- [ ] ミキサーが planar 入力を和算して out に書く
- [ ] `rt_callback_no_alloc`（assert_no_alloc 窓内 0 alloc）がグリーン
- [ ] `plan_swap_under_run` が差替後に新プランを使い panic しない
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス
- [ ] doc 監査で本チェーン起因 findings 0

## Open Questions / Gaps

`OutputStream` callback の具体的シグネチャ・`AudioBuffer` 可変ビューの API 形状は実装時に `audio-core` ソースで再確認（設計で要求形は確定）。

## Self-Review

spec AC（クレート存在・E2E 出力・RT no-alloc・差替・デバイス型非露出・検証列）→ plan Verification の対応を確認済み。

## Implementation Handoff

- Recommended mode: <!-- inline execution or delegated or subagent-capable implementation -->
- Independent work streams: <!-- list task IDs or state none -->
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- <!-- assumptions, missing dependencies, or reviewer focus -->
