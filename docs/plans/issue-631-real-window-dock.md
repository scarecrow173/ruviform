---
id: "PLAN-034XTP6V40ZLnW8sMLZK2N"
type: "plan"
status: "approved"
title: "Issue #631 Real-window native-slot dock lane 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
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
  implements:
    - "docs/specs/issue-631-real-window-dock.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-real-window-dock.md"
    - "docs/designs/issue-631-real-window-dock.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Real-window native-slot dock lane 実装計画 Implementation Plan

## Goal

<!-- One sentence describing what this plan builds. -->

## Scope

<!-- State whether this is one implementation stream or should be split into
separate plans. Call out the boundary explicitly. -->

## Upstream Documents

- <!-- spec or ADR path -->

## Design Inputs

- <!-- approved design doc path under docs/designs/ -->

## File Map

- `path/to/file.ts` - <!-- responsibility -->
- `path/to/test.ts` - <!-- responsibility -->

## Implementation Sequence

- [ ] <!-- one action only -->
- [ ] <!-- one action only -->

## Dependencies and Risks

- <!-- dependency, blocker, or risk -->

## Task Breakdown

- <!-- task candidate or decomposition rule -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->

## Open Questions / Gaps

- <!-- explicitly note what is unknown -->

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: <!-- inline execution or delegated or subagent-capable implementation -->
- Independent work streams: <!-- list task IDs or state none -->
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- <!-- assumptions, missing dependencies, or reviewer focus -->

## スコープ分割

単一 slice（second slice of #631）。T1 のみ。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs` | slot ops 追加・capability gating・module docs・test 更新 |
| `crates/ruviform-plugin-dock-test/Cargo.toml` | `real-dock` feature・optional dep・gated bin |
| `crates/ruviform-plugin-dock-test/src/lib.rs` | `jsonl`/`real_dock` module 配線 |
| `crates/ruviform-plugin-dock-test/src/real_dock.rs` | 新規 — 4 case lane |
| `crates/ruviform-plugin-dock-test/src/jsonl.rs` | 新規 — JSONL render/repo SHA helper |
| `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-scenario.rs` | `jsonl` helper 利用に refactor（出力不変） |
| `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs` | 新規 — real lane bin |
| `crates/ruviform-plugin-dock-test/README.md` | real-dock lane 説明追記 |
| `mise.toml` | `[tasks.test-ui-plugin-dock-real]` 追加のみ |
| `Cargo.lock` | optional dep 追加に伴う再生成のみ |
| `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-631-real-window-dock.*` | doc chain |
| `evidence/rf631/plugin-dock-real-scenario.jsonl`, `verification.md` | 実走 evidence |

## 依存グラフ

`real_dock.rs` → `ruviform-ui-test`（scenario contract・plugin_dock
helpers）→ `ruviform-plugin-host`（session/ops）→ `ruviform-ui-gpui`
（real-scenario feature・`RealOsGuiTest`）→ `ruviform-ui`（NativeSlot
語彙）。optional dep により default lane は不変。

## リスクと rollback

- 実 window が対話 session を要求 → headless は NotRun 記録で
  fail-closed。rollback は feature を使わないだけで default lane に影響なし。
- `real_window.rs` の既存 test 更新は predicate 連動のみ — 無条件
  contains 化はしない。

## 検証マトリクス

| 検証 | コマンド |
| --- | --- |
| fmt | `mise run fmt-check` |
| ui-gpui check/lint/test | `mise run check/lint/test ruviform-ui-gpui` |
| dock-test check/lint/test（default + `--features real-dock`） | `mise run check/lint/test ruviform-plugin-dock-test [--features real-dock]` |
| 非回帰 | `mise run test-ui-plugin-dock`, `test-ui-scenario-contract`(+fake/gpui), `test-ui-gpui`, `check-ui-driver-boundary` |
| real lane | `mise run test-ui-plugin-dock-real`（manual/opt-in） |
| hooks | `mise exec -- prek run --all-files` |
