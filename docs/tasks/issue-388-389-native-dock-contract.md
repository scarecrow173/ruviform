---
id: "TASK-ibEQJ0Q4JzJ0UtqiKM5W5u"
type: "task"
status: "completed"
title: "Issue #388/#389 Native Dock 契約タスク（boundary / native_slot / editor_session）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/impl/ir/issue-388-389-native-dock-contract.md"
  depends-on:
    - "docs/plans/implement-issue-388-389-native-dock-contract.md"
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-388-389-native-dock-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [388, 389]
---

# Issue #388/#389 Native Dock 契約タスク

正本: `docs/plans/implement-issue-388-389-native-dock-contract.md`。

## チェックリスト

- [x] T-1 `ruviform-plugin-core/src/boundary.rs` に
  `PluginEditorSessionId`・`ParentSlotLease` 追加 + lib re-export・
  README 更新。
- [x] T-2 `ruviform-ui/src/native_slot.rs` 新設 + `lib.rs` re-export・
  README 更新。
- [x] T-3 `ruviform-plugin-host/src/editor_session.rs` 新設 + lib
  re-export・README 更新（`InProcessEditorOps`/`L1EditorOps` 含む）。
- [x] T-4 `crates/ruviform-ui/tests/native_slot.rs`・
  `crates/ruviform-plugin-host/tests/editor_session.rs`（test-spec
  全ケース — 11 + 17 tests green）。
- [x] T-5 `mise run fmt-check`/`check`/`lint`/`test` green・
  `Cargo.lock` 差分なし（依存追加なし）・証跡は
  `docs/impl/ir/issue-388-389-native-dock-contract.md`。

## 完了条件

test-spec 全ケース green + workspace lint/fmt/test green。実機
PoC 行は Blocked/NotRun 記録のまま（本 dispatch 範囲外）。
