# Issue #630 editor verb — slice 1+2（境界語彙 + wire / host command + session ops）検証記録

`docs/designs/issue-630-editor-verbs.md` の「実装 slice 分割案」に対し、slice 1
（境界語彙 + wire、`crates/ruviform-plugin-core/`）と slice 2（host command +
session ops、`crates/ruviform-plugin-host/`）を連続実装した。本 slice は
「transport 面」の完結を目的とし、新 verb 語彙が session ops → `HostCommand` →
wire → child まで defined refusal 語彙で往復できることを headless に検証した。
plugin 側の実 seam（fixture-private extension / private IID）は slice 3/4 の
占有範囲であり、本 slice 完了時点の正しい着地点は defined `Unsupported`。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows (win32/x64)、Git Bash MINGW64 |
| worktree | `G:/orca/workspaces/ruviform/rf630-verbs-wire-host` |
| ブランチ | `scarecrow173/rf630-verbs-wire-host`（`origin/main` `49ecc37f` 起点、検証時点で `origin/main` と一致） |
| Rust | mise 管理 toolchain（`mise run check|lint|test|build` 経由） |
| 実施日 | 2026-11-24 |

## 変更内容

### slice 1: 境界語彙 + wire（`crates/ruviform-plugin-core/`）

- `boundary.rs`: `GuiFocusTarget`（`Editor`/`EditorElement{element:u32}`/
  `HostWindow`）と `GuiPopupKind`（`ChildWindow`/`Dialog`）を追加。既存公開
  enum ポリシーどおり `#[non_exhaustive]` + `Copy` 値型（`Copy, Clone, Debug,
  PartialEq, Eq, Hash`）。`GuiFocusTarget`/`GuiPopupKind` は `lib.rs` で re-export。
- `plugin.rs`: `Plugin::set_gui_focus_target`/`set_gui_popup` を default
  `Err(HostError::Unsupported)` で追加（`set_gui_focus(bool)` は F2 決裁どおり
  残置し doc での関連のみ明記、deprecated にしない）。`ErasedPlugin` には
  default 本体付き mirror を追加し、`Erased<P>` impl から concrete hook へ
  forward —— 外部実装者を壊さない additive 形。
- `wire.rs`: `Command::SetGuiFocusTarget{target}`（tag **29**）・
  `Command::SetGuiPopup{kind,open:u8}`（tag **30**）を追加し、
  `WIRE_VERSION` を 7 → **8**。履歴 block・pin test・module doc・
  `Hello.capabilities` doc を追随。codec arm は既存 helper の流儀:
  - `GuiFocusTarget`: `u8` discriminant（`Editor`=0・`EditorElement`=1+`u32`
    element・`HostWindow`=2）。`element` は opaque `u32` で全値 verbatim。
    `encode_gui_focus_target`/`decode_gui_focus_target` を field codec として
    追加（`encode_gui_key_event` 先例）。
  - `GuiPopupKind`: `u8` discriminant（`ChildWindow`=0・`Dialog`=1、
    `decode_gui_popup_kind`）+ `open` は `decode_u8_flag` の strict `{0,1}`。
  - 未割当 discriminant/flag byte は `WireError::Malformed`（
    `GuiKeyDirection`/`OpenGuiMode` の typed-field rejection と同一規則）。
  - encode 側は crate 内 exhaustive match（将来 variant は compile error で
    `WIRE_VERSION` bump を強制）。
- capability 面: `WIRE_CAPABILITY_GUI_FLOATING`（bit 0）は引き続き唯一の
  割当 bit —— tag 29/30 は同 family に atomically に属し新 bit は割らない
  （「one bit covers the family atomically」先例 + 設計 doc open question の
  保守的解消）。doc に反映済み。

### slice 2: host command + session ops（`crates/ruviform-plugin-host/`）

- `command.rs`/`lib.rs`: `HostCommand::SetGuiFocusTarget{handle,target}`・
  `SetGuiPopup{handle,kind,open}` を追加（「L1-reachable since wire v8 — same
  negotiated gate as `SetGuiSize`」doc 付き）。境界型を re-export。
- `composite.rs`: `command_handle`/`retarget`/`to_wire` に両 variant を追加
  （`open` は `u8::from(open)` —— `visible`/`focused` と同一）。`l0_only_gui_verb`
  は変えず `SetGuiParent` だけが L0-only のまま。
- `host.rs`: `execute` に 2 arm、`set_gui_focus_target`/`set_gui_popup` private
  dispatch を `set_gui_focus` 型（Plain slot 直接呼出・`Shared` slot は
  `shared_verb` 経由）で実装。
- `editor_session.rs`: `EditorSessionOps` に `set_focus_target`/`set_popup` を
  default `Err(EditorOpError::Unsupported)` 本体付きで追加
  （`drain_gui_requests` 先例の additive 形）。`PluginEditorSession` に同名 verb
  を `set_focus`/`send_key` と同じ non-fatal 契約で追加
  （`Attached`|`Hidden` 以外は `InvalidState`、ops 拒否は `Ops` 伝播・state
  遷移なし）。`InProcessEditorOps`/`L1EditorOps` は `HostCommand` forward +
  `map_host_error` 写像。`GuiFocusTarget::HostWindow` の実現は adapter/runner
  側責務で、ops 層は target を verbatim で運ぶ。
- `l1/mod.rs`: `gui_family` に tag 29/30 を追加 —— `GUI_FLOATING` bit の
  negotiated gate が新 verb にも atomically に効く。
- capability claim 規則: `EditorCapabilities` の `popup`/`input`/`focus` field
  は既存どおり。本 slice で新たな claim はしない —— 「claim するのは受理の
  実績 or open で証明できた mechanism fact のみ」の規範を struct doc に明記。

### 設計 doc 追記（`docs/designs/issue-630-editor-verbs.md`）

「実装記録（slice 1+2）」節を末尾に追加 —— 確定 byte layout・
`WIRE_VERSION` 8・capability bit 不割当の解消・child 側着地点を記録。
`status: draft` は変更せず、docs index（`docs/designs/README.md`）は
Coordinator が統合時に整備するため未更新のまま。

### 変更しなかった範囲（確認事項）

`crates/ruviform-plugin-export` の `Runner::dispatch` は GUI family 以外の
command を `entry.dispatch` へ流し、catch-all が `Err(HostError::Unsupported)`
を返す。新 tag 29/30 はこの catch-all で defined refusal に落ちる
（slice 3/4 が fixture seam を載せるまでの**正しい**着地点）ことを
`editor_session_l1.rs` の実 child E2E で実測済み —— export crate への変更は
不要（かつ lease 外のため不可）。

## defined refusal 語彙の踏襲

設計 doc「失敗時の倒れ方」どおり、新しい失敗語彙・state 遷移は新設していない:

- trait 未実装 → `Unsupported`（`Plugin`/`ErasedPlugin`/`EditorSessionOps`
  default）。
- wire decode 不能 → `Malformed`（未割当 discriminant・非 `{0,1}` flag）/
  `Truncated`（payload 途中切れ）。
- unadvertised child → capability gate の `Unsupported`（`gui_family` 拡張で
  tag 29/30 も同一 gate）。
- plugin/ops 拒否 → `map_host_error` 経由 `EditorOpError::Unsupported` →
  session では `Ops(...)` の non-fatal 伝播（state 遷移なし）。

## 実行した検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt` → `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-plugin-core` | 0 | Pass |
| `mise run check ruviform-plugin-host` | 0 | Pass |
| `mise run lint ruviform-plugin-core` | 0 | Pass |
| `mise run lint ruviform-plugin-host` | 0 | Pass |
| `mise run test ruviform-plugin-core` | 0 | Pass（unit 144 + 14 + 8 + doctest 35 + compile-fail 2、failed 0） |
| `mise run build ruviform-plugin-export` | 0 | Pass（L1 E2E fixture `ruviform-plugin-l1-runner` 生成 + lease 外 crate の v8 整合 compile 確認） |
| `mise run test ruviform-plugin-host` | 0 | Pass（全 suite、failed 0 —— `editor_session_l1` の実 child E2E 5 件を含む） |
| `mise exec -- prek run --all-files` | 0 | Pass（全 hook） |

注: `mise run test ruviform-plugin-host` は対象 package の test target のみを
build するため、`editor_session_l1` が spawn する
`ruviform-plugin-l1-runner`（`ruviform-plugin-export` 所有 bin）は事前に
`mise run build ruviform-plugin-export` で生成した。初回実行は runner 不在で
当該 suite が panic し、build 後の再実行で全件 Pass —— workspace 全体 build
（`mise run test` 無引数 or `test-full`）であれば member bin も生成される
test fixture の既知の前提。

## 追加した test 面

- `wire.rs`: `wire_version_is_v8` pin、scope test を tag 29/30 に拡張、
  `command_roundtrip_covers_the_v8_editor_verbs`（全 variant/flag 組合せ）、
  `gui_focus_target_codec`（field roundtrip + 未割当 byte `Malformed` +
  途中切れ `Truncated`）、`gui_popup_kind_and_open_flag_vocabulary`・
  `set_gui_focus_target_frame_vocabulary`（frame 単位の malformed 語彙）。
- `composite.rs`（unit）: `l0_only_guard_keeps_only_set_gui_parent` に新
  command の非 L0-only 確認、`to_wire_maps_the_gui_verb_family` に v8 写像。
- `l1/mod.rs`（unit）: gate 両 arm（unadvertised→`Unsupported` /
  advertised→`Ipc` = wire 到達証拠）に tag 29/30。
- `tests/editor_session.rs`: `GuiPlugin` fixture に v8 verb 記録を追加、
  `l0_editor_verbs_reach_the_plugin_through_the_v8_commands`（session →
  `InProcessEditorOps` → `HostCommand` → plugin の end-to-end chain +
  未 attach `InvalidState`）、`l0_plugin_verb_refusal_maps_to_unsupported_
  non_fatally` へ v8 verb の non-fatal 拒否を追加、
  `l1_editor_ops_refuses_every_op_on_an_unadvertised_child` に gate 拒否を
  追加。
- `tests/editor_session_l1.rs`: 実 runner child への v8 verb 到達を検証 —
  未 advertise（非 Windows gate test）では `Unsupported`、advertised
  （Windows）では gate 通過後に child entry dispatch catch-all の defined
  `Unsupported`（fixture seam は slice 3/4 の占有範囲）を実測。

## 判断・残件

- `GuiFocusTarget::HostWindow` の「ops 層が container へ focus を戻す」実現は
  adapter/runner 側の責務（slice 3/4 + export 側 GUI dispatch）。本 slice の
  ops は target を verbatim で運ぶのみ。
- `set_gui_focus(bool)` は F2 決裁どおり残置（`GuiFocusTarget::Editor` 相当の
  短手）。deprecated 化はしない。
- docs index（`docs/**/README.md`）は Worker 契約どおり未編集 —— Coordinator
  が統合時に整備する。
- Issue #630 本体 AC（Native Editor 実走）は slice 3〜6 で完結する。本 slice
  の検証は transport 面の headless 検証に留まる。
