---
id: "DESIGN-034XAVJ5JHtkuLmEl8cMmA"
type: "design"
status: "approved"
title: "Issue #389 CLAP/L0 実外部 Editor attach 設計（ClapPlugin::open_gui/close_gui + Windows HWND embed）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/poc/dock_windowing.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/specs/issue-389-plugin-editor-session.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-388-389-native-dock-contract.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  refines:
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-clap-l0-editor-attach.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issue: 389
---

# Issue #389 CLAP/L0 実外部 Editor attach 設計

## Context

VST3 slice（`docs/designs/issue-389-vst3-l0-editor-attach.md`、
merged）が確定した adapter `open_gui`/`close_gui` + PoC manual
lane の型を、CLAP backend へ mirror する。対象は Issue #389 の
うち **CLAP/L0 attach + headless 該当分**のみ — Issue 受入
checkbox は本 slice で close しない。

現在地（main `3b8b7322` 実測）:

- `crates/ruviform-plugin-clap/src/host.rs`（`host` feature）の
  `ClapPlugin` は `open_gui`/`close_gui` を override せず
  `Err(HostError::Unsupported)` の既定。`instance:
  Option<PluginInstance<BridgeHandlers>>` は `init` 済みで保有
  済みのため `plugin_handle()` → `get_extension::<PluginGui>()`
  に到達可能。
- `clack-extensions = "=0.2.0"` は `clack-plugin`/`audio-ports`/
  `params`/`thread-check` feature 有効・`gui` feature は未启用。
  `gui` module（feature `gui`）は host 側 `PluginGui` を提供:
  `is_api_supported(GuiConfiguration)`/`get_preferred_api`/
  `create(GuiConfiguration)`/`destroy`/`get_size -> Option<
  GuiSize>`/`set_size(GuiSize)`/`can_resize`/`get_resize_hints`/
  `adjust_size`/`set_parent(Window)`(unsafe)/`set_transient`(unsafe)/
  `suggest_title`/`show`/`hide`、`GuiConfiguration{api_type,
  is_floating}`、`GuiApiType::{WIN32,COCOA,X11,WAYLAND}`、
  `Window::from_win32_hwnd`(unsafe)。host 側 method 群は
  `clack-extensions/clack-host` feature 配下（`host` feature で
  既に有効）。
- `ParentLease`（`crates/ruviform-plugin-core/src/boundary.rs`）
  が現行 seam — `generation: u32` + `payload: u64`。`payload()`
  の re-widen は OS adapter 境界内のみ（VST3 `open_gui` と同じ
  契約）。
- `ruviform-poc-native-dock` の `vst3` feature +
  `RVF_POC_VST3_PATH`/`RVF_POC_VST3_PLUGIN_ID` lane、および
  `EnumChildWindows`/`first_child_of` による外部 editor HWND 観測は
  実装済み — CLAP も同じ観測経路を再利用する。
- 対象 binary: 本 slice では実第三者 `.clap` を要求しない。
  workspace fixture（`ruviform_plugin_clap` cdylib）は実 CLAP
  binary だが `clap.ext.gui` を実装しない → manual lane は
  load/instantiate/session 経路の実走 + `open_gui → Unsupported`
  の honest 分類を証拠化する。実 GUI attach の Pass は test_editor
  fixture（scarecrow173/ruviform#630 charter・本 slice 対象外）
  または実第三者 plugin の後続 slice が担う。

## Decisions

### D1: `open_gui` の CLAP gui 呼出列（Windows・`host` feature）

`Plugin::open_gui(parent: ParentLease)` → 下記順序で plugin の
`clap.ext.gui` extension を呼ぶ。全呼出は呼出元スレッド（L0
composition では親 HWND の message pump を所有する UI スレッド
= `plugin.init` と同一スレッド）上でのみ行われる
（**thread-affinity 契約**: adapter は pump を所有せず新規
スレッドも作らない。GUI 呼出を RT/audio thread へ流さない契約は
session/host 側と同一。CLAP の gui extension は main-thread
call であり、`plugin_handle()` が返す `PluginMainThreadHandle`
経由でのみ呼ぶ）。

```text
instance.is_none()（dormant）                    → PluginFailed
self.gui.is_some()（gui 既存の二重 open）         → PluginFailed
EDITOR_GUI_API が None（非 Windows）              → Unsupported
handle.get_extension::<PluginGui>() が None       → Unsupported
  （plugin が clap.ext.gui 未実装）
is_api_supported(GuiConfiguration{WIN32, embedded}) → false
                                               → Unsupported
gui.create(same configuration) が Err             → PluginFailed
gui.set_parent(Window::from_win32_hwnd(
    parent.payload() as *mut c_void)) が Err      → destroy して PluginFailed
gui.show() が Err                                → destroy して PluginFailed
gui_seq を increment し GuiHandle を mint、
gui extension handle を self.gui に保持          → Ok(GuiHandle)
```

`GuiConfiguration` は `api_type = GuiApiType::WIN32`・
`is_floating = false`（embedded のみ — floating window は本
slice 範囲外、D7）。`get_preferred_api` は hint であり host の
選択を拘束しないため呼ばない（WIN32 embedded を明示要求）。
`parent.payload()`（u64）→ `*mut c_void` の re-widen と
`Window::from_win32_hwnd`/`set_parent` の unsafe 呼出は OS
adapter 境界内に閉じ込める（`issue-389` session design D2 /
VST3 D1 と同一規約）。`GuiHandle` は instance-local 単調
counter（`gui_seq`、VST3 踏襲）。

### D2: `get_size`/`set_size`/`can_resize`/`adjust_size` は open 経路で呼ばない

CLAP gui 手順書は embedded window に `get_size`（plugin 希望
size の取得 → parent client area 調整）または `set_size`（host
指定 size の付与）を要求するが、本 adapter の `open_gui` は
size 入力を持たない — container/slot 幾何の所有は
`NativeSurfaceSlot`/`set_geometry` 側にあり、VST3 の
`IPlugFrame::resizeView → kResultFalse`（defined refusal）と
同じ責務分担。session の `set_size` op は現行 `Unsupported`
を維持する（D7）。open 成功判定に `get_size` の結果は不要な
ため呼ばない — 呼んで捨てる観測呼出は行わない。

### D3: GUI 寿命 — close/Drop の ordering

CLAP 契約: plugin GUI resource は instance 生存中に
`gui.destroy` で解放する。`close_gui` は `hide`（best-effort）
→ `destroy` を対称に実行し `Ok(())`。`open_gui` 失敗経路は
`create` 成功後の失敗で必ず `destroy` を呼んでから返す
（resource leak なし）。`close_gui` 未 open → `Unsupported`。

`ClapPlugin::drop` は **既存 instance teardown（stop_processing
→ deactivate → `plugin.destroy`）より先に** GUI teardown
（`hide` best-effort + `destroy`）を実行する — VST3 の
view-lifetime ordering（D3）と同じ契約。`hide`/`destroy` の
戻り値は debug assert せず best-effort（`destroy` は infallible、
`hide` の失敗は destroy が資源を解放するため close の成否に
影響しない）。

### D4: platform gate — Windows のみ実装、他 OS は Unsupported

本 dispatch は Windows/CLAP/L0 限定。`EDITOR_GUI_API:
Option<GuiApiType<'static>>` を `#[cfg]` で分岐し、Windows は
`Some(GuiApiType::WIN32)`、非 Windows は `None` → `open_gui` が
honest `Unsupported`。macOS Cocoa・Linux X11/Wayland の embed
経路は検証不能な cfg コードを増やさず**実装しない**
（VST3 `EDITOR_PLATFORM_TYPE` パターン踏襲）。

### D5: PoC `clap` feature — 実外部 module の consumer lane

`ruviform-poc-native-dock` に opt-in `clap` feature を追加する
（`vst3` feature の鏡）:

```toml
clap = ["dep:ruviform-plugin-clap"]
```

optional dep `ruviform-plugin-clap = { path =
"../ruviform-plugin-clap", features = ["host"], optional = true }`。
`gpui` feature 連動は task 側の `--features gpui,clap` 指定で
行い（container HWND は GPUI window 下の実 container が必要）、
feature 間依存は張らない — `vst3` feature と同じ構造。

実行: `RVF_POC_CLAP_PATH=<path\to\.clap>` env で module path を
注入（CLI arg は取らない既存規約・`RVF_POC_CLAP_PLUGIN_ID` で
複数 plugin の選択可）。`register_plugin`（`poc.rs`）は
feature+env の組合せで lane を選択し、probe plugin は全ての
既定経路で維持する:

1. `ClapLibrary::load(path)` — 失敗は `[poc] clap_load=fail
   error=…` を吐いて exit 3（**Fail/NotRun の honest 分類**）
2. `descriptors` を evidence 出力（plugin id/name/vendor/
   version/count）
3. `host.register_factory` + `Instantiate{plugin_id}` →
   session `begin_session` → `Initialize`
4. probe/VST3 と同一の slot/lease/redeem/`open(Docked)` 経路
5. attach 後 `first_child_of(container)`（`EnumChildWindows`
   系の既存観測）で実 editor HWND を発見・証跡化 — plugin が
   `set_parent` で embed した子 HWND を OS 列挙で観測する
6. hide/show/focus/close/release — 既存 stage machine そのまま

`vst3_mode` bool は「外部 plugin mode → editor HWND を
`first_child_of` で観測」の意味であり、CLAP でも同一意味のため
`external_mode` へ改名して 3 scenario（`poc.rs`/`dock.rs`/
`dock_windowing.rs`）で共有する。証拠 line には
`clap_path`/`clap_size`/`clap_descriptors`/`clap_plugin`/
`format` 相当の識別子と `process_boundary=l0` を含め、
TestID・OS・GPU・GPUI rev・3repo SHA は
`evidence/rf389-clap-editor/report.md` の tuple 表に集約する。

`mise.toml` に manual lane task `poc-clap-editor`
（`cargo run -p ruviform-poc-native-dock --features gpui,clap`、
real-OS-only manual lane、description/comment は既存
real-OS lane 様式）を追加し、`docs/development-environment.md`
のタスク表に lane entry を記載する。

### D6: エラー写像（honest taxonomy）

| 条件 | HostError | EditorOpError（session 側） |
| --- | --- | --- |
| dormant（`instance.is_none()`、init 未呼出） | `PluginFailed` | `Plugin` |
| gui 既存の二重 open | `PluginFailed` | `Plugin` |
| 非 Windows（`EDITOR_GUI_API` = `None`） | `Unsupported` | `Unsupported` |
| `clap.ext.gui` extension 不在 | `Unsupported` | `Unsupported` |
| `is_api_supported` false（WIN32 embedded 非対応） | `Unsupported` | `Unsupported` |
| `create`/`set_parent`/`show` の Err | `PluginFailed` | `Plugin` |
| `set_parent`/`show` 失敗後 | `destroy` 実行済みで返す | — |
| close 未 open | `Unsupported` | `Unsupported` |

`Unsupported` は capability 不存在を、`PluginFailed` は plugin
hook の拒否/不正状態呼出を表す既存 file 内規約（`init`/
`start_processing` 誤状態 → `PluginFailed`）に従う。
`GuiError` variant の詳細は adapter 内で `PluginFailed` に
畳み込む — 中立 `HostError` に binding 語彙を漏らさない
（crate policy: binding-neutral API）。

### D7: 範囲外（honest Unsupported / NotRun の維持）

- floating window（`is_floating=true`）・`set_transient`・
  `suggest_title` — embedded のみ実装。
- `set_scale`・`can_resize`・`get_resize_hints`・`adjust_size`・
  `get_size`/`set_size` — D2 のとおり open 経路では呼ばず、
  session `set_size` op は `Unsupported` 維持。
- host 側 `clap.ext.gui` extension（`HostGuiImpl`:
  `request_resize`/`request_show`/`request_hide`/`closed`/
  `resize_hints_changed`）— **宣言しない**。plugin→host の
  GUI 要求 callback は本 slice 範囲外（plugin は host 拡張不在を
  spec 適合に扱う）。
- `clap.ext.gui` 以外の instance extension（`state`・
  `preset-discovery`・`track-info` 等）— 範囲外。
- `save_state`/`restore_state` — `Unsupported` 維持（`state`
  extension 未実装）。
- keyboard/IME・focus 転送 — **NotRun**（container focus は
  editor 入力成功の代理にしない）。
- L1 path — `L1EditorOps`/wire GUI 動詞は現行 `Unsupported` 維持。
- macOS/Linux attach — D4 の honest `Unsupported`。
- `.clap` bundle 形式（macOS `.clap` bundle 解決は clack-host
  側の既存挙動に委譲、本 slice では Windows `.dll`/`.clap`
  単一モジュールを対象）。
- DSP/param/RT の真値 — 変更しない（GUI session のみ）。
- Ruviform 製 test_editor fixture — scarecrow173/ruviform#630
  charter・本 slice 対象外。

## 不変条件・契約

- GUI resource は必ず `destroy`（＋事前 `hide` best-effort）
  されてから instance が `deactivate`/`plugin.destroy` される
  （Drop で強制 — `close_gui` 未呼出でも対称 teardown）。
- 同時に開く GUI は instance あたり 1 つ（session 契約の 1:1
  規則に沿う）。
- `open_gui` 失敗は必ず `create` 成功後の `destroy` を経て返す
  — plugin 側 resource のリークなし。
- `parent.payload()` の re-widen・`Window` 生成・`set_parent`
  の unsafe は `open_gui` の adapter 境界内のみ — 上位層へ
  OS handle を漏らさない。
- GUI 呼出は RT/audio thread へ流れない（既存契約維持）。
- 偽装禁止: extension 不在/API 非対応/attach 失敗を成功と
  報告しない。`GuiHandle` は実 attach 列完了時にのみ mint
  される。

## Consumer 影響分類

| Consumer | 影響 | 根拠 |
| --- | --- | --- |
| audio-lens | なし | plugin-clap `host` feature・poc crate は consumer 非依存 |
| synvelis | 語彙のみ | `ClapPlugin::open_gui` 契約追加。GUI 利用時に SV 側 session wiring が必要（別工程） |

## Deferred Design Concerns

- **plugin→host GUI 要求**（`HostGuiImpl` callback・floating
  window・`closed` 通知）: session/slot 側に「plugin 要求
  resize/show/hide 通知」チャンネルが必要 — RF387 Dock 統合
  時に再検討（VST3 `resizeView` と同じ deferred concern）。
- **size negotiation**: `get_size`/`set_size`/`adjust_size`/
  `can_resize` を session `set_size` op へ接続する設計 —
  container geometry 所有権の確定後。
- **keyboard/IME 転送**: container → editor HWND への
  focus/keyboard forwarding — 実 editor 応答検証と共に別工程。
- **L1 cross-process HWND**: `issue-115` design の SetParent
  cross-process 制約評価と wire GUI 動詞 — L1 契約確定時。
- **macOS/Linux embed**: `EDITOR_GUI_API` を他 OS へ拡張する
  実装と実機検証。
- **実 GUI attach の PASS**: test_editor fixture（#630
  charter）または実第三者 `.clap` binary での manual lane
  実施 — 本 slice は fixture の `Unsupported` 分類まで。

## Verification Notes

- [x] headless adapter 契約: dormant `open_gui`→`PluginFailed`、
  init 済 fixture（`clap.ext.gui` 非搭載）`open_gui`→
  `Unsupported`、未 open `close_gui`→`Unsupported`、既存
  descent 非回帰（test-spec VA-01..06 相当）。
- [x] canonical lane: `mise run fmt-check`/`check`/`lint`/`test`
  （対象 package）+ `check-l1-real-plugin`/`lint-l1-real-plugin`/
  `test-l1-real-plugin` + `test ruviform-plugin-host --test
  editor_session --test editor_session_driver` + `check
  ruviform-poc-native-dock` + `check-ui-driver-boundary` +
  `test-rt`。
- [ ] manual lane `poc-clap-editor`: real-OS-only。workspace
  fixture `.clap` で load→instantiate→session 経路と
  `open_gui→Unsupported` 分類を実走証拠化（実 GUI attach は
  #630 fixture または実第三者 plugin の後続 slice — 本 slice
  では実 attach Pass を主張しない）。
