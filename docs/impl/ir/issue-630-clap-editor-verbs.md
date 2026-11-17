---
id: "IMPL-630clapEditorVerbs07"
type: "impl"
status: "completed"
title: "Issue #630 CLAP editor verb 実バイナリ coverage slice — 実装記録"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-630-clap-editor-verbs.md"
    - "docs/plans/issue-630-clap-editor-verbs.md"
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/designs/issue-630-clap-editor-verbs.md"
  changes:
    added:
      - type: "file"
        path: "evidence/rf630/clap-editor-verbs-verification.md"
      - type: "file"
        path: "docs/discovery/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/specs/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/designs/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/plans/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/tasks/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/test-specs/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/adr/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/impl/ir/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-editor-verbs-exp.jsonl"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/src/editor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/tests/host_path.rs"
      - type: "file"
        path: "crates/ruviform-plugin-test-clap/README.md"
      - type: "file"
        path: "evidence/rf630/abi-validator/run.json"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_gain.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_fault.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_stateful.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_instrument.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_editor.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_spawn_fail.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/clap-test_fault-unfiltered.txt"
      - type: "file"
        path: "evidence/rf630/abi-validator/vst3-validator.txt"
  implements:
    - "docs/tasks/issue-630-clap-editor-verbs.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-630-clap-editor-verbs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-630-clap-editor-verbs.md"
---

# Issue #630 CLAP editor verb 実バイナリ coverage slice — 実装記録

## Summary

`ruviform#754` で production chain に着地した CLAP editor verb を
実 CLAP cdylib 経由で検証する coverage を追加し、`test_editor`
fixture を VST3 対称の resize-capable に拡張した。外部 ABI
validator lane を pin 済み binary で再実走し、stale だった
evidence の digest/repo.head を更新した（verdict Passed 不変）。

## Context

CLAP verb 実装（`host.rs` の can_resize gate → `set_size`、
hide/show、set_parent）は plugin-host scripted fake と VST3 real
lane でしか検証されておらず、実 CLAP バイナリ経由の coverage が
ゼロだった。本 slice がそのギャップを埋める唯一の実バイナリ検証。

## Implementation

### Changes

- `src/editor.rs`: `GuiState.size` 追加・`Default` 手書き化、
  `can_resize` → `true`、`set_size` 実装（created gate・size 記録
  ・`win32::resize` で child HWND を `SetWindowPos` resize）、
  `get_size` が `state.size` を返す、`destroy` で size を default
  に戻す。Windows `win32` module に `resize` helper 追加
  （`SWP_NOMOVE|SWP_NOZORDER`）、non-Windows shim に対応 stub。
  module doc・`size_and_transient_verbs_are_honest` unit test を
  resize-capable 契約に追従。
- `tests/host_path.rs`: `GuiSize` import 追加。非 ignore 層
  `editor_gui_verbs_decline_without_open_gui`（editor 4 verb →
  `Err(Unsupported)`）と `non_editor_classes_decline_gui_verbs`
  （index 0-3 の 4 verb → `Err(Unsupported)`）、Windows
  `#[ignore]` `editor_gui_verbs_over_real_child_hwnd`（hidden
  parent ×2 → open_gui → GW_CHILD/GetParent/IsWindow 観測 →
  GetWindowRect 800x600 resize oracle → GWL_STYLE&WS_VISIBLE
  hide/show oracle → set_gui_parent PluginFailed → close_gui 破棄
  → double-close Unsupported → audio 継続）を追加。
- `README.md`: size verbs 記述を resize-capable 契約に追従。
- `evidence/rf630/abi-validator/`: `mise run
  test-plugin-abi-validator`（pin 済み `CLAP_VALIDATOR`/
  `VST3_VALIDATOR`）再実走で run.json・raw txt 9 ファイル更新。
- `evidence/rf630/clap-editor-verbs-verification.md`: 本 slice の
  検証記録（新規）。

### Decisions During Implementation

- **resize-capable 決裁**: discovery で VST3 `ruviform#754` 対称の
  resize-capable を採用（ADR 記録）。decline 維持では `set_size`
  success 経路を原理的に coverage 不能。
- **visibility oracle**: hidden parent のため `IsWindowVisible`
  は祖先連鎖で不成立 — `GetWindowLongW(GWL_STYLE) & WS_VISIBLE`
  を child 自身の visibility oracle として採用。
- **size reset on destroy**: `destroy` で `state.size` を
  `DEFAULT_GUI_SIZE` に戻す — VST3 の per-`createView` fresh
  state と同じ semantics。
- **`set_size` before `create`**: defined rejection（protocol
  violation）— host bridge は open_gui 成功後にしか呼ばない。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-630-clap-editor-verbs.md` /
  `issue-630-clap-editor-verbs-exp.jsonl`

### Rejected

- fixed-size decline 維持（ADR の Considered Options 参照 —
  `set_size` success 経路が coverage 不能）。
- `IsWindowVisible` oracle（hidden parent で祖先連鎖不成立）。

## Validation

`evidence/rf630/clap-editor-verbs-verification.md` に全記録。
要約: fmt-check/check/test/build 全 exit 0、ignored 実機層
実走 Pass（GetWindowRect 800x600・WS_VISIBLE bit・IsWindow
収束を実測）、validator lane verdict Passed（7/7 canonical
legs）、`lint ruviform-plugin-test-clap` のみ scope 外既存破損
（`ruviform-plugin-clap` host feature の clippy 4 件、stash
検証で本 slice 起因でないことを確認）で Fail 記録。

## Risks

- `lint ruviform-plugin-test-clap` の既存破損は main に残る —
  `ruviform-plugin-clap` host feature の `f64 as u32` cast に
  `cast_possible_truncation`/`cast_sign_loss`。別 Issue 起票
  判断は Coordinator。
- fixture の `set_size` は u32 への host 側 truncate を前提 —
  負値/NaN は host bridge が saturate するため fixture は
  u32 のみ受ける。

## Follow-ups

- docs index 再生成は Coordinator が統合時に実施。
- 実 dock/message pump/画素 lane は `ruviform#631`、DComp 解消
  判定は `ruviform#532`、consumer oracle は各 consumer Issue。
