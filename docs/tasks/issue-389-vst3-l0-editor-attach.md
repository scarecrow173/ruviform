---
id: "TASK-ll0hVC7GhO5tm1LEDBWZIe"
type: "task"
status: "completed"
title: "Issue #389 VST3/L0 実外部 Editor attach タスク（adapter open_gui + PoC vst3 lane）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
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
    - "docs/plans/implement-issue-389-vst3-l0-editor-attach.md"
    - "docs/test-specs/issue-389-vst3-l0-editor-attach.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
    - "docs/tasks/issue-388-389-native-dock-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-389-vst3-l0-editor-attach.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-vst3-l0-editor-attach.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  issues: [389]
---

# Issue #389 VST3/L0 実外部 Editor attach タスク

正本: `docs/plans/implement-issue-389-vst3-l0-editor-attach.md`。

## チェックリスト

- [x] T-1 `crates/ruviform-plugin-vst3/src/host.rs` —
  `BridgePlugFrame` + `Vst3Plugin` `view`/`plug_frame`/`gui_seq`
  フィールド + `close_view` + `Drop` 先頭 teardown +
  `open_gui`/`close_gui` override（D1–D4・D6）。
  追加実機発見: distributed plugin 向け `IConnectionPoint`
  connect/disconnect + host `createInstance` の
  `IMessage`/`IAttributeList` 提供が `createView` 前提だった。
- [x] T-2 adapter unit test 追加/更新 — VA-01..VA-06（dormant
  →`PluginFailed`、init 済 fixture→`Unsupported`、close 未 open
  →`Unsupported`、既存 descent 非回帰）。
- [x] T-3 `ruviform-poc-native-dock` `vst3` feature — `poc.rs` 内に
  `RVF_POC_VST3_PATH`/`RVF_POC_VST3_PLUGIN_ID` 駆動の実 module mode
  + `EnumChildWindows` による editor HWND 観測（probe mode は既定の
  まま維持）。
- [x] T-4 `mise.toml` manual lane `poc-vst3-editor` + 実機 run →
  `evidence/rf389-vst3-editor/`（`poc-vst3-vinyl-win11.txt` +
  `report.md`）。
- [x] T-5 `mise run fmt-check`/`check`/`lint`/`test` + feature lane
  green + review-protocol + IR + PR。

## 完了条件

- test-spec の headless 全 case green、manual lane の結果が
  PASS/Fail/NotRun/Unsupported の個別分類で evidence 化される。
- `open_gui`/`close_gui` が実外部 module の `IPlugView` を
  生成・`attached(HWND)`/`removed()`・release し、
  `PluginEditorSession`/`ParentSlotLease` 境界を経由する実装。
- 実現不能経路（L1/CLAP/AU/macOS/Linux/onSize/キー転送）は
  honest Unsupported/NotRun のまま。
