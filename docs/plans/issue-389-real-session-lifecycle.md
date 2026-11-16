---
id: "PLAN-rf389-real-session-lifecycle-01"
type: "plan"
status: "approved"
title: "Issue #389 real session lifecycle remainder 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-389-real-session-lifecycle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-real-session-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-real-session-lifecycle.md"
  blocks:
    - "docs/tasks/issue-389-real-session-lifecycle.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-389-real-session-lifecycle.md"
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-real-session-lifecycle.md"
    - "docs/designs/issue-389-real-session-lifecycle.md"
  derived-by:
    - "docs/tasks/issue-389-real-session-lifecycle.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-real-session-lifecycle.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 real session lifecycle remainder 実装計画

## Goal

`rf631.plugin-dock-real` catalog に session lifecycle 残件 6 行を
追加し（15 → 21）、degraded/recovery/timeout を Pass oracle、
visibility を defined-refusal `Unsupported`、crash/hang を正直
`Unsupported` 境界として real OS window で判定する。

## Scope

単一実装ストリーム。`crates/ruviform-plugin-dock-test/` crate 内
（feature `real-dock` 配下の `real_dock.rs` + GPUI-free
`real_catalog.rs` + console test + `README.md`）に閉じる。
plugin-host/plugin-vst3/plugin-clap・`mise.toml`・
`Cargo.toml`/`Cargo.lock`・docs index は変更しない。

## Upstream Documents

- `docs/specs/issue-389-real-session-lifecycle.md`
- `docs/designs/issue-389-real-session-lifecycle.md`
- `docs/specs/issue-389-editor-session.md`（`real-session-
  terminate` 先例）

## File Map

- `src/real_catalog.rs` — `REAL_CATALOG` 末尾へ 6 行追加、module
  doc 追従。
- `src/real_dock.rs` — `real_entries()` 21 entry 化、
  `attempt_real_attach` の `drive_open` seam 一般化 +
  `drive_open_docked` 先例、`real_editor_ops` redeemer factory
  抽出、case 本体（degraded/recovery/timeout/visibility/
  crash/hang）と observe helper、blocked/test 行数追従、module
  doc 追従。
- `tests/console.rs` — 23 → 29 行・期待 case 一覧追従。
- `README.md` — 行数・verdict 表追従。
- `docs/designs/issue-389-plugin-editor-session.md` — S4 節と
  して本 slice の capability/DoD reconcile を追記。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp,
  discovery}/issue-389-real-session-lifecycle.*`、`evidence/rf389/`。

## Implementation Sequence

- [ ] `real_catalog.rs`: 6 行追加（degraded/recovery/timeout/
  visibility は `NativePluginDock`、crash/hang は `required: &[]`）。
- [ ] `real_dock.rs`: `real_entries()` を `[...; 21]` 化。
- [ ] `real_dock.rs`: `attempt_real_attach` に `drive_open`
  クロージャ引数を追加し、open 呼出しと `session.open(Docked)`
  step row を `drive_open` 側へ移す。`drive_open_docked` が現行
  step row を逐語再現。既存 caller（`editor_child_body`/
  `editor_op_body`/dcomp bodies）を更新。
- [ ] `real_dock.rs`: `real_editor_ops`（`InProcessEditorOps`
  + `redeem_native_lease` redeemer）を `open_session_docked`
  から抽出し、新 drive で再利用。
- [ ] `real_dock.rs`: degraded — `observe_session_degraded_
  lifecycle`（child contract → close → teardown → events
  `[Degraded{Docked}, Closed]`）+ drive（`open_or_degrade`）。
- [ ] `real_dock.rs`: recovery — drive（`open(Floating)` refused
  確認 → `recover(Docked)`）+ observe（events
  `[Failed{Unsupported}, Recovered{Docked}, Closed]`）。
- [ ] `real_dock.rs`: timeout — observe（deadline ZERO →
  `begin_detach` → `request_deadline` → `tick_all==1` →
  `Failed{Timeout}` → close → events
  `[Attached{Docked}, Failed{Timeout}, Closed]`）。
- [ ] `real_dock.rs`: visibility — `observe_editor_visibility_
  lifecycle`（`set_visibility` refusal → `Attached` 維持 →
  分岐 event 期待）→ `EditorOpVerification` → `editor_op_body`。
- [ ] `real_dock.rs`: crash/hang — `run_scenario` body が
  `Unsupported` を返す case fn 2 本。
- [ ] `real_dock.rs`: `blocked_reports`/`blocked_probe_
  violations`/`mod tests` の `[...; 15]`→21・`len()==15`→21
  追従、module doc case 表・行数記述追従。
- [ ] `tests/console.rs`: 23→29 行・期待 case 配列追従。
- [ ] `README.md`: "fifteen"→"twenty-one"・verdict 表 6 行追加。
- [ ] 検証 lane 実走と `evidence/rf389/` 記録、design reconcile、
  review-protocol。

## Dependencies and Risks

- `open_gui` は view 既存で `PluginFailed` — recovery の `Failed`
  起点は refused open（`editor_live=false`）に限る（design 確定）。
- `drive_open` 一般化は既存 evidence の step 名を変えないことが
  前提 — `drive_open_docked` が逐語再現し、feature test と実走
  JSONL で確認する。
- `real-dcomp-pixel` の既知 `Fail` は lane exit 1 の正直な
  regression 信号であり、本 slice で緩和しない。
- `docs/**/README.md` index は生成物 — Worker は編集・再生成
  しない。Coordinator が統合時に再生成する。

## Task Breakdown

- T1: catalog/entry 追加と `drive_open` seam 一般化。
- T2: 6 case 本体 + observe helper 実装。
- T3: console test・README 追従。
- T4: 検証 lane 実走と `evidence/rf389/` 記録。
- T5: doc chain 完結（ir/exp/discovery）・design reconcile・
  review-protocol。

## Verification

- [ ] `mise run check/lint/test ruviform-plugin-dock-test`
  （default feature）。
- [ ] `mise run check/lint/test ruviform-plugin-dock-test
  --features real-dock`。
- [ ] `mise run check/lint ruviform-plugin-dock-test
  --features lab-gui`。
- [ ] `mise run build ruviform-plugin-test-vst3`。
- [ ] `mise run test ruviform-plugin-host --test editor_session
  --test editor_session_driver`。
- [ ] `mise run test-ui-plugin-dock`。
- [ ] `mise run test-ui-plugin-dock-console`。
- [ ] `mise run test-ui-plugin-dock-real-headless`。
- [ ] `mise run test-ui-plugin-dock-real`（interactive —
  `real-dcomp-pixel` の既知 fail で exit 1 は設計どおり）。
- [ ] `mise run test-ui-scenario-contract`。
- [ ] `mise run check-ui-driver-boundary`。
- [ ] `mise exec -- prek run --all-files`。
- [ ] `review-protocol` 1 cycle・blocker 0。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent.

## Implementation Handoff

- Recommended mode: inline execution（Worker worktree 内で
  単一ストリーム実行）。
- Independent work streams: none。
- User approval required before dispatch: no（review-protocol の
  blocker 0 を承認として扱う）。
