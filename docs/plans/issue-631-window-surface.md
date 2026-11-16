---
id: "PLAN-034YJ2M9f2fgfWOT7UhjEs"
type: "plan"
status: "approved"
title: "Issue #631 RealOsGuiTest window-close/DPI 観測面 実装計画"
created: "2026-11-16"
updated: "2026-11-16"
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
    - "docs/specs/issue-631-window-surface.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-window-surface.md"
    - "docs/designs/issue-631-window-surface.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 RealOsGuiTest window-close/DPI 観測面 実装計画 Implementation Plan

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

- 前提: `WindowDriver` close 契約・`GpuiDriver::window_geometry`・
  `flush_events` の handler-detach 設計は実装確認済み。sentinel
  window 要件は実走で確定（GPUI loop は最終 window close で終了）。
- reviewer focus: `arm_close_response` のタイミング契約
  （同期 flush 内消費のみが veto/defer に間に合う）と、
  `window_events` の append-only / `Action` 除外の境界。

## スコープ分割

単一 slice（#631 window-surface slice）。T1 のみ — 観測面の追加と
実 window 検証。catalog row flip・plugin-dock evidence 再生成は
後続 slice。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs` | `WindowLifecycleEvent` 追加・session log/armed queue・public methods・handler 拡張 |
| `crates/ruviform-ui-gpui/src/lib.rs` | `WindowLifecycleEvent` re-export 1 行（Coordinator 承認済み） |
| `crates/ruviform-ui-gpui/tests/real_window_lifecycle.rs` | 新規 — close lifecycle + DPI geometry の実 window 検証 |
| `evidence/rf631/real-window-surface.jsonl` | 新規 — `rf631-real-window-surface/v1` run/case evidence |
| `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-631-window-surface.*` | doc chain |

## 依存グラフ

`WindowLifecycleEvent`/`observe` → handler 拡張（events log +
close_responses 消費）→ public methods → test file → evidence。
`lib.rs` re-export は public methods と独立に compile 可能だが、
公開面の完成として同一 commit に含める。

## リスクと rollback

- `Veto`/`Defer` のタイミング: armed queue が `CloseRequested`
  flush 内でのみ消費されることを test が直接検証（veto で window
  生存・defer で close-pending・その後の approve で destroy）。
- 最終 window 破棄で GPUI loop 終了 → UiJob pump 死亡: test は
  sentinel window を残して全観測を pump 生存下で行う（実走で
  timeout を再現済み・修正済み）。
- rollback: 追加は `real-scenario` feature 内の新規 API のみで
  既存挙動を変えない — revert で完全に除去可能。

## 検証マトリクス

| 検証 | コマンド |
| --- | --- |
| format | `mise run fmt-check` |
| check | `mise run check ruviform-ui-gpui --features real-scenario` |
| lint | `mise run lint ruviform-ui-gpui --features real-scenario` |
| unit/feature test | `mise run test ruviform-ui-gpui --features real-scenario` |
| default-feature test | `mise run test ruviform-ui-gpui` |
| scenario contract | `mise run test-ui-scenario-contract` |
| driver boundary | `mise run check-ui-driver-boundary` |
| 実 window lane | `cargo test -p ruviform-ui-gpui --features real-scenario --test real_window_lifecycle`（interactive desktop 前提） |

## Open Questions / Gaps

- なし — sentinel window 要件・armed queue 契約・evidence schema は
  実走で確定済み。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker — 単一 slice、
  依存は直列）。
- Independent work streams: none。
- User approval required before dispatch: yes（worker contract により
  review-protocol の blocker 無しを承認として扱う）。
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- spec `docs/specs/issue-631-window-surface.md`、design
  `docs/designs/issue-631-window-surface.md`、discovery
  `docs/discovery/issue-631-window-surface.md` を参照。
