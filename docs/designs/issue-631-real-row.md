---
id: "DESIGN-034YqGS0FcTkFNpOd08X6E"
type: "design"
status: "approved"
title: "Issue #631 real-row 拡充 設計"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/specs/issue-631-real-row.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-631-real-row.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-631-real-row.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-631-real-row.md"
    - "docs/specs/issue-631-real-window-dock.md"
  blocks:
    - "docs/plans/issue-631-real-row.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-631-real-row.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-real-row.md"
  derived-by:
    - "docs/plans/issue-631-real-row.md"
    - "docs/test-specs/issue-631-real-row.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-631-real-row.md"
  references:
    - "evidence/rf631/verification.md"
  defers: []
  deferred-by: []
---

# Issue #631 real-row 拡充 設計

## 概要

`rf631.plugin-dock-real` catalog を 5 → 12 行へ拡張する。AC1 の
残り能力は「実際に駆動できる行」と「公開 API が無いため
`Unsupported` と正直に判定する行」に分け、AC3 は非画素 oracle
（session event・HWND 契約・session 状態）のみで判定する editor
op case を 2 本追加する。headless fail-closed は session を
開始しない `--blocked-probe` で決定的に実証する。

## catalog 行と判定写像

| case | required | session | 判定 |
|---|---|---|---|
| `real-multiwindow-focus` | `MultiWindow` | 必要 | 2 window の `request_focus` → `window_focused` settle 観測。release は driver 応答を記録のみ |
| `real-os-input` | `RealOsPointerOrKeyboard` | 必要 | preflight effective → focus 済み window へ pointer move/click + `Tab` が `Injected`。`EnvironmentBlocked`→`NotRun`、`Unsupported`→`Unsupported` |
| `real-window-dpi` | `SemanticAction` | 必要 | body が `Unsupported`（scale-factor/DPI API 非公開） |
| `real-window-ime` | `Ime` | 不要 | preflight `Unsupported`（capability 未 claim） |
| `real-window-close` | `SemanticAction` | 必要 | body が `Unsupported`（close API 非公開） |
| `real-editor-resize` | `NativePluginDock` | 必要 | `set_size` の定義済み拒否 → `Unsupported`（受理時は oracle 未整備で `NotRun`） |
| `real-editor-redock` | `NativePluginDock` | 必要 | `reattach` の定義済み拒否 → session `Failed` → close 収束を観測 → `Unsupported` |

## 実装方針

- **2 window mount**: `run_all_with_pixel_artifacts` が
  `RealWindowMount` を 2 件渡す（view instance 631/632、同一
  `plugin_dock_view_spec`、window B は `position` 指定で window A
  と非重複）。`RealDockContext` に `window_b: Option<WindowId>` を
  追加し、`gui.windows()` の順序（`WindowId` 昇順 = mount 順）で
  確定する。既存 5 case は `ctx.window`（先頭）のみを使い不変。
- **`attempt_real_attach` 拡張**: `observe_open` クロージャに
  `lease_raw: u64` を追加（`ParentSlotLease::new(lease.raw())`
  再構成用）。既存 2 call site（attach / dcomp-pixel）は無視して
  渡すだけ。
- **非画素 editor op case**: `observe_open` 内で
  `oschild::observe` + `check_attached_children`（attach と同一の
  子 HWND 契約）→ `rejecting_editor_ops`（本番
  `InProcessEditorOps`、これら verb は redeemer 不参照）で op
  試行 → session 状態確認 → close/収束。resize は
  `conclude_editor_session` を再利用（event 系列不変
  `[Attached{Docked}, Closed]`）。redock は拒否時の event 系列
  `[Attached{Docked}, Detached, Failed{..}, Closed]` を検証する
  専用収束 check を追加する。op の着地点は `OpLanding`
  （`Refused`/`Accepted`/`Other`）で表し、bool の増殖を避ける。
- **`--blocked-probe`**: `plugin-dock-real-scenario` に排他モード
  追加。`blocked_reports` と同一の report を生成し、全行が
  `NotRun` + `observed.environment_blocked` +
  `environment_kind != "real-window"` であることを bin が検査。
  違反は exit 1。出力 lane 名は `plugin-dock-real-headless`。
  probe の detail は「session start 未試行」を明示し、観測した
  headless run を装わない。
- **catalog 同期**: `REAL_CATALOG`（headless 可視 metadata）と
  `real_entries()`（実行可能 entry）の行を同順で追加し、既存の
  pairwise test が drift を検出する。blocked path は
  `real_entries` を読むため自動的に全行をカバーする。

## 境界

- GPUI 型・OS handle は `real_dock.rs` 内に留め、
  `real_catalog.rs`/public API/evidence schema に漏らさない
  （`check-ui-driver-boundary` green を維持）。
- `run_scenario` の capability preflight が `Ime` 行を
  `Unsupported` に落とすため body は防御的 `not_run` のみ。
- 新規 case は全て非 RT・test-only。`unsafe` は既存の
  `#[cfg(windows)]` query-only pattern（`oschild`）の再利用のみで
  新規追加しない。

## リスク

- 2 window mount で既存 case の証拠が変わる可能性 → 既存 case は
  `ctx.window` のみ参照、window B は focus/os-input case のみが
  使う。実機で JSONL を確認する。
- `request_focus(_, false)` が platform により `Ok` を返す場合が
  ある → release は verdict にせず観測記録のみ（Pass/Fail 材料に
  しない）。
- OS input が物理 desktop を実際に動かす → preflight effective
  確認後のみ注入し、`EnvironmentBlocked` は `NotRun` に落とす
  （#552 規約）。
