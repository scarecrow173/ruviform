---
id: "ADR-PELNtFsK8EusLCqC7qI0qf"
type: "adr"
status: "accepted"
title: "Issue #630 CLAP test_editor を resize-capable fixture として拡張"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-clap-editor-verbs.md"
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/designs/issue-630-clap-editor-verbs.md"
  changes:
    added:
      - type: "file"
        path: "docs/adr/issue-630-clap-editor-verbs.md"
    modified: []
    deleted: []
  implements:
    - "docs/specs/issue-630-clap-editor-verbs.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/designs/issue-630-clap-editor-verbs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-630-clap-editor-verbs.md"
---

# Issue #630 CLAP test_editor を resize-capable fixture として拡張

## Context and Problem Statement

`ruviform#754` で plugin-core `Plugin` trait に
`set_gui_size`/`set_gui_visibility`/`set_gui_parent` が追加され、
CLAP adapter `host.rs` に verb 実装が着地した。CLAP `test_editor`
fixture は従来 fixed-size（`can_resize=false`、`set_size` は defined
rejection）であり、この状態では `set_gui_size` の success 経路
（can_resize gate 通過 → `set_size` → 実 HWND resize）を実
バイナリで検証できない。

同じ `ruviform#754` で VST3 `test_editor` は `canResize=kResultTrue`
+ `onSize` 実 resize に変更され、verb chain の end-to-end coverage
を可能にした。CLAP fixture を fixed-size decline のままにするか、
VST3 対称の resize-capable に拡張するかの決裁が必要だった。

## Decision Drivers

- `set_gui_size` の success 経路を実 CLAP cdylib 経由で検証できる
  こと。decline 経路だけでは can_resize gate 後の `set_size` 呼出し
  と実 resize は coverage されない。
- VST3/CLAP fixture の対称性 — 両形式で同じ verb semantics を
  fixture が提供すること。
- fixture の変更面を `src/editor.rs` + `tests/host_path.rs` +
  `README.md` に限定し、production adapter・他 fixture・依存を
  変更しないこと。
- 既存の正直な decline semantics（re-embed 拒否・transient 拒否・
  scale 制約）を維持すること。

## Considered Options

- fixed-size decline 維持 — `set_gui_size -> Err(Unsupported)` の
  refusal は観測できるが、`set_size` success 経路は fixture が
  受理しないため原理的に coverage 不能。VST3 との非対称が残る。
- resize-capable 化 — `can_resize=true` + `set_size` が child HWND
  を実 resize。`ruviform#754` の VST3 変更と同じ判断で、verb の
  success 経路まで実バイナリで証明できる。

## Decision Outcome

CLAP `test_editor` を resize-capable に拡張する。`can_resize` は
`true`、`set_size` は `created` gate 後に要求 `GuiSize` を
`GuiState.size` に記録し `get_size` がそれを返す。attach 済み
child HWND があれば `SetWindowPos`（`SWP_NOMOVE|SWP_NOZORDER`）
で実 resize する。`create` 前の `set_size` は defined rejection。
`adjust_size`/`get_resize_hints` は制約宣言なしの trait default
（`None`）を維持し、`set_scale`/`set_transient`/re-embed 拒否の
既存 semantics は変更しない。

この決定は fixture の振る舞い変更であり、production adapter
（`ruviform-plugin-clap`）の verb 実装や CLAP ABI 解釈は変更
しない。CLAP `clap_plugin_gui` の `can_resize`/`set_size` は
embedded window に対して host が resize を要求できることを
示す標準的な capability であり、fixture がそれを正直に実装する
のは ABI 適合の改善である。

## Consequences

- Good: `set_gui_size` の success 経路（can_resize gate → `set_size`
  → 実 HWND resize）が実バイナリで検証可能になり、VST3/CLAP の
  fixture 対称性が回復する。ignored 実機試験で `GetWindowRect`
  round-trip の実 resize oracle を観測できる。
- Bad: fixture の size state が可変になり、`get_size` が session
  状態に依存する（`GuiState` の `Default` を手書き化する必要が
  あった）。`set_size` の `created` gate は protocol violation
  を defined rejection で表面化させる追加の防御。

## Implementation Plan

- Affected paths: `crates/ruviform-plugin-test-clap/src/editor.rs`、
  `crates/ruviform-plugin-test-clap/tests/host_path.rs`、
  `crates/ruviform-plugin-test-clap/README.md`、
  `evidence/rf630/`、本 ADR を含む Issue #630 doc chain。
- Constraints: production adapter・他 fixture・`Cargo.toml`/
  `Cargo.lock`・`mise.toml`・`.github/**`・docs index は変更
  しない。`adjust_size`/`get_resize_hints` の `None` default と
  re-embed/transient/scale の既存 semantics は維持する。
- Patterns to follow: VST3 `editor_view.rs` の `canResize`/`onSize`
  resize 実装（`ruviform#754`）を CLAP `clap_plugin_gui` の
  `can_resize`/`set_size` に写像。host_path の既存
  `prepared_editor_plugin()`/`drive_block` helper と
  `editor.rs` の `set_parent_creates_real_child_hwnd` hidden
  parent 先例を踏襲。
- Verification: `mise run check/lint/test ruviform-plugin-test-clap`、
  `cargo test -p ruviform-plugin-test-clap --test host_path --
  --ignored`（interactive Windows）、`mise run
  test-plugin-abi-validator`（pin 済み validator）。

## More Information

- Issue: <https://github.com/scarecrow173/ruviform/issues/630>
- Precedent: `ruviform#754`（VST3 fixture resize-capable 化・
  editor verbs production chain）
- Spec/design: `docs/specs/issue-630-clap-editor-verbs.md`、
  `docs/designs/issue-630-clap-editor-verbs.md`
- Discovery: `docs/discovery/issue-630-clap-editor-verbs.md`
