# Issue #630 editor verb — slice 4（CLAP adapter + fixture）検証記録

`docs/designs/issue-630-editor-verbs.md` の「実装 slice 分割案」に対し、
slice 4（CLAP adapter + fixture）を実装した。本 slice の着地点は「CLAP
adapter が private extension seam（`ruviform.editor-ops`）経由で verb を
plugin に配送し、fixture が event-log token と実 HWND で観測可能になる」
こと —— dock-test 行の verdict flip（`real-clap-editor-*`）と実 GUI 受入は
slice 5/6 の責務であり、本 slice の受入は headless/semantic 層まで。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows (win32/x64)、Git Bash MINGW64 |
| worktree | `G:/orca/workspaces/ruviform/rf630-verbs-clap` |
| ブランチ | `scarecrow173/rf630-verbs-clap` |
| Rust | mise 管理 toolchain（`mise run check|lint|test|fmt` 経由） |
| 実施日 | 2026-11-24 |

## 拡張 id と vtable の確定形

K1 決裁どおり `clap.ext.gui` と同じ mechanism（拡張 id で query した vtable
経由の plugin 内関数呼出）で private seam を引いた。

- 拡張 id: `ruviform.editor-ops`（ruviform-namespaced —— format 規格外の
  test 用 seam であることを命名で正直に示す）
- vtable: `#[repr(C)]` の 3 フィールド固定。両 crate が同じレイアウトを
  個別に定義する（fixture は adapter に依存しないため型を共有できない ——
  `#[repr(C)]` 面そのものが唯一の ABI 契約。拡張 id・フィールド順・各
  discriminant byte は adapter `host.rs` / fixture `editor.rs` 両側の doc
  comment に明記済み）
  - `deliver_key(plugin, direction: u8, key: u32, key_code: u32,
    modifiers: u32) -> bool` —— `direction` は `GuiKeyDirection` の wire
    discriminant（`Down`=0/`Up`=1）、`key` は UTF-16 code unit、`key_code`
    は Win32 virtual-key 値（`i16` bit pattern の zero-extend）、
    `modifiers` は `GuiModifiers` bitmask。いずれも 16bit 値を `u32` slot
    に載せる。
  - `apply_focus(plugin, focused: u8, target: u8, element: u32) -> bool`
    —— `focused` は strict `{0,1}`、`target` は `GuiFocusTarget` の wire
    discriminant（`Editor`=0/`EditorElement`=1/`HostWindow`=2 ——
    `HostWindow` は adapter が消費し plugin には届かない）、`element` は
    `EditorElement` の opaque `u32`（非 element target では 0）。
  - `popup(plugin, kind: u8, open: u8) -> bool` —— `kind` は
    `GuiPopupKind` の wire discriminant（`ChildWindow`=0/`Dialog`=1）、
    `open` は strict `{0,1}`。
  - `false` は defined rejection で、adapter は `HostError::PluginFailed`
    に写像。

## 実装内容

### adapter（`crates/ruviform-plugin-clap/`）

- `host.rs`: `ClapPlugin` に `gui_parent: Option<ParentLease>` を追加
  （`open_gui`/`set_gui_parent` で記録、`close_gui` で消去 —— VST3 adapter
  の `self.gui_parent` 先例）。`editor_ops()` helper が「GUI 未 open →
  `Unsupported`」「拡張無し → `Unsupported`」の gate を担い、4 verb が
  vtable 経由で配送する。
- `send_gui_key`: supplementary-plane `key`（`u16::try_from(u32::from(key))`
  失敗）と未知 `direction` は `InvalidValue`（boundary.rs の BMP 契約・
  VST3 と同じ検査）。`deliver_key` に全 field を配送。
- `set_gui_focus`: `apply_focus(focused, Editor)` に写像（F2 の
  `GuiFocusTarget::Editor` 相当短手）。
- `set_gui_focus_target`: `Editor` → `apply_focus(1, Editor, 0)`、
  `EditorElement{n}` → `apply_focus(1, EditorElement, n)`、`HostWindow` →
  Windows 限定で adapter が `IsWindow` gate 後に `SetFocus(container HWND)`
  を実行 + plugin に `apply_focus(false, Editor)` を通知（`f:0` token が
  残る）。非 Windows は `Unsupported`。
- `set_gui_popup`: `popup(kind, open)` に転送。未知 `kind` は
  `InvalidValue`。
- `Cargo.toml`: `[target.'cfg(windows)'.dependencies] windows-sys = "0.61"`
  を追加（`Win32_Foundation`/`Win32_UI_Input_KeyboardAndMouse`/
  `Win32_UI_WindowsAndMessaging` —— `SetFocus`/`GetFocus`/`IsWindow` 用。
  fixture crate と同 version。`Cargo.lock` は dep edge 1 行のみの差分で
  lockfile 再生成は不要）。

### fixture（`crates/ruviform-plugin-test-clap/`）

- `editor.rs`: `declare_extensions` に `RuviformEditorOps` を register
  （`ExtensionImplementation` + `RawExtensionImplementation::new(&'static
  vtable)` で vtable を供給。3 つの `extern "C"` router は
  `PluginWrapper::handle(...).unwrap_or(false)` で main-thread object へ
  転送 —— `None` は seam の defined rejection `false`）。
- event-log title channel: `GuiState` に `events`/`attach_seq`/
  `verb_popup`/`verb_popup_open`/`verb_popup_seq` を追加。受信 verb を
  VST3 同文法の token（`f:{0|1}`/`fe:{element}:{0|1}`/
  `kd:{key:04x}:{key_code:04x}:{mods:04x}`/`ku:{…}`/`po:{seq}`/`pc:{seq}`）
  として記録し、child HWND の window text に
  `rvf-test-editor-attach-{n}|{ev},{ev},…` の形で刻印（`GetWindowTextW` が
  観測 channel —— VST3 `window_title` grammar verbatim）。
- `apply_focus(Editor, 1)` は実 `SetFocus(child)` まで上げる（
  `GetFocus()==child` の実 OS oracle —— 設計 doc open question の推奨案
  どおり採用。`HostWindow`/`false` への戻しまでを一連として扱う旨を実装
  と doc に反映済み）。`EditorElement` は intent 記録のみ（synthetic
  のみ、OS focus の主張はしない）。
- `popup`: `Dialog` は既存 `win32::popup` で plugin 所有 `WS_POPUP` を
  editor child の root ancestor（`GetAncestor(GA_ROOT)`）を owner に開閉、
  `ChildWindow` は新設 `win32::child` で editor child 配下の `WS_CHILD` を
  開閉。両者とも title は `rvf-test-editor-popup-{seq}`、class
  `RuviformClapTestEditor` を共有。popup 面は単一 slot —— close は kind を
  問わず収束、repeated edge は `false` で拒否。token 記録は
  HWND-independent —— unattached editor や HWND 生成失敗でも
  `po:`/`pc:` は残る（論理面と HWND 面の分離 —— VST3 `popup_open`
  先例）。非 Windows では `create` 自体が `is_api_supported` で拒否
  されるため verb は全て `!created` の `false` に収束し、seam は実質
  Windows-only（実装確認で判明 —— 「非 Windows でも token 記録」の
  設計想定は CLAP 側では成立しない）。
- `Cargo.toml`: `windows-sys` features に `Win32_UI_Input_KeyboardAndMouse`
  を追加（`SetFocus`/`GetFocus` 用 —— fixture 自身の cdylib build に必要。
  `--all-targets` では dev-dep の `ruviform-plugin-clap` 経由の feature
  unification で隠れていたが、`cargo build -p` 単独 build で顕在化し修正
  した）。
- floating-mode surface（`set_transient`/`suggest_title`/`show`）は verb 面
  に載せず `win32::popup` helper の供給元として残置。

## verb 別着地点（L0/L1）

- L0 in-process（session ops → `HostCommand` → adapter → plugin）:
  全 4 verb が seam 経由で fixture に到達し、event-log token + 実 HWND で
  観測可能 —— `#[ignore]` real-HWND test が実走で全 verb を検証済み（後述）。
- L1（out-of-process wire）: tag 26/27（`send_gui_key`/`set_gui_focus`）は
  `ruviform-plugin-export` の `dispatch_gui` → UI-thread marshal → adapter
  impl で fixture へ届く。tag 29/30（`set_gui_focus_target`/`set_gui_popup`）
  は export の `entry.dispatch` catch-all → `Err(Unsupported)` に落ちる —
  export crate は lease 外であり、defined refusal が正しい現状の着地点
  （tag 29/30 の export 側 GUI dispatch 配線は後続 slice/別タスク）。

## defined refusal 語彙の踏襲

設計 doc「失敗時の倒れ方」どおり、新しい失敗語彙・state 遷移は新設してい
ない:

- GUI 未 open → adapter `Unsupported`（`editor_ops` gate）
- 拡張未実装（non-editor classes・外部 plugin）→ adapter `Unsupported`
- plugin/seam 拒否（`false` 戻り）→ `PluginFailed`（fixture 側の defined
  rejection: `created` 前・re-entrant borrow・未割当 byte・strict flag 違反・
  範囲外値・repeated popup edge）
- 表現不能な値 → `InvalidValue`（supplementary-plane `key`・未知
  direction/target/kind）
- 非 Windows の `HostWindow` → `Unsupported`
- `gui` open 済みで `gui_parent` 未記録 → `PluginFailed`（broken
  invariant は capability 回答ではない）

## synthetic-verb-delivery 語彙の適用

実装・doc・test doc・本記録に一貫して適用 —— verb chain が `GuiKeyEvent`
値を plugin vtable へ配送し、fixture が event-log token で受領を記録する。
`SendInput`/`keybd_event`/`PostMessage` 擬似入力は行わず、OS の input
queue・focus routing・message pump を通る配送は検証対象外。証拠語彙は
`clap-editor-input:synthetic-verb-delivery` で、`real-os-*` 語彙は付けない。
唯一の実 OS 観測は `SetFocus`/`GetFocus`/`EnumChildWindows` 系の HWND
oracle（fixture が生成した実 window の実在検査）であり、入力配送の主張
ではない。

## ABI validator 再実走の要否判断 → 実走した（生成物は lease 内の別 dir）

判断: fixture の `get_extension` 応答集合に private id `ruviform.editor-ops`
が増えるが、CLAP の extension mechanism は「host が知っている id で query
する」方式で列挙面を持たず、外部 validator が未知 id を問い合わせることは
ない —— 標準 ABI 面（entry/factory/plugin/gui/state/params vtable）は不変の
はずで、理論上は再実走不要と読める。ただし validator binary が本環境で利用
可能（`%LOCALAPPDATA%/ruviform/plugin-abi-validator/`）だったため、
**実走して実測で確認した**。生成物は lease 外の
`evidence/rf630/abi-validator/` を汚さないよう、lease 内の
`evidence/rf630/editor-verbs-impl/abi-validator/` に `--evidence-dir` で分離し
た。

実走結果（`run.json` = `rf630-abi-validator-run/2`、記録 head
`ee80fff107677ba6ceedd77b92a4bec4eb4016da` —— 実走は未 commit の
working tree の dll に対して行った）:

| leg | 結果 | 補足 |
| --- | --- | --- |
| `clap-validator test_gain` | Failed（27 success / 0 failed / 2 crashed） | `preset-discovery-crawl`/`load` が各 45s timeout crash |
| `clap-validator test_fault` | Failed（同上、2 crashed = preset-discovery） | |
| `clap-validator test_stateful` | Failed（同上、2 crashed = preset-discovery） | |
| `clap-validator test_instrument` | Failed（同上、2 crashed = preset-discovery） | |
| `clap-validator test_editor` | **Unsupported**（exit 101 — JSON report 未到達） | 詳細は下記 |
| `clap-validator test_fault`（unfiltered） | Unsupported（informational —— fault arm の意図的 crash として lane 設計どおり） | |
| `vst3-sdk-validator` | **Passed**（282/282、baseline と不変） | 本 slice の非対象 crate だが lane が一括実走 |

`test_editor` leg の Unsupported 内訳と非回帰の根拠:

- `param-set-events`/`param-set-no-cookies`/`param-fuzz-bounds` が sandboxed
  run 内で失敗（validator の「Failed to deserialize the output from the
  sandboxed operation」panic —— sandbox 子 process の途中死）、続く
  `preset-discovery-*`/`param-fuzz-modulation` が 45s timeout → validator 自体が
  scoped-thread panic で exit 101 → run.mjs が `Unsupported`（tool-side
  termination）と記録。
- 原因は **fixture の stepped Fault param**（`TestEditorProcessor` が
  `TestFaultProcessor` を内包 —— commit `1319c1e5` で追加された fault arm。
  param-mutating case が `FAULT_PARAM` を掃引して `exit`/`abort`/`hang` mode
  を発火し sandbox 子を殺す）であり、これは run.mjs が `test_fault` について
  既に「expected: fatal fault modes are armed by param-mutating cases」と
  annotation している設計どおりの fixture law。**本 slice が変更した面
  （gui extension・win32 helper・Cargo.toml feature）は param 系処理に無関係**
  で、非 fault 全 case（query/scan/process/transport/gui-free）は baseline
  どおり完走している —— 標準 ABI 面に private seam の影響がないことを
  実測で確認した。
- baseline 比較: `evidence/rf630/abi-validator/`（別 worktree
  `rf630-preset-reeval`・旧 HEAD 生成）では test_editor の param 系 case が
  skip だった（当時の fixture は param 面が validator に見えない構成）—
  直接の diff ではないが、非回帰の根拠は上記の機構面で成立する。
- `preset-discovery-crawl`/`load` の 45s timeout は **全 plugin で発生**
  （未タッチの `test_gain` でも同じ —— 本 slice との因果なし）。実走中に
  `clap-validator.exe` の残留 sandbox 子 process が十数個滞留し dll lock まで
  起こした環境 flake であり、post-run に taskkill で掃除済み。lane の
  verdict としては Failed 扱いが正直なためそのまま記録する（Pass 変換
  しない）。
- `run.mjs` の fixture 表は `test_fault` だけが fault-arm death を expected
  扱いで、`test_editor` 側の同 law を annotation していない ——
  `ci/plugin-abi-validator/run.mjs` は lease 外のため本 slice では触らず、
  残件として記録（後続で test_fault と同じ informational 扱いに揃える余地）。

## 実行した検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt` → `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-plugin-clap` | 0 | Pass |
| `mise run check ruviform-plugin-test-clap` | 0 | Pass |
| `mise run lint ruviform-plugin-clap` | 0 | Pass |
| `mise run lint ruviform-plugin-test-clap` | 0 | Pass |
| `mise run test ruviform-plugin-clap` | 0 | Pass（lib + doctest、failed 0） |
| `mise run test ruviform-plugin-test-clap` | 0 | Pass（lib 90 + host_path 26 + doctest、failed 0） |
| `mise exec -- cargo test --locked -p ruviform-plugin-test-clap --test host_path -- --ignored editor_verb_surface_over_real_child_hwnd --nocapture` | 0 | **Pass — real-HWND lane 実走**（interactive window station が存在する環境で全 verb を実 HWND 観測） |
| `mise exec -- prek run --all-files` | 0 | Pass（全 hook） |
| `node ci/plugin-abi-validator/run.mjs --clap-validator … --vst3-validator … --evidence-dir evidence/rf630/editor-verbs-impl/abi-validator` | 非 0 | 実走 —— VST3 leg Passed、CLAP leg は test_editor Unsupported（fault-arm law・非回帰）+ preset-discovery 環境 timeout（上記） |

real-HWND lane で実測した観測点（`editor_verb_surface_over_real_child_hwnd`、
production bridge + private seam 経由の実 `WS_CHILD` attach 上）:

- `send_gui_key` down+up 'a' → child title が
  `…|kd:0061:0041:0000,ku:0061:0041:0000` を記録（`GetWindowTextW`）。
- supplementary-plane `key`（`\u{1F600}`）→ `Err(InvalidValue)`。
- `set_gui_focus(true)` → `f:1`、`EditorElement{7}` → `fe:7:1`。
- `set_gui_focus_target(HostWindow)` → `GetFocus()==parent`（実 SetFocus
  oracle）+ `f:0` token。
- `set_gui_popup(Dialog,true)` → top-level inventory に
  `rvf-test-editor-popup-1`（class `RuviformClapTestEditor`・owner =
  editor child の `GA_ROOT`・`WS_POPUP` set / `WS_CHILD` clear）→
  `po:1`/`pc:1` + destroy 実測。
- `set_gui_popup(ChildWindow,true)` → editor child 配下に
  `rvf-test-editor-popup-2`（`WS_CHILD`）→ `po:2`/`pc:2`。
- title 全文 `rvf-test-editor-attach-1|kd:…,ku:…,f:1,fe:7:1,f:0,po:1,pc:1,
  po:2,pc:2` を一致確認。

## 追加した test 面

- `editor.rs`（unit）: `editor_ops_verbs_reject_undefined_inputs`（
  `created` 前の全 verb 拒否 + 未割当 byte・strict flag・element 規則・
  out-of-range field・repeated popup edge）、
  `editor_ops_verbs_record_event_log_tokens`（全 token 文法 + title 刻印・
  unattached `ChildWindow` の HWND-independent 記録）、
  `destroy_tears_down_the_verb_popup`（popup HWND + logical flag の teardown）。
- `tests/host_path.rs`: `editor_verb_surface_declines_without_open_gui`
  （GUI 未 open の全 verb → `Unsupported`）、
  `non_editor_classes_decline_editor_verbs`（拡張未実装 class →
  `Unsupported`）、`#[ignore]` `editor_verb_surface_over_real_child_hwnd`
  （上記 real-HWND lane —— interactive window station 要、既定 lane 外）。

## 判断・残件

- tag 29/30（`set_gui_focus_target`/`set_gui_popup`）の export 側 GUI
  dispatch 配線は lease 外・後続 slice/別タスク —— L1 到達は tag 26/27 のみ。
- `ci/plugin-abi-validator/run.mjs` の `test_editor` leg が fault-arm death
  （`1319c1e5` で導入済みの fixture law）を expected annotation していない
  —— lease 外のため残件（`test_fault` と同じ informational 扱いへの揃え余地）。
- `preset-discovery-crawl`/`load` 45s timeout は全 plugin 共通の環境 flake
  （validator 残留 process 多数 + 高負荷）— 本 slice 非因果として記録。
- `windows-sys` feature（`Win32_UI_Input_KeyboardAndMouse`）は両 crate の
  `Cargo.toml` に追加（`Cargo.lock` diff は adapter 側 dep edge 1 行のみ）。
- docs index（`docs/**/README.md`）は Worker 契約どおり未編集 ——
  Coordinator が統合時に整備する。設計 doc `status: draft` は変更していない。
- Issue #630 本体 AC（Native Editor 実走・dock-test verdict flip・実 GUI
  受入）は slice 3/5/6 で完結する。本 slice の検証は headless/semantic 層 +
  opt-in real-HWND lane の実走まで。
