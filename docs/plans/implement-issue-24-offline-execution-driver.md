---
id: "PLAN-034JRcspAdty0HzWF0KzIr"
type: "plan"
status: "approved"
title: "Issue #24 オフライン実行ドライバ実装"
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
  implemented-by:
    - "docs/tasks/offline-exec-primitives.md"
    - "docs/tasks/offline-exec-parallel.md"
    - "docs/tasks/offline-exec-closeout.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/offline-execution-driver.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/offline-execution-driver-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #24 オフライン実行ドライバ実装 Implementation Plan

## Goal

`ruviform-exec` にオフライン実行ドライバを実装する（Issue #24 / spec `docs/specs/offline-execution-driver.md`）。`Processor`/`RtProcessor` をチャンク列に対し逐次・並列で駆動し、出力順序保存・進捗・キャンセル・バックプレッシャー・決定論保証範囲の明文化を達成する。

## Scope

対象クレート: `crates/ruviform-exec`（`ruviform-rt` への `path` 依存を追加）。公開 API のみ新設 — 既存 thread-priority API は不変。

## Upstream Documents

- spec: `docs/specs/offline-execution-driver.md`（approved）
- design: `docs/designs/offline-execution-driver.md`（approved — API 形状・決定論保証範囲・バックプレッシャー設計）
- test-spec: `docs/test-specs/offline-execution-driver-tests.md`

## Design Inputs

- `Processor`/`RtProcessor`/`RtContext`（#21、`ruviform-rt`）
- 発行規則: `RtContext::mint()` はドライバ内部のみ（ベースライン L302）
- 有界 `sync_channel` + `BTreeMap` 順序再構成（design §Data and Control Flow）

## File Map

- `crates/ruviform-exec/Cargo.toml` — `ruviform-rt = { path = "../ruviform-rt" }` 追加
- `crates/ruviform-exec/src/offline.rs` — `CancellationToken`/`Progress`/`OfflineConfig`/`OfflineDriver`/`OfflineError`
- `crates/ruviform-exec/src/lib.rs` — `mod offline` + 再エクスポート + crate doc 更新
- `crates/ruviform-exec/src/offline/tests`（インラインテスト、ワークスペース慣例）

## Implementation Sequence

1. T1 `offline-exec-primitives`: dep 追加 + トークン/Progress/OfflineError/逐次 `run`（workers=1 経路）
2. T2 `offline-exec-parallel`: `run_parallel`（workers>1、Send 境界シグネチャ分離）+ 有界チャネル + 順序再構成
3. T3 `offline-exec-closeout`: 文書確定・検証列

## Dependencies and Risks

- `Processor::Setup`/`Rt`/`Io` は全てジェネリック — 具象型は呼び出し側。テストはフィクスチャ `Processor`（加算・エコー）で検証
- リスク: `RtProcessor::process` がパニックした場合の join 伝播 → `OfflineError::WorkerPanicked` へ写像（design §Risks）

## Task Breakdown

- [x] `docs/tasks/offline-exec-primitives.md`（T1）
- [x] `docs/tasks/offline-exec-parallel.md`（T2、depends-on T1）
- [x] `docs/tasks/offline-exec-closeout.md`（T3、depends-on T2）

## Verification

- [ ] 逐次/並列の出力一致（workers ∈ {1,2,4,8}）テストがグリーン
- [ ] キャンセル即時停止・`Cancelled` 返却テストがグリーン
- [ ] `Progress` 正確性・prepare 失敗集約・ワーカーパニック写像テストがグリーン
- [ ] `mise run fmt-check`/`check`/`lint`/`test` パス、`unsafe` なし
- [ ] doc 監査で本チェーン起因 findings 0

## Open Questions / Gaps

なし（決定論保証範囲は design で確定）。

## Self-Review

spec AC → plan Verification の対応を確認済み。Design の Send 境界分離（逐次は非 Send 許容、並列のみ要求）がシグネチャ分割で表現されることを確認。

## Implementation Handoff

- Recommended mode: <!-- inline execution or delegated or subagent-capable implementation -->
- Independent work streams: <!-- list task IDs or state none -->
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- <!-- assumptions, missing dependencies, or reviewer focus -->
