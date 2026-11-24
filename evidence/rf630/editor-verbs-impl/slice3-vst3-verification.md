# Issue #630 editor verb — slice 3（VST3 adapter + fixture seam）検証記録

`docs/designs/issue-630-editor-verbs.md` の「実装 slice 分割案」に対し、slice 3
として `set_gui_focus_target`/`set_gui_popup` の VST3 adapter 実現と、実 verb
到達を可能にする fixture-private COM seam（`IRvfEditorOps`）を実装した。
slice 1+2 が完結させた transport 面（boundary 語彙 + wire-v8 + session ops）に
続き、本 slice は「plugin 側に verb を届ける」面を VST3 で完結させた。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows (win32/x64)、Git Bash MINGW64 |
| worktree | `G:/orca/workspaces/ruviform/rf630-verbs-vst3` |
| ブランチ | `scarecrow173/rf630-verbs-vst3`（PR #947 merge commit 起点） |
| Rust | mise 管理 toolchain（`mise run check|lint|test` 経由） |
| 実施日 | 2026-11-24 |

## 変更内容

### seam 契約（duplicated-by-contract）

- `crates/ruviform-plugin-vst3/src/editor_ops.rs`（adapter 側・新規）と
  `crates/ruviform-plugin-test-vst3/src/editor_ops.rs`（fixture 側・新規）に
  `IRvfEditorOps` を定義 —— fixture cdylib は adapter crate に依存できない
  （`Cargo.toml` の dep 境界）ため、IID byte string・vtbl layout・verb 意味論を
  両 file に同一複写し、module doc で byte-identical 維持を義務化。
- IID = `"RVFEDITOROPS0001"`（ruviform-namespaced 16-byte TUID —— SDK 割当
  IID と構造的に衝突しない）。vtbl = `FUnknownVtbl` prefix +
  `apply_focus(element: u32, focused: TBool)` +
  `popup(kind: u32, open: TBool)`。
- `vst3::support` は private module のため、`Unknown` の 3 method
  （`query_interface`/`add_ref`/`release`）は `FUnknownVtbl` prefix への自前
  delegate で実装 —— generated bindings と同じ `#[repr(C)]` + `Interface` +
  `Inherits<FUnknown>` + blanket `SmartPtr` dispatch pattern。
- adapter 側は `Construct`/`make_vtbl` を `#[cfg(test)]` のみ（adapter は seam
  を実装せず query/call のみ）、fixture 側は常時コンパイル。

### adapter（`crates/ruviform-plugin-vst3/src/host.rs`）

- `set_gui_focus_target` —— `Editor` → `onFocus(1)`（`set_gui_focus(true)`
  と同一行先）、`EditorElement{n}` → `view.cast::<IRvfEditorOps>()` →
  `apply_focus(n, 1)`、seam 不在 → defined `Unsupported`、seam 非
  `kResultOk` → `PluginFailed`、`#[non_exhaustive]` 未知 target →
  `InvalidValue`、view 未 open → `Unsupported`。
- `GuiFocusTarget::HostWindow` —— attach 時に保存した `gui_parent` の
  container HWND へ `SetFocus` + `GetFocus` oracle 検証 + `onFocus(0)` 通知。
  adapter crate は `windows-sys` edge を持たないため、`user32` の
  `SetFocus`/`GetFocus` を bare `#[link]` extern（`mod win32`、
  `cfg(windows)`）で宣言 —— `SetFocus` 戻り値は成功時も null を返し得るため
  `GetFocus()==container` を oracle とする。off-Windows・lease 未保存は
  `Unsupported`、oracle 失敗・`onFocus` 拒否は `PluginFailed`。
- `set_gui_popup` —— `view` 未 open → `Unsupported`、seam 不在 →
  `Unsupported`、kind code（`ChildWindow`=0・`Dialog`=1、wire discriminant
  と同一値）を `popup(kind, open)` へ、seam 非 `kResultOk` →
  `PluginFailed`、未知 kind → `InvalidValue`。

### fixture（`crates/ruviform-plugin-test-vst3/`）

- `TestEditorView::Interfaces = (IPlugView, IRvfEditorOps)` —— view の
  `queryInterface` が private IID に seam vtbl を返す。
- `IRvfEditorOpsTrait` 実装: `apply_focus` → `fe:{element}:{focused}` token、
  `popup` → kind dispatch（`Dialog` → `set_dialog_open`、`ChildWindow` →
  `set_child_open`、未知 kind → `kInvalidArgument`）。
- `popup_toggle`（`VK_P` 共有）を explicit-state worker `set_dialog_open` に
  分解し、`set_child_open` を追加 —— 両者とも冪等（現状態と同一要求は
  `kResultOk`・token 不記録）で、`po:`/`pc:`/`co:`/`cc:` token を editor child
  HWND title の event log（`rvf-test-editor-attach-{n}|{ev},{ev},…`）へ記録。
- Windows `win32` module に `child_window`（editor child 配下の
  `WS_CHILD|WS_VISIBLE`、`rvf-test-editor-child-{seq}` title）と `focus`
  （`SetFocus`）を追加 —— `onFocus(1)` は実 `SetFocus(child)` まで上げ、
  `removed` は child/popup 両 surface を teardown。`windows-sys` の
  feature set に `Win32_UI_Input_KeyboardAndMouse` を追加
  （`SetFocus`/`GetFocus` は `WindowsAndMessaging` ではなくこの namespace）。
- fixture `Cargo.toml` 変更は既存 dep の feature 追加のみ —— dep 木不変。

## defined refusal 語彙の踏襲

- view 未 open → `Unsupported`（既存 `set_gui_focus`/`send_gui_key` と同一
  gate）。
- seam 不在（`kNoInterface`）→ `Unsupported` —— capability absence の正直な
  refusal。
- seam verb 拒否（`kResultFalse`/`kInvalidArgument` など非 `kResultOk`）→
  `PluginFailed` —— plugin の refusal を偽装成功にしない。
- `#[non_exhaustive]` 未知 variant → `InvalidValue` —— 未割当語彙の定義済み
  rejection。

## 実行した検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt` | 0 | Pass |
| `mise run fmt-check` | 0 | Pass（`cargo fmt --all -- --check`） |
| `mise run check-l1-real-plugin` | 0 | Pass（4 crate × host/l1-runner feature × all-targets） |
| `mise run lint-l1-real-plugin` | 0 | Pass（`clippy -D warnings`） |
| `mise run test ruviform-plugin-vst3` | 0 | Pass（canonical・default feature、lib 44 件、failed 0） |
| `mise run test ruviform-plugin-vst3 --features host --lib` | 0 | Pass（canonical + host feature、98 件、failed 0 —— seam 写像 test 群を含む） |
| `mise run test ruviform-plugin-test-vst3` | 0 | Pass（lib 121 + host_path 26、failed 0） |
| `cargo test -p ruviform-plugin-test-vst3 -- --ignored` | 0 | Pass（real-HWND lane 3 件全て実走 Pass —— `#[ignore]`d lane は mise test 対象外のため直接起動の補助証拠） |
| `mise run test-l1-real-plugin` | 0 | Pass（cdylib + runner bin build、`--lib --bins` unit、`#[ignore]`d L1 E2E 全 lane） |

## delivery 語彙（synthetic-verb-delivery の名指し）

設計 doc「『実 OS イベントではない』ことの正直な表現」の語彙規約に従い、本
evidence の全 test 面は `vst3-editor-verbs:synthetic-verb-delivery` とする。
verb の配送経路は host API → adapter verb dispatch → fixture-private COM seam
（`apply_focus`/`popup`/`onFocus`）であって、OS 入力キュー・OS focus routing・
window message 経路は一切検証していない。real-HWND lane の
`SetFocus`/`GetFocus`/`EnumChildWindows`/`EnumWindows` は verb 到達後の
fixture 状態遷移を観測する state-machine oracle であり、OS 実入力経路の証拠
ではない（synthetic verb delivery; no OS input queue or focus routing
exercised）。`kd:`/`ku:` token を持つ `send_gui_key` 系は本 slice の範囲外で
未検証。

## 追加した test 面

- `host.rs`（adapter unit）: `ComWrapper` 偽装 view 3 種 —
  `SeamView`（seam 有・decline 切替）/`PlainView`（seam 無）。verb 写像
  （`Editor`→`onFocus`・`EditorElement`→`apply_focus`・両 popup kind →
  code 写像）、seam 不在 → `Unsupported`、seam 拒否 → `PluginFailed`、
  view 未 open → `Unsupported`、`HostWindow` の lease 未保存 →
  `Unsupported` / dead lease → `PluginFailed`（Windows arm）。
- `editor_view.rs`（fixture unit）: seam QI dispatch、`fe:`/`po:`/`pc:`/
  `co:`/`cc:` token、冪等 no-token、未知 kind `kInvalidArgument`、
  `removed` の両 surface teardown。
- Windows real-HWND lane（`#[ignore]`、本機 session で実走済み）:
  - `editor_view.rs::seam_verbs_drive_real_windows` —— 実 attach 後に
    `popup(ChildWindow)` の実 `WS_CHILD` を `EnumChildWindows` +
    title token で発見、`onFocus(1)` → `GetFocus()==child`、
    `popup(Dialog)` の `WS_POPUP` style 検証、title log の全 token。
  - `tests/host_path.rs::editor_verbs_drive_real_windows_through_bridge` ——
    production bridge（`Vst3Library::load` → `open_gui`）で全 verb を
    end-to-end に駆動し、`GetFocus`（Editor→child・HostWindow→container）、
    `EnumChildWindows`/`EnumWindows` の surface 発見、title token
    （`f:1`/`fe:7:1`/`co:1`/`po:1`/`f:0`）を検証。
- `tests/host_path.rs`（headless）: `editor_verbs_without_open_gui_are_
  unsupported`（test_editor + test_gain 両 class）、`editor_verbs_after_
  declined_open_stay_unsupported`（declined `open_gui` 後の rollback
  確認）。

## 判断・残件

- `GuiFocusTarget::HostWindow` は Windows のみ実現 —— off-Windows は
  defined `Unsupported`（`EDITOR_PLATFORM_TYPE` と同じ正直な refusal
  policy）。
- `apply_focus` の `focused` flag は現行常に `1` を配送 —— release 側は
  seam signature を変えずに後続 slice で載せられる形。
- CLAP 側の対称実現（slice 4）は別 lease —— 本 slice の seam 設計は
  `clap.ext.gui`-style extension query の VST3 mirror として意図的に対称化
  されている。
- docs index（`docs/**/README.md`）は Worker 契約どおり未編集。
