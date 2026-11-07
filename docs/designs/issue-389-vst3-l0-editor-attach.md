---
id: "DESIGN-8EX9crwtfdh9PhZtOJ2cfY"
type: "design"
status: "approved"
title: "Issue #389 VST3/L0 実外部 Editor attach 設計（Vst3Plugin::open_gui/close_gui + Windows HWND attach）"
created: "2026-11-07"
updated: "2026-11-07"
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
        path: "crates/ruviform-plugin-vst3/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/README.md"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/src/main.rs"
      - type: "file"
        path: "crates/ruviform-poc-native-dock/README.md"
      - type: "file"
        path: "mise.toml"
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
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-388-389-native-dock-contract.md"
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-vst3-l0-editor-attach.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issue: 389
---

# Issue #389 VST3/L0 実外部 Editor attach 設計

## Context

Issue #389 design（`issue-389-plugin-editor-session.md`）が確定した
PluginEditorSession/EditorSessionOps 契約は PR399 で実装済み。
同設計の明示的 deferred concern:

- 各 OS adapter の `open_gui` 実装本体（VST3 `IPlugView` の HWND
  attach）— **本設計が担当する範囲**。
- 実 Native Dock PASS（GPUI dock layout widget 管理下）— RF387 側。
- L1 cross-process GUI — `issue-115` design で別途検討（現行
  `L1EditorOps` は全 op 明示 Unsupported、wire 未到達）。

現在地（main e8adcc0 実測）:

- `crates/ruviform-plugin-vst3/src/host.rs`（`host` feature）の
  `Vst3Plugin` は `open_gui`/`close_gui` を override せず
  `Err(HostError::Unsupported)` の既定。
  `controller: Option<ComPtr<IEditController>>` は `init` 済みで
  保有済みのため `IEditController::createView` に到達可能。
- `vst3 = "0.3.0"` binding crate は `IEditControllerTrait::createView`、
  `IPlugViewTrait`（`isPlatformTypeSupported`/`attached`/`removed`/
  `getSize`/`onSize`/`onFocus`/`setFrame`/`canResize`/
  `checkSizeConstraint`）、`IPlugFrameTrait`（`resizeView`）、
  `Steinberg::Vst::ViewType::kEditor`、
  `kPlatformTypeHWND`/`kPlatformTypeNSView`/
  `kPlatformTypeX11EmbedWindowID`、`ViewRect` を提供する。
- `InProcessEditorOps::open`（Docked のみ）は `LeaseRedeemer` 経由で
  `GuiParentToken` を得て `HostCommand::OpenGui` →
  `plugin.open_gui(parent)` を駆動済み。`set_size`/
  `set_visibility`/`reattach` は honest `Unsupported`（slot 側の
  container OS 操作が geometry/visibility を担当する分担）。
- `ruviform-poc-native-dock`（consumer composition root）は probe
  plugin による実 WS_CHILD attach PoC を実装済み。実外部 plugin
  binary 経路は「binary 未入手」で NotRun と記録されていた。
- 候補 binary: `C:\Program Files\Common Files\VST3\iZotope\
  Vinyl.vst3`（PE32+ x86-64 DLL、2,045,952 bytes、同 dir に
  `iZVinyl.dll` 64,826,368 bytes）。既インストール品のみ使用、
  installer/SDK/binary 追加取得は行わない。

## Decisions

### D1: `open_gui` の VST3 呼出列（Windows・`host` feature）

`Plugin::open_gui(parent: GuiParentToken)` → 下記順序で
`IPlugView` を生成・attach する。全呼出は呼出元スレッド
（L0 composition では親 HWND の message pump を所有する UI スレッド）
上でのみ行われる（**thread-affinity 契約**: adapter は pump を
所有せず新規スレッドも作らない。GUI 呼出を RT/audio thread へ
流さない契約は session/host 側と同一）。

```text
phase >= Initialized && controller.is_some()   else PluginFailed
view.is_none()                                 else PluginFailed（二重 open）
createView(ViewType::kEditor)  → null          → Unsupported（editor 非搭載）
view.isPlatformTypeSupported(kPlatformTypeHWND) → false
                                               → Unsupported
frame = ComWrapper<BridgePlugFrame>             （resizeView は honest refusal）
view.setFrame(frame.as_ptr())  ≠ kResultOk      → PluginFailed
view.attached(parent.raw() as *mut c_void, kPlatformTypeHWND)
                               ≠ kResultOk      → PluginFailed（view release）
GuiHandle::new(++gui_seq) を mint し view/frame を保持
```

`GuiParentToken::raw()`（u64）→ `*mut c_void` の re-widen は OS
adapter 内部で行う（`issue-389` design D2: raw 語彙は adapter
境界内限定）。`GuiHandle` は instance-local 単調 counter。
`close_gui` は `view` 存在時のみ `removed()` → release →
frame release → `Ok(())`、未 open 時は `Unsupported`。

### D2: `BridgePlugFrame` — plugin-initiated resize の honest refusal

`IPlugView::setFrame` は attach 前に非 null の `IPlugFrame` を
要求し得る（SDK 契約）。`BridgePlugFrame` を bridge 側 COM host
object として `BridgeHostApplication`/`BridgeComponentHandler` と
同じ `Class`/`ComWrapper`/`Trait` パターンで実装する。

`resizeView(view, newSize)` は **`kResultFalse`** を返す。根拠:
container HWND の geometry 所有は slot 側（`NativeSurfaceSlot`/
`set_geometry`）にあり、plugin adapter から消費者 container を
resize する経路は中立 surface に存在しない。`kResultOk` や
無断 resize は「resize 成功の偽装」あるいは責務越権であり、
`kResultFalse`（拒否）が honest な回答。`EditorCapabilities.
resize` が `Unsupported` を宣言する現行 ops 契約と整合する。
（`kNotImplemented` ではなく `kResultFalse`: resize は実装済みの
上での拒否として plugin が扱える「拒否」応答）

### D3: view 寿命 — Drop/close の ordering

VST3 契約: view は `removed()` され release された後でなければ
controller へ `terminate` できない。`close_gui` が呼ばれず Drop
へ至った場合でも同一の対称 teardown を保証するため、
`Vst3Plugin::drop` は **既存 phase descent の前に** view teardown
（`removed()` 呼出 + ComPtr release + frame release）を実行する。
`removed()` の戻りは debug assert のみ（release ビルドは
best-effort）— 既存 `setProcessing(false)`/`setActive(false)`/
`terminate` の debug-assert 方針と同じ。

### D4: platform gate — Windows のみ実装、他 OS は Unsupported

本 dispatch は Windows/VST3/L0 限定。platform type 選択は
`#[cfg]` ではなく runtime の `platform_type()` helper が
`Option<FIDString>` を返す形にし、`None`（非 Windows）→
`Unsupported`。macOS NSView/X11EmbedWindowID は検証不能な
cfg コードを増やさず**実装しない**（honest Unsupported を維持）。

### D5: PoC `vst3` feature — 実外部 module の consumer lane

`ruviform-poc-native-dock` に opt-in `vst3` feature を追加する:

```toml
vst3 = ["gpui", "dep:ruviform-plugin-vst3", "ruviform-plugin-vst3/host"]
```

`gpui` feature 連動（container HWND は GPUI window 下の実 container
が必要）。`ruviform-plugin-vst3` dep は optional（headless lane
不変、`cargo check --workspace` は引き続き GPUI なしで green）。

実行: `RVF_POC_VST3_PATH=<path\to\.vst3>` env で module path を
注入（CLI arg は取らない既存規約・`RVF_POC_VST3_PLUGIN_ID` で複数
class の選択可）。`register_plugin`（`poc.rs`）は:

1. `Vst3Library::load(path)` — 失敗は `[poc] vst3_load=fail error=…`
   を吐いて exit 3（**Fail/NotRun の honest 分類**）
2. `descriptors`/`params` を evidence 出力（plugin name/vendor/
   version/id/params count）
3. `host.register_factory` + `Instantiate{plugin_id}` →
   session `begin_session`
4. probe PoC と同一の slot/lease/redeem/`open(Docked)` 経路
5. attach 後 `EnumChildWindows(container)` で実 editor HWND を
   発見・証跡化（`parent_of` で実親確認。probe とは違い adapter
   内部所有の HWND を OS 列挙で観測する）
6. editor の実 rect を読み `set_geometry` で container を plugin
   preferred size へ resize → editor rect の追の追従を観測（追従は
   plugin の挙動次第 — **観測値をそのまま記録し成功を偽装しない**）
7. hide/show/focus/close/release — probe と同 stage machine

証拠 line には `plugin_path`/`plugin_sha256`（report 側で
`Get-FileHash` 併記）/`descriptor`/`format=vst3`/
`process_boundary=l0` を含め、TestID・OS・GPU・GPUI rev・
3repo SHA は `evidence/rf389-vst3-l0-editor/report.md` の tuple
表に集約する。

### D6: エラー写像（honest taxonomy）

| 条件 | HostError | EditorOpError（session 側） |
| --- | --- | --- |
| `phase < Initialized`（controller 未存在） | `PluginFailed` | `Plugin` |
| `createView` null / platform 非対応 | `Unsupported` | `Unsupported` |
| `setFrame`/`attached` 非 ok | `PluginFailed` | `Plugin` |
| 二重 open | `PluginFailed` | `Plugin` |
| close 未 open | `Unsupported` | `Unsupported` |
| `attached` 後の後続失敗 | —（view を removed+release してから返す） | — |

`Unsupported` は capability 不存在を、`PluginFailed` は plugin
hook の拒否/不正状態呼出を表す既存 file 内規約（`init`/
`start_processing` 誤状態 → `PluginFailed`）に従う。

### D7: 範囲外（honest Unsupported / NotRun の維持）

- `IPlugView::onSize`/`checkSizeConstraint` の host→plugin 通知、
  `set_size`/`set_visibility`/`reattach` session op — L0 ops の
  現行 `Unsupported` を維持（resize/visibility は slot 側 container
  操作で代行、plugin への onSize 通知は未実装）。
- `onFocus`/`onKeyDown`/`onKeyUp`/`onWheel` キー転送 — **NotRun**
  （editor への実入力到達は別工程。container focus は editor
  入力成功の代理にしない）。
- `IPlugViewContentScaleSupport` — attach 証跡に不要なため実装しない。
- `IConnectionPoint` — **実機検証で必須と判明し実装済み**: distributed
  plugin（Vinyl は `single_component=false`）は component↔controller 未接続で
  `createView` が null を返す。接続時の `notify` が `IMessage` を
  `IHostApplication::createInstance` に要求するため `BridgeMessage`/
  `BridgeAttributeList` host class も実装（設計時「不要」判断を実機証拠で更新）。
  それ以外の任意 interface（`IUnitInfo`/`IContextMenu` 等）は範囲外。
- `createView` の `"editor"` 以外の view type — 扱わない。
- L1 path — `L1EditorOps` 現行 Unsupported を維持（wire GUI
  動詞未到達の既存設計を黙って動作済みにしない）。
- CLAP/AU adapter — 別 owner（無断拡張しない）。
- `.vst3` ディレクトリ bundle 形式（`Contents/x86_64-win/`）—
  対象 binary は単一 PE モジュール。bundle 解決は範囲外。
- DSP/param/RT の真値 — 変更しない（GUI session のみ）。

## 不変条件・契約

- view は必ず `removed()` + release 後にしか controller から
  `terminate` されない（Drop で強制）。
- 同時に開く view は instance あたり 1 つ（VST3 一般は
  multi-view を plugin 側が拒否し得るが、session 契約の
  1:1 規則に沿う）。
- `open_gui` 失敗は必ず `view`/`frame` を release してから返す
  — COM 参照のリークなし。
- GUI 呼出は RT/audio thread へ流れない（既存契約維持）。
- 偽装禁止: platform 非対応/null view/attach 失敗を成功と報告
  しない。`GuiHandle` は実 attach 成功時にのみ mint される。

## Consumer 影響分類

| Consumer | 影響 | 根拠 |
| --- | --- | --- |
| audio-lens | なし | plugin-vst3 `host` feature・poc crate は consumer 非依存 |
| synvelis | 語彙のみ | `Vst3Plugin::open_gui` 契約追加。GUI 利用時に SV 側 session wiring が必要（別工程） |

## Deferred Design Concerns

- **Plugin-initiated resize 経路**: `resizeView`→`onSize` の
  roundtrip を中立 surface へ乗せるには slot 側に「plugin 要求
  size 通知」チャンネルが必要 — RF387 Dock 統合時に再検討。
- **キー/IME 転送**: container → editor HWND への focus/keyboard
  forwarding、および IME — 実 editor 応答検証と共に別工程。
- **L1 cross-process HWND**: `issue-115` design の SetParent
  cross-process 制約（同一 integrity/UIPI）評価と wire GUI 動詞 —
  L1 契約確定時。
- **macOS NSView/X11Embed**: `platform_type()` を他 OS へ拡張する
  実装と実機検証。
