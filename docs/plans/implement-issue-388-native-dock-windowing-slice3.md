---
id: "PLAN-rf388DockWindowing01"
type: "plan"
status: "approved"
title: "Issue #388 slice 3 実装計画 — clip 修正 + windowing PoC + 証拠"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-388-native-dock-windowing-slice3.md"
    - "docs/designs/issue-388-native-dock-windowing-slice3.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-388-native-dock-windowing-slice3.md"
    - "docs/test-specs/issue-388-native-dock-windowing-slice3.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-388-native-dock-windowing-slice3.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 388
---

# Issue #388 slice 3 実装計画

## タスク分解

1. **render.rs clip 修正**: `slot_clip` 純粋関数 +
   `project_slot_frame` の差し替え + 単体テスト
   （内側/部分はみ出し/完全外側/中間 mask）。
2. **poc.rs ヘルパ追加**: `region_box`（GetWindowRgn+GetRgnBox）、
   `plugin_tree`（editor+descendants）、`WH_GETMESSAGE` hook
   （install/drain/uninstall）、`send_key`/`click_at`/
   `set_focus_hwnd`、probe wnd_proc の message counter。
3. **dock_windowing.rs 新設**: attach → clip_overhang →
   clip_direct → focus_keys → redock → teardown の stage 機械。
4. **配線**: `poc.rs` の scenario dispatch、`main.rs` doc、
   `mise.toml` に `poc-native-dock-windowing` task。
5. **検証**: `mise run fmt-check check lint test
   test-ui-gpui` + probe mode での scenario 完走 +
   Vinyl 実機 lane。
6. **証拠・文書**: `evidence/rf388-native-dock-windowing/`、
   matrix 更新、impl 記録。

## リスク

- WH_GETMESSAGE hook がキーを観測できない場合:
  focus が plugin tree にあること + probe counter で補完し、
  hook 観測不能は `NotRun` として正直に記録。
- 実クリックが plugin focus を移さない場合: `SetFocus` での
  focus 移行を fallback として記録し、click 経路は別 check。
- `update_view` の spec 差し替えが ViewHost update で拒否
  される場合: 原因を特定して修正（slot binding の残存を
  前提に同 key で再 mount）。
