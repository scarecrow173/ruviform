---
id: "IMPL-7fK2mQx9wRt4vBn8sLp3"
type: "impl"
status: "completed"
title: "Issue #389 rf631.plugin-dock-real session lifecycle 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-389-real-session-lifecycle.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/designs/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/plans/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/tasks/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/test-specs/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/discovery/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/impl/ir/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/impl/exp/issue-389-real-session-lifecycle.md"
      - type: "file"
        path: "docs/impl/exp/issue-389-real-session-lifecycle-exp.jsonl"
      - type: "dir"
        path: "evidence/rf389"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/bin/plugin-dock-console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "docs/designs/issue-389-plugin-editor-session.md"
      - type: "file"
        path: "evidence/rf631/plugin-dock-console.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-headless.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-scenario.jsonl"
      - type: "file"
        path: "evidence/rf631/plugin-dock-scenario.jsonl"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-389-real-session-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-389-real-session-lifecycle.md"
    - "docs/plans/issue-389-real-session-lifecycle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-389-session-lifecycle.md"
    - "docs/impl/ir/issue-631-real-row.md"
  refines: []
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
    adopted:
      - "docs/impl/exp/issue-389-real-session-lifecycle.md"
    rejected: []
---

# Issue #389 rf631.plugin-dock-real session lifecycle 実装記録

## Summary

`rf631.plugin-dock-real` lane の real catalog を 15 → 21 row へ拡張し、
Issue #389 session lifecycle の real-window 受入 6 row を追加した。
`real-session-degraded`/`real-session-recovery`/`real-session-timeout`
は実 lease chain 上の pass-oracle、`real-editor-visibility` は定義済み
`Unsupported` refusal の観測 row、`real-session-crash`/`real-session-hang`
は in-process lane の architectural boundary を正直に記録する executable
`Unsupported` row。interactive Windows 実走で新規 6 row 全て設計どおりの
verdict（pass ×3・unsupported ×3）。

## Context

`docs/tasks/issue-389-real-session-lifecycle.md`（← plan/design/spec/
test-spec/discovery）に従う Issue `scarecrow173/ruviform#389` tracker の
real-window slice。#631/#690 系で確立した NativeSlot → container HWND →
`ParentSlotLease` → `InProcessEditorOps` redeem の実 chain と、catalog
（driver-neutral `real_catalog.rs`）と executable entry
（`real_dock.rs` `real_entries`）の 1:1 同期規律を維持する。
plugin-host/plugin-vst3/plugin-clap・`mise.toml`・`Cargo.toml` は無変更。
Issue #389 は OPEN のまま。

## What was built

- `real_catalog.rs`: `REAL_CATALOG` 末尾へ 6 行追加（degraded/
  recovery/timeout/visibility は `required: &[NativePluginDock]`、
  crash/hang は `required: &[]`）+ module doc 追従。
- `real_dock.rs`:
  - `real_entries()` を `[ScenarioEntry<RealDockContext>; 21]` 化、
    `blocked_reports`/test の固定長・件数を追従。
  - `attempt_real_attach` に case 固有の session drive seam を導入
    — `AttachDrive { drive_open, observe_open }` bundle で open 駆動と
    post-open 観測を case が差し替え可能に。`drive_open_docked` が
    従来の `session.open(Docked)` step row を逐語再現し既存 case の
    evidence shape を維持。
  - `real_editor_ops` — `RealOsGuiTest::redeem_native_lease` 経由の
    実 ledger redeem のみを行う `InProcessEditorOps` factory を
    `open_session_docked` から抽出・再利用。
  - `ChildLifecycle` bundle + `editor_child_body` で child-lifecycle
    系 case（attach/terminate/degraded/recovery/timeout）の entry を
    統一。
  - `conclude_editor_session`/`check_session_converged` を期待 event
    列パラメータ化（既定は従来の `[Attached{Docked}, Closed]`）。
  - `drive_detach_timeout` — ZERO `SessionDeadlinePolicy` +
    `begin_detach` + `request_deadline` + `tick_all` の deadline 駆動。
  - `observe_session_degraded_lifecycle`/`observe_session_recovery_
    lifecycle`/`observe_session_timeout_lifecycle`/
    `observe_editor_visibility_lifecycle` — child HWND contract →
    case 固有 session op → close → child teardown → event 収束の
    pass-oracle。
  - `session_crash_body`/`session_hang_body` — fault を実行せず
    `Unsupported` を返す boundary row。
  - `pixel_observe`/`foreground_observe`/`present_observe` — dcomp
    3 case の artifact 束縛 `impl FnOnce` wrapper（closure 推論対策）。
- `tests/console.rs`: 23 → 29 行・期待 case 一覧追従。
- `plugin-dock-console.rs`/`lib.rs`/`README.md`: 15 → 21・verdict 表
  6 行追加など記述追従。
- `docs/designs/issue-389-plugin-editor-session.md`: 改訂 3（S4/S5）
  として本 slice の capability/DoD reconcile を追記。
- `evidence/rf389/`: 本 slice の lane 出力 JSONL と
  `verification.md` を更新。

## Verification

`evidence/rf389/verification.md` に全 lane 結果を記録。要約:
fmt-check / check / lint / test（default・real-dock・lab-gui）/
plugin-host session tests / test-ui-plugin-dock /
test-ui-plugin-dock-real-headless（21 行 fail-closed）/
test-ui-plugin-dock-console / check-ui-driver-boundary / prek —
全て pass。`test-ui-plugin-dock-real` は interactive 実走で exit 1 —
既知 `real-dcomp-pixel` fail（#532 決裁待ち・本 slice 無関係）のみ。
新規 6 row: degraded/recovery/timeout = pass、visibility/crash/hang
= unsupported（設計どおり）。

## Notes / boundaries

- crash/hang は `required: &[]` で executable row として catalog に
  残し、body が architectural boundary を説明して `Unsupported` を
  返す — `NotRun` 偽装・行欠落なし。
- `real-dcomp-pixel` の fail は GPUI DComp topmost occlusion の既知
  問題（#532）— baseline 系 3 実走で同一 `green_pixels=0` を確認済み。
- GPUI/Win32 型は public contract・evidence schema に非露出
  （`check-ui-driver-boundary` pass で確認）。
