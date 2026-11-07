---
id: "TSPEC-rf388DockLayoutPoc01"
type: "test-spec"
status: "approved"
title: "Issue #388 GPUI Panel/Tab レイアウト内 Editor 埋め込みの実機テスト仕様（Windows / VST3 / L0）"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/lib.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-388-native-dock-layout-poc-implementation.md"
    - "evidence/rf388-native-dock-layout/report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-388-native-surface-slot.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-388-native-surface-slot.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  defers: []
  deferred-by: []
metadata:
  issues: [388]
---

# Issue #388 GPUI Panel/Tab レイアウト内 Editor 埋め込みの実機テスト仕様

## 目的

GPUI の Panel/Tab レイアウトに投影された `NativeSlot` の container へ、実 VST3 editor を attach し、
tab 切替・resize・focus・close が OS 状態に反映されることを実機で確認する。
本仕様の Pass は **Dock PASS ではない**。Dock PASS は完了条件（clip・入力・re-dock・plugin resize 等）を
含めて判定する（`evidence/rf388-native-dock-layout/report.md`）。

## 実行入口

`RVF_POC_SCENARIO=dock`（mise: `mise run poc-native-dock-layout`）。`RVF_POC_VST3_PATH` で実 module、
`RVF_POC_EVIDENCE_DIR` でウィンドウキャプチャ。各観測は `check=<id> result=pass|fail` で出力し、
失敗数が 1 以上なら exit 1。slot 仕様が取得できない場合は `projection` が fail になり、無判定の成功終了にはならない。

## ケース

| TestID | check id | 期待 | 判定の根拠 |
| --- | --- | --- | --- |
| DL-01 | `bind_native_slot` | layout key `dock.editor` を declare 済み slot へ bind | `Ok(())` |
| DL-02 | projection | renderer が laid-out 矩形と `Visible` を canonical slot へ投影（非空矩形） | `projected_rect` と `slot_visibility` |
| DL-03 | `attach_parented` | editor HWND の親が slot container HWND | `GetParent` 比較 |
| DL-04 | `attach_visible` | attach 直後の editor が可視 | `IsWindowVisible` |
| DL-05 | `update_view` / `tab_hide` | 非アクティブ tab へ切替後、slot は `Hidden` かつ editor は非可視 | slot 状態と `IsWindowVisible` |
| DL-06 | `tab_show` | アクティブ tab へ戻すと slot `Visible` かつ editor 可視 | 同上 |
| DL-07 | `focus_container` | `request_focus` 後、フォーカス HWND が container | `GetFocus` 比較 |
| DL-08 | `resize_grow_follows` | window 拡大で slot 矩形が追従し、container の物理サイズが slot の論理サイズ × scale と一致 | slot 矩形の変化と container の `GetWindowRect` |
| DL-09 | `resize_shrink_follows` | window 縮小で slot 矩形が追従し、container が同じ規則で追従 | 同上 |
| DL-10 | `close_session` | close 後 session が `Closed`（`EditorSessionState::Closed`）かつ editor HWND が破棄 | `session.state()` と `IsWindow` |
| DL-11 | `unbind_native_slot` / `release_lease` / `release_slot` | 解放後 container 破棄 | `IsWindow` |
| DL-12 | `slot_event_order` | slot event に `LeaseRevoked` があり、それが `Destroyed` より前 | `drain_events` の位置比較 |

## 補助観測（判定ではなく証拠）

- DL-05 / DL-06 のキャプチャ領域平均 RGB（`pixel-sample.ps1`）: hidden で GPUI 背景、visible で Vinyl 色。
- キャプチャ時の `foreground_is_window=true` を確認すること（false の画像は証拠として採用しない）。

## 対象外（NotRun / Unsupported。判定に含めない）

- clip（ancestor overhang）、keyboard / IME / popup、pointer による tab 操作、re-dock / float / move、
  DPI / 複数モニタ、2 window、crash / hang 隔離、plugin 側 resize（host `onSize` 未配線）。
- macOS / Linux / CLAP / AU / L1 editor 埋め込み。
