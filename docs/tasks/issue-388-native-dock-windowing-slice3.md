---
id: "TASK-rf388DockWindowing01"
type: "task"
status: "in-progress"
title: "Issue #388 slice 3 タスク — clip/focus/redock windowing 実測"
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
    - "docs/plans/implement-issue-388-native-dock-windowing-slice3.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-388-native-dock-windowing-slice3.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-388-native-dock-windowing-slice3.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 388
---

# Issue #388 slice 3 タスク

- [x] T1: render.rs `slot_clip` 修正 + 単体テスト
- [x] T2: poc.rs OS 観測/入力注入ヘルパ + probe message counter
- [x] T3: `poc/dock_windowing.rs` scenario + dispatch + mise task
- [x] T4: probe mode での完走確認
- [x] T5: Vinyl 実機 lane 実行 + `evidence/rf388-native-dock-windowing/`
- [x] T6: capability matrix 更新 + impl 記録
- [x] T7: `mise run fmt-check check lint test test-ui-gpui` +
      既存 `poc-native-dock-layout` lane の非退行確認
