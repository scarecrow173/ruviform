# Issue #630 editor verb 設計提案 — 調査証跡

`docs/designs/issue-630-editor-verbs.md`（設計提案・`status: draft`）の各判断が
実コードに基づくことを、第三者が再現検証できるように記録した調査証跡である。
コードの変更は一切行っていない（`crates/` は read-only の参照材料）。

## 作業対象

- worktree: `G:/orca/workspaces/ruviform/rf630-editor-verbs-design`
- ブランチ: `docs/630-editor-verbs-design`（`git fetch origin` 後の
  `origin/main` から作成）
- worktree HEAD: `fff784929d5a6dbeaec457ea96f77b55afc2c132`
  （`Merge pull request #940`，`origin/main` と一致）
- `Cargo.lock` の ruviform 解決 rev: 対象外 — `ruviform-plugin-*` クレートは
  全て workspace path dependency（`version = "0.1.0"`、外部 rev なし）であり、
  本調査の対象 SHA は worktree HEAD で一意に決まる。
- 実施日: 2026-11-24

## 確定事実（ファイル・行範囲つき）

### plugin-core: trait 面・境界語彙・wire

| 事実 | 根拠 |
| --- | --- |
| `Plugin::set_gui_focus(&mut self, focused: bool)` は trait default `Err(HostError::Unsupported)` | `crates/ruviform-plugin-core/src/plugin.rs:269-280` |
| `Plugin::send_gui_key(&mut self, event: GuiKeyEvent)` は trait default `Err(HostError::Unsupported)` | `plugin.rs:282-295` |
| `Plugin::set_gui_request_sink` / `Plugin::drain_gui_requests` は plugin→host 方向の request channel（sink = 同期 verdict 系、drain = 非同期 ack 系）。default は sink 破棄・drain 空 | `plugin.rs:297-321` |
| `GuiKeyDirection`（`#[non_exhaustive]`、`Down`/`Up`）・`GuiModifiers`（u16 bitmask、`NONE`/`SHIFT`/`ALTERNATE`/`COMMAND`/`CONTROL`）・`GuiKeyEvent{direction,key,key_code,modifiers}` の定義。`key_code` は「adapter の key-code 空間」で opaque | `crates/ruviform-plugin-core/src/boundary.rs:164-256` |
| `GuiRequest`/`GuiRequestVerdict`/`GuiRequestRefusal`（`Resize`/`Show`/`Hide`/`ResizeHintsChanged`/`Closed` — 全て plugin→host 方向） | `boundary.rs:340-449` |
| `wire::Command::SetGuiFocus{focused:u8}` = tag 26、`SendGuiKey{event}` = tag 27（wire-v6、Issue #872/#882 GUI floating verb 群） | `crates/ruviform-plugin-core/src/wire.rs:709-715, 909-921` |
| `encode`/`decode` 実体: tag 26/27 と `encode_gui_key_event`/`decode_gui_key_event`（`u8` direction + `u32` key char + `i16` key_code + `u16` modifiers） | `wire.rs:1794-1801, 1899-1904, 2200-2235` |
| `WIRE_VERSION = 7`。enum doc は「new verbs arrive only with a `WIRE_VERSION` bump」と明記 | `wire.rs:135, 718` |
| `OpenGuiMode::{Floating,Docked}`（tag 8 payload の mode byte、`Docked` は decode されるが child 層で defined refusal） | `wire.rs:1036-1041, 1824-1832` |
| `HostError` の語彙（`Unsupported`/`PluginFailed`/`InvalidValue`/`Failed`/`Ipc`/`InvalidHandle` 等） | `boundary.rs:663-700` |
| `ErasedPlugin` が `set_gui_focus`/`send_gui_key`/`set_gui_request_sink`/`drain_gui_requests` を mirror し forwarding impl が concrete hook に委譲 | `plugin.rs:436-457, 527-544` |

### plugin-host: session ops・command dispatch

| 事実 | 根拠 |
| --- | --- |
| `EditorSessionOps::set_focus`/`send_key`（`Err` 語彙は `EditorOpError`） | `crates/ruviform-plugin-host/src/editor_session.rs:584-600` |
| `PluginEditorSession::set_focus`/`send_key` — `Attached`\|`Hidden` 要求・plugin 拒否は `Ops(Unsupported)` で non-fatal（state 遷移しない） | `editor_session.rs:1319-1355` |
| `EditorCapabilities` — `dock`/`float`/`resize`/`hide_show`/`reparent`/`ime`/`popup`/`close`/`input`/`focus` の 10 flag（`Support::{Supported,Unsupported}`）。`capabilities_from_bits` が bit 0-9 に写像 | `editor_session.rs:91-138, 1819-1846` |
| `InProcessEditorOps::capabilities()` は `EditorCapabilities::default()`（全 `Unsupported` — L0 は verb 実行前に per-plugin support を証明できないため over-claim しない） | `editor_session.rs:1864-1872` |
| `InProcessEditorOps::set_focus`/`send_key` → `HostCommand::SetGuiFocus`/`SendGuiKey` を `send_command` 経由で dispatch、`HostError`→`EditorOpError` 写像（`map_host_error`） | `editor_session.rs:2010-2044, 1848-1862` |
| `L1EditorOps::set_focus`/`send_key` も同じ `HostCommand` を execute（wire v6 verb として L1 child へ到達可能）。`reattach`/`drain_gui_requests`/`adjudicate_gui_request` は L1 側で defined refusal `Unsupported` | `editor_session.rs:2280-2333` |
| `EditorOpError` 語彙（`Unsupported`/`LeaseRejected`/`Timeout`/`HostLost`/`GraphicsLost`/`Plugin`/`Closed`/`InvalidState`/`Io`/`InvalidValue`/`Internal`） | `editor_session.rs:293-318` |
| `HostCommand::SetGuiFocus`/`SendGuiKey` — 「L1-reachable since wire v6」の doc 明記。`SetGuiParent` は対照的に L0-only（wire verb 非対応） | `crates/ruviform-plugin-host/src/command.rs:129-162` |
| `InProcessHost::execute` が plain slot / shared slot 双方で `plugin.set_gui_focus`/`plugin.send_gui_key` を呼ぶ | `crates/ruviform-plugin-host/src/host.rs:291-292, 1303-1339` |

### VST3 側（pass 済み・既存 pattern）

| 事実 | 根拠 |
| --- | --- |
| adapter: `set_gui_focus`→`IPlugView::onFocus(TBool)`、view 未 open → `Unsupported`、非 `kResultOk` → `PluginFailed` | `crates/ruviform-plugin-vst3/src/host.rs:3431-3449` |
| adapter: `send_gui_key`→`direction` で `onKeyDown`/`onKeyUp` 分岐、`key` は BMP のみ（supplementary-plane は `InvalidValue`）、`GuiModifiers`→`KeyModifier` per-flag 写像 | `host.rs:3451-3486` |
| fixture `TestEditorView`: `onFocus`→`f:{state}`、`onKeyDown`→`kd:{key:04x}:{key_code:04x}:{modifiers:04x}`、`onKeyUp`→`ku:{…}` token を event log（child HWND title `{attach_tag}|{t1},{t2},…`）に append | `crates/ruviform-plugin-test-vst3/src/editor_view.rs:87-93, 162-168, 215-221, 305-317, 363-365` |
| `key_code == 0x50`（`VK_P`）の `onKeyDown` で `popup_toggle` — `po:{seq}`/`pc:{seq}` token + `win32::popup` で実 `WS_POPUP` HWND `rvf-test-editor-popup-{seq}`（editor child の `GA_ROOT` を owner、(0,0) 200x100）を open/close | `editor_view.rs:100-115, 118-135, 186-213, 305-311` |
| `removed()` は child 破棄前に popup を必ず畳む（owner 契約） | `editor_view.rs:278-285` |
| 非 Windows shim は `popup` が `None` を返すが `po:`/`pc:` token は記録される（verb 論理面と HWND 実在面を分離） | `editor_view.rs:684-711` |

### CLAP 側（unsupported の現状）

| 事実 | 根拠 |
| --- | --- |
| `ClapPlugin` が実装する GUI verb は `open_gui`/`close_gui`/`set_gui_size`/`set_gui_visibility`/`set_gui_parent`/`drain_gui_requests`/`drain_dirty_notices` のみ。`set_gui_focus`/`send_gui_key` は未実装 = trait default `Err(Unsupported)` | `crates/ruviform-plugin-clap/src/host.rs:1716-1873`（`fn set_gui_focus`/`fn send_gui_key` の grep ヒット無し） |
| `open_gui` は embedded のみ — `GuiConfiguration{api_type: WIN32, is_floating: false}` を hardcode（design D7: floating は out of scope） | `host.rs:1736-1741` |
| `BridgeShared`（`HostGuiImpl`）が `request_resize`/`request_show`/`request_hide`/`closed`/`resize_hints_changed` を bounded queue（`MAX_GUI_REQUESTS=64`）へ enqueue → `ClapPlugin::drain_gui_requests` で pull | `host.rs:320-449, 1862-1864` |
| `clap_plugin_gui` 規格に key/focus 配送 verb が存在しないことは、adapter が実装した verb 一覧（`is_api_supported`/`create`/`set_parent`/`show`/`hide`/`destroy`/`set_size`/`can_resize`/`get_size`/`set_scale`/`set_transient`/`suggest_title` 系のみ）からも確認できる | `host.rs` 全体 + fixture `editor.rs:522-748` |
| fixture `test_editor`（clap）は floating-mode popup surface を持つ: `is_api_supported` は `WIN32` であれば embedded/floating 両方受理、`set_transient`（IsWindow gate）/`suggest_title`/`show` → `win32::popup(owner,title,size)` で実 `WS_POPUP` HWND | `crates/ruviform-plugin-test-clap/src/editor.rs:529-531, 546-560, 671-732, 977-1007` |
| floating surface は embedded `open_gui` 経路から到達不能（host bridge が `is_floating: false` を固定するため fixture の floating 分岐に入らない） | `host.rs:1738-1741` |
| CLAP fixture の child HWND は `win32::attach` で title `ptr::null()` — VST3 fixture のような event-log title channel は未装備（fixture 側の拡張余地） | `editor.rs:892-928`（`:911` が title=null） |
| fixture unit test: `floating_popup_surface_is_honest` / `size_and_transient_verbs_are_honest` — plugin-impl 単体 test が popup surface の受入 oracle | `editor.rs:1258, 1288, 1360` |

### dock-test catalog・verdict・oracle

| 事実 | 根拠 |
| --- | --- |
| `REAL_CLAP_CATALOG` 14 行中 `real-clap-editor-{input,focus,popup}` の 3 行が `NativePluginDock` のみ required | `crates/ruviform-plugin-dock-test/src/real_catalog.rs:392-406` |
| catalog doc コメントに「`send_gui_key`/`set_gui_focus` は `ClapPlugin` の trait-default `Err(Unsupported)`、embedded `open_gui` は floating に到達しない — これらの case は real lease 経由で verb を実行し defined `Unsupported` landing を正直に記録する（Coordinator decision 1）」と明記 | `real_catalog.rs:348-352` |
| `real_clap_editor_{input,focus,popup}_case` は `clap_op_case` + VST3 と同じ `observe_editor_{input,focus,popup}_lifecycle` を使い、`pass_detail` に「(unused — the CLAP ops surface refuses … with the defined Unsupported)」と記録 | `crates/ruviform-plugin-dock-test/src/real_dock.rs:17410-17475` |
| verdict 体系: `OpLanding::{Refused,Accepted,Blocked,Other}` — `Unsupported` → Refused、`Ok` → Accepted（`effect_confirmed` が Pass を gate、観測不能は `NotRun`）、環境 block → `NotRun`+`environment_blocked` | `real_dock.rs:10880-10928` |
| VST3 popup oracle: class `RuviformTestEditor`・title prefix `rvf-test-editor-popup-`・owner=child の `GA_ROOT`・`WS_POPUP` set/`WS_CHILD` clear・(0,0) 200x100 を `find_fixture_popups`/`check_popup_open_hwnd`/`check_popup_closed_hwnd`/`check_popup_teardown` で検証（oracle は title event log と HWND inventory であって画素ではない） | `real_dock.rs:11706-12014` |
| `POPUP_KEY_CODE = 0x50`（`VK_P`）、`POPUP_OPEN_LOG_SUFFIX = "|kd:0070:0050:0000,po:1,ku:0070:0050:0000"` | `real_dock.rs:11714, 11751` |
| lane 名: `test-ui-plugin-dock-real`（VST3）/`test-ui-plugin-dock-real-headless`/`test-ui-plugin-dock-real-clap`/`test-ui-plugin-dock-real-clap-headless` — 全て manual/opt-in で CI 必須 gate ではない | `mise.toml:191-218, 246-265` |
| fault carrier: `FAULT_PARAM`（`ParamId 0`、plain `[0.0, 8.0]`、stepped 8 mode）を `TestFaultProcessor`（VST3/CLAP 両 fixture 共有）が `process()` 内で in-band dispatch — fixture 拡張の既存流儀 | `crates/ruviform-plugin-test-clap/src/fault.rs:96-151, 226-248`、`crates/ruviform-plugin-test-vst3/src/fault.rs`（同名構成） |

### ledger（read-only 正本の実測値）

| ledger | 実測 `repo.sha` | case 内訳 |
| --- | --- | --- |
| `evidence/rf630/plugin-dock-real-clap.jsonl` | `f6b6d0e2dcf2764f9730975eb7b542593a5fb49b`（run row + `environment.repo_sha` 一致） | 14 case 行 — `real-clap-editor-{input,focus,popup}` は verdict `unsupported`・step detail `the editor verb ran through InProcessEditorOps and the plugin declined it — the defined EditorOpError::Unsupported` |
| `evidence/rf631/plugin-dock-real-scenario.jsonl` | `a0e3456405092605519077988e012130cf82d32d` | 32 case 行 — `real-editor-{input,focus,popup}` は verdict `pass`（title suffix `|kd:0061:0041:0000,ku:0061:0041:0000` / `|f:1,f:0` / `|kd:0070:0050:0000,po:1,ku:0070:0050:0000` + 実 `WS_POPUP` HWND） |

注記: `docs/discovery/issue-630-ac-inventory.md` の ledger 表は CLAP ledger の
sha を `a0e34564` と記しているが、本調査時点の実測では同ファイルの run row・
`environment.repo_sha` はともに `f6b6d0e2…` であった（read-only 正本のため
差分は指摘のみ）。case 数・verdict 内訳は inventory の記述と一致する。

## 用いた列挙コマンド（全て worktree ルートで実行・exit 0）

| コマンド | 用途 | 結果 |
| --- | --- | --- |
| `git fetch origin && git rev-parse HEAD origin/main` | 対象 SHA 確定 | 両者 `fff78492…` 一致 |
| `grep -n "fn set_gui_\|fn send_gui_key\|fn open_gui\|fn close_gui\|fn set_gui_request_sink\|fn drain_gui_requests\|is_api_supported\|is_floating\|set_transient\|suggest_title" crates/ruviform-plugin-clap/src/host.rs` | CLAP adapter の GUI verb 面列挙 | 10 件ヒット — `set_gui_focus`/`send_gui_key` の実装無しを確認 |
| `grep -n "GuiRequest\|GuiRequestSink" crates/ruviform-plugin-core/src/boundary.rs` | request channel 語彙の所在 | 6 件（`:354-449` に enum 群） |
| `grep -n "SendGuiKey\|SetGuiFocus\|GuiKeyEvent\|26 =>\|27 =>" crates/ruviform-plugin-core/src/wire.rs` | wire tag・codec 位置特定 | 14 件ヒット |
| `grep -n "WIRE_VERSION" crates/ruviform-plugin-core/src/wire.rs` | wire version 現行値 | `pub const WIRE_VERSION: u16 = 7`（`:135`） |
| `grep -n "L1EditorOps\|impl EditorSessionOps\|fn send_key\|fn set_focus" crates/ruviform-plugin-host/src/editor_session.rs` | session ops 実装の所在 | 11 件ヒット |
| `grep -n "fn \|onFocus\|onKeyDown\|onKeyUp\|popup\|event_log\|title" crates/ruviform-plugin-test-vst3/src/editor_view.rs` | VST3 fixture の verb 応答面 | 80+ 件（行範囲は上表） |
| `grep -n "fn \|floating\|transient\|popup\|title\|hwnd\|show\|destroy" crates/ruviform-plugin-test-clap/src/editor.rs` | CLAP fixture の verb 応答面 | 80+ 件（行範囲は上表） |
| `grep -n "fn popup\|pub fn attach\|pub fn destroy\|pub fn set_title\|pub fn is_window\|pub fn resize\|pub fn set_visible" crates/ruviform-plugin-test-clap/src/editor.rs crates/ruviform-plugin-test-vst3/src/editor_view.rs` | 両 fixture の win32 helper 比較 | CLAP: `popup(owner,title,size)`、VST3: `popup(anchor,title)` — 署名差を確認 |
| `grep -n "observe_editor_popup_lifecycle\|fn observe_editor_input_lifecycle\|fn observe_editor_focus_lifecycle\|popup_title\|rvf-test-editor-popup\|EnumWindows" crates/ruviform-plugin-dock-test/src/real_dock.rs` | popup oracle 所在 | 8 件ヒット（`:11633-12014`） |
| `grep -n "fn editor_op_body\|OpLanding\|enum OpLanding" crates/ruviform-plugin-dock-test/src/real_dock.rs` | verdict mapping 所在 | `enum OpLanding` は `:10883` |
| `grep -h "editor-input\|editor-focus\|editor-popup" evidence/rf630/plugin-dock-real-clap.jsonl evidence/rf631/plugin-dock-real-scenario.jsonl` | 両 ledger の対象 case 行 | CLAP 3 行 `unsupported`・VST3 3 行 `pass` |
| `grep -o '"sha":"[a-f0-9]*"\|"repo_sha":"[a-f0-9]*"\|"sha256":"[a-f0-9]*"' evidence/rf631/plugin-dock-real-scenario.jsonl \| sort -u` | VST3 ledger sha 実測 | `a0e34564…` + plugin artifact sha256 群 |
| `grep -o '"sha":"[a-f0-9]*"' evidence/rf630/plugin-dock-real-clap.jsonl \| sort -u` | CLAP ledger sha 実測 | `f6b6d0e2…`（inventory 記述との差分を上に注記） |
| `grep -c "verdict" evidence/rf630/plugin-dock-real-clap.jsonl evidence/rf631/plugin-dock-real-scenario.jsonl` | ledger case 行数 | 14 行 / 32 行 |
| `grep -n "dock-real-clap\|dock-real\b\|blocked-probe" mise.toml` | opt-in lane 名 | `:191-265` に 4 lane |
| `grep -rn "EditorCapabilities" crates/ruviform-plugin-{core,host}/src/*.rs` | capability flag 語彙 | `editor_session.rs:117` に定義 |
| `grep -rn "enum Support" -A 12 crates/ruviform-plugin-host/src/editor_session.rs` | `Support` enum | `Supported`/`Unsupported` 2 値（`:91-97`） |
| `grep -rn "FAULT_PARAM\|TestFaultProcessor" crates/ruviform-plugin-test-{clap,vst3}/src/fault.rs` | fault carrier pattern | `FAULT_PARAM`（`ParamId 0`、8 mode）両 crate に存在 |

## 参照した既存文書（read-only）

- `docs/designs/issue-630-clap-editor-verbs.md`（approved — resize 対称拡張先例）
- `docs/designs/issue-630-fault-modes.md`（approved — fault carrier・popup 契約先例）
- `docs/designs/issue-631-editor-input.md`（approved — 本設計の直接の先行: VST3 側
  `send_gui_key`/`set_gui_focus` 実装の設計記録。boundary 語彙・L0-only/L1-reachable
  command・fixture event log・title-suffix oracle の確立元）
- `docs/designs/issue-631-real-editor-attach.md`（front matter 形式の参照）
- `docs/specs/issue-630-test-editor.md`（fixture 契約の正本）
- `docs/discovery/issue-630-ac-inventory.md`（AC census — AC4 の implementable
  分類と ledger 内訳の根拠）
- `POLICIES.md`（OS-L1/GUI-L2/EXT-L3、Required/Deferred、Pass/Fail/NotRun/
  Unsupported の規範）
