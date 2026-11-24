# Issue #630 editor verb — export 側 L1 dispatch 配線（wire tag 29/30）検証記録

`docs/designs/issue-630-editor-verbs.md` の slice-4 実装記録が「export 側
dispatch 配線は後続 slice/別タスク」と宣言した残件 —— wire tag 29/30
（`SetGuiFocusTarget`/`SetGuiPopup`）の `ruviform-plugin-export` 側 L1
dispatch 配線を headless に実装・検証した。本 slice の着地点は session
ops → host → wire → child runner の GUI marshal → plugin hook の往復が
session 層から `Ok` で返ることであり、実 GUI 上の OS 効果（実 HWND の
`SetFocus`・実 popup surface）の検証は slice 6（実 GUI 受入）の責務。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows (win32/x64)、Git Bash MINGW64 |
| worktree | `G:/orca/workspaces/ruviform/rf630-verbs-wiring` |
| ブランチ | `scarecrow173/rf630-verbs-wiring`（`origin/main` `15cc9e7f` 起点） |
| Rust | mise 管理 toolchain（`mise run check|lint|test|fmt` 経由） |
| 実施日 | 2026-11-25 |

## 配線の形

- `Runner::dispatch`（`crates/ruviform-plugin-export/src/lib.rs`）の GUI
  family arm に `WireCommand::SetGuiFocusTarget { .. }` /
  `WireCommand::SetGuiPopup { .. }` を追加し `dispatch_gui` へ流す。定義済み
  順序（handle check → `Docked` 拒否 → no-surface `InvalidValue` →
  marshal）をそのまま通るため `dispatch_gui` 本体と
  `#[cfg(not(windows))]` stub は無変更 —— 非 Windows では handle 解決後の
  defined `Unsupported` が自動適用される。
- `run_verb`（`crates/ruviform-plugin-export/src/gui.rs`）に 2 arm を
  `SetGuiFocus`/`SendGuiKey` と同形で追加: `!surfaces.contains_key(&
  instance)` → `Err(InvalidValue)` の surface 存在検査の後、`crate::guarded`
  経由で `ErasedPlugin::{set_gui_focus_target,set_gui_popup}` を呼ぶ。
  `SetGuiPopup.open` は strict `{0,1}` `u8` なので `open != 0` で bool 化。
  `GuiRequest::Verb` が command を verbatim で運ぶため marshal 経路は
  無変更、`SendPluginPtr` の SAFETY 契約と unsafe inventory も不変。
- l1-runner の `GuiPlugin` mock（`src/bin/ruviform-plugin-l1-runner.rs`）に
  2 hook を `set_gui_focus`/`send_gui_key` の既存語彙で実装:
  `view.is_none()` → `Err(Unsupported)`、surface open 中は全 target・全
  kind で `Ok`。mock が pin するのは marshal/reply round-trip と reply
  語彙であり、target 別の意味差ではない。

## `HostWindow` 決裁の結果と根拠

`set_gui_focus_target(GuiFocusTarget::HostWindow)` の export 側意味論は
設計 doc 未記載だったため Coordinator に決裁を仰ぎ、**verbatim
forward**（Spec の推奨案 (b)）に確定した（msg_d4e98b8e406a への回答
msg_3dca3b86f6bc）。runner は `HostWindow` を含む全 target を
`plugin.set_gui_focus_target` へそのまま渡し、runner 側で特別扱いしない。

根拠:

- container HWND は `open_gui` 時の runner-minted `ParentLease` payload
  として既に plugin 側へ渡っている。L0 adapter（slice 3/4）が
  `HostWindow` を adapter 側で消費する確定設計（`SetFocus(container)` +
  plugin への focus 喪失通知）は L1 経路でもそのまま成立する。
- `EditorElement` の「delivery は focus intent であり OS-focus claim では
  ない」契約と同じ意味論に family 全体を揃えられる。runner 側で実
  `SetFocus` を足す案（L0 adapter 先例の (a)）は `windows-sys` の
  `Win32_UI_Input_KeyboardAndMouse` feature 追加（lease 外の
  `Cargo.toml` 変更）か bare `#[link]` extern が必要で、headless L1
  配線という着地点に対して不釣り合いな新規 OS 依存だった。
- 実 OS focus 効果の検証は slice 6（実 GUI 受入、Coordinator
  computer-use 駆動）の責務として分離済み。

## pin 反転の記録

`crates/ruviform-plugin-host/tests/editor_session_l1.rs` の wire-v8 verb
pin は「mock runner に fixture seam がなく entry dispatch catch-all が
defined `Unsupported` を返す」を意図的に固定していた。配線後の着地点
（session ops → 実 child → mock hook の実 `Ok`/round-trip）へ反転する
ことは設計 doc 宣言済みで、Task Spec をもって Coordinator 承認済みとして
実施した。

- 反転後の pin: `GuiFocusTarget::{Editor, EditorElement{element:7},
  HostWindow}` と `GuiPopupKind::{ChildWindow} open=true`/`{Dialog}
  open=false` が surface-open 中に `Ok` を返す。
- post-close の `InvalidValue` ループに `set_focus_target`/`set_popup`
  を追加し、no-surface 語彙が family 全体で一貫することを維持。
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` の 3 箇所
  （surface-open `Ok` round-trip・no-surface `InvalidValue` リスト・
  非 Windows `Unsupported` リスト）に tag 29/30 行を追加。surface-open
  区間は verb 追加による `too_many_lines` 超過を受け、既存の
  command-loop 形（rejection 系テストと同型）へ集約した。

## synthetic-verb-delivery 語彙の確認

verb は合成コマンド配送であり、OS イベント注入（`SendInput`/
`keybd_event`/`PostMessage`）・OS 入力キュー検証ではない ——
`send_gui_key` の shortcut seam 意味論は不変。本 slice の pin が主張
するのは wire round-trip と reply 語彙のみで、`real-os-*` 語彙は証拠・
doc・test コメントのいずれにも付けていない。

## 実行した検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt` | 0 | Pass（`cargo fmt --all`、差分は本変更分のみ） |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-plugin-export` | 0 | Pass |
| `mise run lint ruviform-plugin-export` | 0 | Pass（初走で `e2e_gui_floating_open_and_verb_round_trip` が `clippy::too_many_lines`（121/100）— verb assert を command-loop へ集約して解消・再 pass） |
| `mise run test ruviform-plugin-export` | 0 | 全 pass — `l1_runner_e2e` 26/26（`e2e_gui_floating_open_and_verb_round_trip`・`e2e_gui_defined_rejections_are_window_independent` の tag 29/30 行を含む）、`l1_video_runner_e2e` 20/20、unit 11/11、doc 2/2 |
| `mise run check ruviform-plugin-host` | 0 | Pass |
| `mise run lint ruviform-plugin-host` | 0 | Pass（補助 — test file を編集する crate の lint） |
| `mise run test ruviform-plugin-host` | 0 | 全 pass — `editor_session_l1` 5/5（`windows::l1_editor_ops_drives_the_floating_session_over_the_wire` の pin 反転区間を含む）ほか全 suite |
| `mise exec -- prek run --all-files` | 0 | Pass |

実行順の注意（Spec どおり）: `editor_session_l1.rs` は
`ruviform-plugin-l1-runner` bin を test binary 近傍から探索するため、
`mise run test ruviform-plugin-export`（bin がビルドされる）を先に
実行した。

## 判断・残件

- ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA に
  対する必須 CI 結果であり、Coordinator が確認する。
- 本 slice の受入範囲外: 実 GUI 受入（slice 6 — Coordinator
  computer-use 駆動の別工程）、`#[ignore]` real-HWND 行、dock-test
  lane verdict、AC7 の consumer oracle 実走 legs（external owner）。
  いずれも NotRun —— headless L1 配線が本 slice の着地点。
- `GuiFocusTarget::HostWindow` の実 OS focus 効果は verbatim forward
  決裁により plugin/adapter 側の責務であり、headless 層では
  round-trip の成立のみを検証した。
