---
id: "SPEC-630clapEditorVerbs02"
type: "spec"
status: "approved"
title: "Issue #630 CLAP editor verb 実バイナリ coverage slice — spec"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-clap-editor-verbs.md"
    - "docs/specs/issue-630-abi-validator-rerun.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-630-clap-editor-verbs.md"
  implements:
    - "docs/discovery/issue-630-clap-editor-verbs.md"
  implemented-by:
    - "docs/designs/issue-630-clap-editor-verbs.md"
  depends-on:
    - "docs/specs/issue-630-abi-validator-rerun.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 CLAP editor verb 実バイナリ coverage slice — spec

## 目的

`ruviform#754` で production chain に着地した CLAP editor verb
（`open_gui`/`close_gui`/`set_gui_size`/`set_gui_visibility`/
`set_gui_parent`）を、実 CLAP cdylib を `ClapLibrary` 経由で load
する `tests/host_path.rs` の試験で検証し、併せて `ruviform#754`
で stale 化した外部 ABI validator 証拠（artifact SHA-256・
repo.head）を opt-in lane 再実走で更新する。

## 要求

1. `test_editor` fixture を resize-capable に拡張する
   （discovery 決裁 — `ruviform#754` の VST3 `canResize`/`onSize`
   対称）。`can_resize=true`、`set_size` が要求 size を記録して
   `get_size` に反映し、attach 済み child HWND を実 resize する。
   `adjust_size`/`get_resize_hints` は制約なしの trait default
   （`None`）を維持し、`set_scale`/`set_transient` の既存
   semantics と re-embed 拒否は変更しない。
2. `tests/host_path.rs` に非 ignore 層（全 OS）を追加する:
   - `test_editor` で GUI 未 open の `set_gui_size`/
     `set_gui_visibility`/`set_gui_parent`/`close_gui` が各々
     defined `Err(Unsupported)` に収束すること。
   - 非 editor class（`test_gain`/`test_fault`/`test_stateful`/
     `test_instrument`）の同 4 verb が trait/adapter default の
     `Err(Unsupported)` に収束すること。
3. `tests/host_path.rs` に Windows `#[cfg(windows)]` + `#[ignore]`
   の実機層を追加する: hidden top-level parent を
   `CreateWindowExW` で作り `ParentLease::new(0, parent)` で
   `open_gui` 成功 → `GetWindow(parent, GW_CHILD)` で実 child HWND
   観測 → `set_gui_size` で実 resize（`GetWindowRect` oracle）→
   `set_gui_visibility(false/true)` で child の WS_VISIBLE bit
   観測（hidden parent のため `IsWindowVisible` は祖先連鎖で
   使えず `GetWindowLongW(GWL_STYLE)` を oracle とする）→
   `set_gui_parent`（第二 parent）が fixture の
   `gui already embedded` で `Err(PluginFailed)` → `close_gui`
   で child 破棄（`IsWindow==false`）→ 再 `close_gui` は
   `Err(Unsupported)`。fake・semantic 証拠を実機 Pass に昇格
   しない。実走不可なら `NotRun`+`observed.environment_blocked`
   を正直に記録する。
4. `mise run test-plugin-abi-validator` を provisioned validator
   （clap-validator pin `47e2424`・VST3 SDK validator — run.json
   記録の SHA-256 と一致確認済み）で再実走し、
   `evidence/rf630/abi-validator/` の run.json（artifact
   SHA-256・repo.head）と raw 出力を更新する。verdict 変化なし
   見込み — digest/HEAD 簿記更新が主。binary 不在・実走不可なら
   fail-closed `NotRun` を正直に記録する。
5. production adapter（`ruviform-plugin-clap`/`vst3`/`host`/`core`）、
   `crates/ruviform-plugin-test-vst3/`、`mise.toml`、
   `Cargo.toml`/`Cargo.lock`、`.github/**`、docs index、
   `evidence/` の他 Issue 分は変更しない。VST3 側の同型試験は
   追加しない（`rf631` real lane が `ruviform#754` で証拠化済み）。

## 受入条件

- [ ] 非 ignore 試験が全 OS で `Err(Unsupported)` の defined
      refusal を検証し、`mise run test ruviform-plugin-test-clap`
      が exit 0。
- [ ] Windows ignored 試験が interactive window station で実走し、
      open → child HWND 観測 → 実 resize → hide/show → reparent
      拒否 → close → double-close の一連を実測値で記録する。
      実走不可の場合は `NotRun`+`environment_blocked` を記録。
- [ ] `mise run check`/`lint`/`test`（`ruviform-plugin-test-clap`・
      `ruviform-plugin-clap`）が exit 0。
- [ ] validator lane 再実走で run.json の artifact SHA-256・
      repo.head が更新され、verdict は前回と同等（Passed）。
      不在なら `NotRun` を記録。
- [ ] doc chain（slug `issue-630-clap-editor-verbs`）と検証 md を
      作成し、docs index は触らない。
