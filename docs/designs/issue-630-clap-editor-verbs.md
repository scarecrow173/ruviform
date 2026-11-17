---
id: "DESIGN-630clapEditorVerbs03"
type: "design"
status: "approved"
title: "Issue #630 CLAP editor verb 実バイナリ coverage slice — 設計"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/discovery/issue-630-clap-editor-verbs.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-630-clap-editor-verbs.md"
  implements:
    - "docs/specs/issue-630-clap-editor-verbs.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-clap-editor-verbs.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 CLAP editor verb 実バイナリ coverage slice — 設計

## 方針

production adapter は `ruviform#754` で着地済みのため変更しない。
本 slice は (a) fixture `test_editor` の resize 受理、(b) 実
cdylib 経由の verb 試験、(c) 外部 validator 証拠の簿記更新の
3 点に限定する。fixture の resize-capable 化は VST3 `test_editor`
（`ruviform#754` で `canResize=kResultTrue` + `onSize` 実 resize）
の対称変更とし、decline 系 semantics（re-embed 拒否・transient
拒否・scale 制約）は維持する。

## 変更点

- `crates/ruviform-plugin-test-clap/src/editor.rs`:
  - `GuiState` に `size: GuiSize`（clack 型・u32）を追加し、
    `Default` を手書き impl で `DEFAULT_GUI_SIZE` 初期化に変更
    （derive Default では 0x0 になるため）。
  - `get_size` は `state.size` を返す。`can_resize` は `true`。
    `set_size` は `created` gate 後に `state.size` を記録し、
    `hwnd` があれば `win32::resize`（新設 `SetWindowPos` +
    `SWP_NOMOVE|SWP_NOZORDER` helper、VST3 `onSize` と同形）で
    child HWND を実 resize する。`create` 前の `set_size` は
    defined rejection（protocol violation）。
  - `adjust_size`/`get_resize_hints` は trait default `None`
    （制約宣言なし）を維持。`set_scale`/`set_transient`/
    re-embed 拒否は不変。
  - module doc・unit test（`size_and_transient_verbs_are_honest`）
    を resize-capable 契約に追従。
- `crates/ruviform-plugin-test-clap/tests/host_path.rs`:
  - 非 ignore 層: `editor_gui_verbs_decline_without_open_gui`
    （editor の 4 verb → `Err(Unsupported)`）と
    `non_editor_classes_decline_gui_verbs`（index 0-3 の 4 verb →
    `Err(Unsupported)`）を追加。`GuiSize`（plugin-core f64 型）を
    import に追加。
  - Windows `#[ignore]` 層:
    `editor_gui_verbs_over_real_child_hwnd` — hidden top-level
    parent ×2 を `CreateWindowExW` で作成、`open_gui` →
    `GetWindow(GW_CHILD)`/`GetParent`/`IsWindow` で child 観測、
    `set_gui_size(800x600)` → `GetWindowRect` で 800x600 観測、
    `set_gui_visibility` → `GetWindowLongW(GWL_STYLE)&WS_VISIBLE`
    bit 観測（hidden parent のため `IsWindowVisible` は祖先連鎖で
    不成立 — child 自身の style bit が ShowWindow の直接 oracle）、
    `set_gui_parent`（第二 parent）→ `Err(PluginFailed)`、
    `close_gui` → `IsWindow(child)==false`、再 `close_gui` →
    `Err(Unsupported)`、audio pass-through 非回帰、parent 後始末。
  - `windows-sys` は crate の通常 dependency（`cfg(windows)`）で
    integration test から利用可能。新規依存なし。
- `crates/ruviform-plugin-test-clap/README.md`: size verbs の記述を
  resize-capable 契約に追従。
- `evidence/rf630/abi-validator/`: `mise run
  test-plugin-abi-validator`（pin 済み `CLAP_VALIDATOR`/
  `VST3_VALIDATOR`）再実走で run.json・raw txt を更新。
- `evidence/rf630/clap-editor-verbs-verification.md`（新規）+
  doc chain 後半（plan/task/test-spec/adr/impl ir/exp/jsonl）。

## 失敗時の倒れ方

- `set_size` が `created` 前に呼ばれた場合は defined rejection —
  host bridge は `open_gui` 成功後にしか verb を呼ばないため
  production 経路では発生しない。
- `set_gui_size` に NaN/負値が来ても host bridge が f64→u32 に
  truncate/saturate するため fixture 側は u32 のみ受ける。0 や
  巨大値は `SetWindowPos` が OS 側で処理し fixture は best-effort
  で resize を試みる（記録値は要求値のまま — VST3 `onSize` と
  同じ「要求を記録・適用は best-effort」方針）。
- validator が fixture/adapter 欠陥を指摘した場合は本 slice で
  修正せず証拠つき報告（`ruviform#722`/`ruviform#732` 先例）。
- ignored 実機試験が environment_blocked なら `NotRun` を記録し、
  headless/semantic 証拠を実機 Pass に昇格しない。
