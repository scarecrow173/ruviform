---
id: "TSPEC-630clapEditorVerbs06"
type: "test-spec"
status: "approved"
title: "Issue #630 CLAP editor verb 実バイナリ coverage slice — test specification"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/designs/issue-630-clap-editor-verbs.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-clap-editor-verbs.md"
  implements:
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/designs/issue-630-clap-editor-verbs.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-clap-editor-verbs.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 CLAP editor verb 実バイナリ coverage slice — test specification

## Rule: GUI 未 open の session verb は defined Unsupported に収束する

`test_editor` を含む全 plugin class で、`open_gui` 未成功の状態の
`set_gui_size`/`set_gui_visibility`/`set_gui_parent`/`close_gui`
は `Err(HostError::Unsupported)` を返す（adapter の `self.gui`
gate と trait default の両経路）。この refusal は全 OS で
`#[ignore]` なしに検証される。

- Example: `prepared_editor_plugin()` 直後に 4 verb を呼ぶと全て
  `Err(Unsupported)`、その後も audio pass-through が継続する。
- Example: index 0-3（gain/fault/stateful/instrument）でも同じ
  4 verb が `Err(Unsupported)`。

## Rule: 実 child HWND 経由の verb lifecycle を実機で観測する

Windows interactive window station で `#[ignore]` 試験を実走し、
hidden top-level parent に対する `open_gui` 成功 → 実 child HWND
生成 → `set_gui_size` による実 resize（`GetWindowRect` が要求
size と一致）→ `set_gui_visibility` の hide/show（child の
`GWL_STYLE & WS_VISIBLE` bit が反転）→ `set_gui_parent` の
re-embed 拒否（`Err(PluginFailed)`）→ `close_gui` で child 破棄
（`IsWindow == false`）→ 再 `close_gui` が `Err(Unsupported)` を
一連で観測する。fake・semantic 証拠を実機 Pass に昇格しない。

- Example: `GetWindow(parent, GW_CHILD)` が非 null、
  `GetParent(child) == parent`、`GetWindowRect` が 800x600。
- Example: `close_gui` 後 `IsWindow(child) == 0`、再度の
  `close_gui` は `Err(Unsupported)`。

## Rule: fixture の resize は要求を記録し child HWND に適用する

`test_editor` の `set_size` は `created` 後に要求 `GuiSize` を
`GuiState.size` に記録し `get_size` がそれを返す。attach 済み
なら `SetWindowPos` で child HWND を resize する。`create` 前の
`set_size` は defined rejection。`can_resize` は `true`。
`adjust_size`/`get_resize_hints` は `None`（制約宣言なし）。

- Example: unit test で `create` → `set_size(800x600)` →
  `get_size() == Some(800x600)`。
- Example: `create` 前の `set_size` は `Err`。

## Rule: 外部 validator 証拠は再実走で簿記を更新する

`ruviform#754` で test-vst3 fixture の ABI surface が変化した
ため、provisioned validator（SHA-256 が run.json 記録と一致）で
`mise run test-plugin-abi-validator` を再実走し、run.json の
artifact SHA-256・repo.head・raw 出力を更新する。verdict は
前回と同等（canonical Passed・informational fault leg
Unsupported/exit 101）を期待し、変化があれば証拠つきで報告。
validator binary 不在なら fail-closed `NotRun` を記録する。

- Example: run.json の `artifacts.clapDll.sha256` が再 build 後の
  cdylib digest と一致し `repo.head` が検証対象 commit を指す。
