---
id: "DESIGN-034UjQBuwtfaWhLg6ZZMBn"
type: "design"
status: "approved"
title: "Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "docs/specs/issue-257-driver-neutral-ui-contract.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/specs/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/plans/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/tasks/issue-257-driver-neutral-ui-contract.md"
      - type: "doc"
        path: "docs/test-specs/issue-257-driver-neutral-ui-contract.md"
    modified:
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-257-driver-neutral-ui-contract.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-257-driver-neutral-ui-contract.md"
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
    - "docs/designs/issue-386-driver-neutral-gui-host.md"
    - "docs/designs/issue-387-component-view-specs.md"
    - "docs/designs/issue-390-driver-contract-lab.md"
    - "docs/designs/issue-464-consumer-neutral-gui-scenario-contract.md"
    - "docs/designs/issue-451-focus-next-wiring.md"
    - "docs/designs/issue-451-keyboard-tab-nav.md"
    - "docs/designs/issue-451-tabs-pointer-descriptor.md"
    - "docs/designs/issue-478-fader-knob-height-contract.md"
    - "docs/designs/issue-479-keyboard-value-controls.md"
    - "docs/specs/issue-465-legacy-webview-p0p1.md"
    - "docs/adr/issue-465-legacy-webview-retirement-status.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-257-driver-neutral-ui-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/451"
    - "https://github.com/scarecrow173/ruviform/issues/455"
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/478"
    - "https://github.com/scarecrow173/ruviform/issues/479"
    - "https://github.com/scarecrow173/synvelis/issues/112"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  defers: []
  deferred-by: []
---

# Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合設計

## Context

Issue #257 は「Driver-neutral Reusable UI Contract — GPUI は標準
Presentation Driver」の**横断設計正本**を要求する親 Issue である。
契約の実体は子 Issue（#385〜#390）と後続契約 Issue（#464・#451・
#478・#479）の merge 済み文書・実装型に分散して蓄積されており、
本書はそれらを**再定義せず参照として統合**し、Issue #257 の受入
項目ごとの証拠対応を固定する索引・正本である。

**優先順位ルール（Issue #257 本文より）**: 2026-11-07 確定仕様 v2・
2026-11-08 現行契約補足（旧 WebView 非サポート・撤去対象）・
2026-11-09 UI 依存方針（#384/#465 を現行基準）が、それ以前の
WebView-first / SolidJS / DOM・CustomEvent 前提の記述に優先する。
本書もこの順序に従う。

**GPUI 採否の状態（重要）**: GPUI の最終採用判定は**未確定**である。
#262（採否 PoC）は Open で verdict pending、#419（後継 ADR）が
残 DoD を所有し、#388/#389（Native Dock 実機 PoC）も Open である。
本書は契約の所在を索引するものであり、**GPUI 採用確定の根拠文書
ではない**。Driver-neutral 契約層自体は GPUI 非依存で維持される
（2026-11-08 補足どおり）。

文書チェーン: [spec](../specs/issue-257-driver-neutral-ui-contract.md)・
[plan](../plans/issue-257-driver-neutral-ui-contract.md)・
[task](../tasks/issue-257-driver-neutral-ui-contract.md)・
[test-spec](../test-specs/issue-257-driver-neutral-ui-contract.md)。

## Scope

### In Scope

- 蓄積済み契約の領域別参照索引（文書・型・fixture・検証 lane）。
- Issue #257 受入項目ごとの証拠対応表。
- 歴史的経緯 / 廃止済み参照の区別一覧。
- 未確定事項（GPUI 採否・Native Dock・2-consumer slice）の明記。

### Out of Scope

- 契約内容の再定義・変更提案（各正本は参照先が所有）。
- コード・API・テストの変更。既存他 Issue 文書の編集。
- #262 verdict・#419 後継 ADR・#388/#389 PoC・SV#112 consumer slice
  の実作業。Issue #257 の Close 判断。

## Contract Canonical Index

各契約領域の**正本**は参照先文書・型であり、本書は索引のみを持つ。
「状態」列は本書作成時点（2026-11-09、main `19371783`）の merge /
Issue 状態。

### A. アーキテクチャ方針・依存方向・Driver 抽象（#384）

| 項目 | 正本 |
| --- | --- |
| 後継 ADR（Driver-neutral GUI / GPUI 標準 Driver 仮決定） | [driver-neutral-gui-gpui-driver-webview-first-issue-384.md](../adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md)（status: proposed = 仮決定。最終採否は #262/#419） |
| spec | [issue-384-driver-neutral-gui-architecture.md](../specs/issue-384-driver-neutral-gui-architecture.md)（approved） |
| 契約詳細設計（ID 体系・crate 配置・公開 API ルール・移行表） | [issue-384-driver-neutral-gui-contract.md](issue-384-driver-neutral-gui-contract.md)（approved） |
| plan / task | [rf384-rf385-…plan](../plans/rf384-rf385-driver-neutral-gui-contract-ruviform-ui-theme-dtcg-interpreter-issue.md) / [implement-rf384-rf385-…task](../tasks/implement-rf384-rf385-driver-neutral-contract-docs-ruviform-ui-theme-crate.md) |
| 実装 crate | `crates/ruviform-ui`（契約層）・`ruviform-ui-gpui`（GPUI adapter）・`ruviform-ui-webview`（legacy adapter）・`ruviform-ui-test`（Fake/test）・`ruviform-ui-theme` |
| 境界 guard | `mise run check-ui-driver-boundary`（`crates/ruviform-ui-test/src/driver_boundary.rs` + `src/bin/gui_boundary_guard.rs`、#390） |

要点（正本側の要約・再定義ではない）: 共通契約は `gpui::*`・DOM・
wry/tao・OS handle・JS eval を公開 signature に出さない。例外は
composition root・明示 driver extension・driver crate 自身のみ。
依存方向は一方向（Driver → 共通契約、Consumer → 共通契約）。

### B. Theme / Design Token（#385）

| 項目 | 正本 |
| --- | --- |
| spec / design / test-spec | [specs/issue-385-dtcg-resolved-theme.md](../specs/issue-385-dtcg-resolved-theme.md) / [designs/issue-385-dtcg-resolved-theme.md](issue-385-dtcg-resolved-theme.md) / [test-specs/issue-385-dtcg-resolved-theme.md](../test-specs/issue-385-dtcg-resolved-theme.md) |
| crate | `crates/ruviform-ui-theme`（`doc.rs`/`resolve.rs`/`snapshot.rs`/`registry.rs`/`errors.rs`。DTCG profile `rvf-dtcg-profile.v1` → typed 不変 `ResolvedTheme`。GPUI・`ruviform-ui` 非依存） |
| 実装 PR | #397（#384/#385 契約 ADR + interpreter）、#417（Theme Registry + golden fixtures） |

### C. Window / Host / Driver-neutral 契約（#386）

| 項目 | 正本 |
| --- | --- |
| spec / design / test-spec | [specs/issue-386-driver-neutral-gui-host.md](../specs/issue-386-driver-neutral-gui-host.md) / [designs/issue-386-driver-neutral-gui-host.md](issue-386-driver-neutral-gui-host.md) / [test-specs/issue-386-driver-neutral-gui-host.md](../test-specs/issue-386-driver-neutral-gui-host.md) |
| plan / tasks | [plans/implement-issue-386-…](../plans/implement-issue-386-driver-neutral-gui-host.md) / `docs/tasks/t1..t5-*-issue-386.md` |
| inventory / IR | [discovery/issue-386-…inventory](../discovery/issue-386-driver-neutral-gui-host-inventory.md) / [impl/ir/issue-386-…](../impl/ir/issue-386-driver-neutral-gui-host.md) |
| 実装 PR | #394（`driver.rs` 契約 + `ruviform-ui-gpui` 最小 Window fixture）、#422（Fake/GPUI 共通 lifecycle fixture + 旧 API 移行表） |

契約型（`crates/ruviform-ui/src/`、実在確認済み）:

- **Window / Driver**: `driver.rs` — `WindowSpec`・`WindowLifecycle`
  （`Created`/`Ready`/`Resized`/`Moved`/`Focused`/`ScaleFactorChanged`/
  `CloseRequested`/`Destroyed`/`Action(Intent)`）・`DriverEvent`・
  `WindowDriver` trait・`WindowCapabilities`/`Capability`・
  `UiError::Unsupported`（`error.rs`）・`LogicalSize`/`LogicalPosition`。
- **Window 識別**: `window.rs` — `WindowId`（NonZeroU64、再利用なし）、
  `CloseDecision`、`MonitorInfo`。旧 `WindowHandle`/`WindowConfig`
  は WebView 経路の残置 API（後述「歴史的参照」）。
- **View / Dock**: `dock.rs` — `ViewInstanceId`・`DockWorkspace`・
  `DockTree`/`DockNode`・2-phase move（`begin_move`/`commit_move`/
  `abort_move`）・`WindowCloseDisposition`・`LayoutSnapshot`。
- **IPC / Binding**: `ipc.rs` — `Intent`/`Event`/`Envelope`/
  `parse_intent`（`rvf.ipc.v1`）。`behavior.rs` — `ValueDescriptor`/
  `ValueMappingSpec`/`ValueMapper`・`InteractionControl`/
  `InteractionDispatcher`・`ControlInput`/`InteractionEvent`・
  `CommandIntent`/`CommandReceipt`/`StateProjection`/
  `CommandBinding`/`CommandPhase`。
- **View spec / lifecycle**: `view.rs` — `ComponentKey`・
  `ComponentSpec`/`ViewSpec`・`PanelSpec`/`TabsSpec`/`ControlSpec`/
  `WaveformSpec`/`MeterSpec`/`SpectrogramSpec`/`TimeAxisSpec`/
  `TableSpec`/`ButtonSpec`/`LabelSpec`/`NativeSlotSpec`・
  `ViewHost`（`mount`/`update`/`unmount`/`detach`/`attach`/
  `dispatch_input`/`select_row`/`select_tab`/`invoke`/`observe`/
  `apply_receipt`/`apply_projection`/`deliver_event`/`drain_events`）・
  `AttachEpoch`/`AttachEpochAllocator`・`generation()`・
  `ProjectionOp`/`ViewEvent`/`ViewError`（`StaleGeneration` 等）・
  `ComponentSnapshot`。
- **Native slot（契約層）**: `native_slot.rs` — `SurfaceSlotId`・
  `SlotLease`・`NativeSurfaceSlot` trait・`SurfaceSlotRegistry`・
  `SlotCapabilities`/`SlotError`/`SlotState`/`SlotEvent`。
  `WindowCapabilities::native_surface_slot` が宣言点。
- **旧 WebView 残置**: `event.rs`（`HostEvent`/`HostHandle`）・
  `runtime.rs`（`UiRuntime`。`dispatch_window_intent` が新経路）・
  `render.rs`（`RenderTier`/`NativeSurfaceHost`）・`bridge.rs`
  （`BRIDGE_JS`/`dispatch_script`）・`assets.rs`/`data.rs`/
  `service.rs`/`detect.rs`（共通維持）。移行表の正本は
  [issue-386 design](issue-386-driver-neutral-gui-host.md)「旧 API →
  新 API 移行表」。

### D. Component / View Specs・Headless Behavior・Renderer（#387）

| 項目 | 正本 |
| --- | --- |
| spec / design / test-spec | [specs/issue-387-component-view-specs.md](../specs/issue-387-component-view-specs.md) / [designs/issue-387-component-view-specs.md](issue-387-component-view-specs.md) / [test-specs/issue-387-component-renderer.md](../test-specs/issue-387-component-renderer.md) |
| plan / tasks | [plans/implement-issue-387-…](../plans/implement-issue-387-component-renderer.md) / `docs/tasks/t1..t4-*-issue-387.md` |
| inventory | [discovery/issue-387-…inventory](../discovery/issue-387-component-view-inventory.md) |
| 実装 PR | #406（view/behavior 契約 + headless behavior + GPUI renderer/mount seam + NativeSlot glue）、#420（Knob/Toggle/Meter/TimeAxis） |

要点: `ViewHost` が唯一の authoritative projection engine。
GPUI renderer（`crates/ruviform-ui-gpui/src/render.rs`、crate-private）
は spec → element 写像と入力正規化のみ。mount seam は
`ViewMount` + `GpuiDriver::create_view_window`（GPUI 型を公開 API に
出さない）。NativeSlot glue は `bind_native_slot`/`unbind_native_slot`/
`surface()`（`gpui_driver/surface.rs`）。

### E. Driver Contract Lab / Boundary Guard / Fake Driver（#390）

| 項目 | 正本 |
| --- | --- |
| spec / design / test-spec | [specs/issue-390-driver-contract-lab.md](../specs/issue-390-driver-contract-lab.md) / [designs/issue-390-driver-contract-lab.md](issue-390-driver-contract-lab.md) / [test-specs/issue-390-driver-boundary-lab.md](../test-specs/issue-390-driver-boundary-lab.md) |
| plan / task / report / inventory | [plans/issue-390-…](../plans/issue-390-driver-contract-lab.md) / [tasks/issue-390-…](../tasks/issue-390-driver-contract-lab.md) / [reports/issue-390-driver-lab.md](../reports/issue-390-driver-lab.md) / [discovery/issue-390-…inventory](../discovery/issue-390-driver-contract-lab-inventory.md) |
| 実装 | `crates/ruviform-ui-test/src/driver_boundary.rs`(+`driver_boundary/sig.rs`)・`driver_lab.rs`・`fake_driver.rs`・`fake.rs`・`fake_native_slot.rs`・`lifecycle.rs`・`headless.rs`・`fixtures/driver-lab/{boundary,cases,stories}.json` |
| 実装 PR | #396（Boundary Guard + Driver Lab registry）、#425（fake 実行 fixture・manifest 実態化・境界 pin） |
| 検証 lane | `mise run check-ui-driver-boundary` / `test-ui-driver-contract` / `test-ui-parity` / `test-ui-native-slot` / `test-ui-component-view` / `test-ui-golden` |

判定語彙: `pass`/`fail`/`not_run`/`unsupported`/`blocked`。UI-DVR
case（UI-DVR-01..08）と story registry（`LAB-*`）が manifest の正本。

### F. Scenario Contract — consumer-neutral 共通実行（#464）

| 項目 | 正本 |
| --- | --- |
| spec / design / test-spec | [specs/issue-464-…](../specs/issue-464-consumer-neutral-gui-scenario-contract.md) / [designs/issue-464-…](issue-464-consumer-neutral-gui-scenario-contract.md) / [test-specs/issue-464-…](../test-specs/issue-464-consumer-neutral-gui-scenario-contract.md) |
| plan / task | [plans/issue-464-…](../plans/issue-464-consumer-neutral-gui-scenario-contract.md) / [tasks/issue-464-…](../tasks/issue-464-consumer-neutral-gui-scenario-contract.md) |
| 実装 | `crates/ruviform-ui-test/src/scenario.rs`（`SemanticGuiTest`・`TargetQuery`/`ScenarioTarget`・`ScenarioCapability`/`ScenarioDriver`・`ScenarioVerdict`/`ScenarioError`/`ScenarioReport`・`GuiAction`・`MintedIntent`・`begin`/`preview`/`commit`/`cancel` helper・`parity_gesture_scenario`）・`scenario/fake.rs`（`FakeGuiTest`）・`scenario/visual.rs`（`VisualGuiTest`）。GPUI adapter: `crates/ruviform-ui-gpui/src/gpui_driver/scenario.rs`（`GpuiGuiTest`、`gpui-test` feature 限定）+ `gpui_driver/observe.rs`（`GpuiObserver`） |
| 実装 PR | ruviform #477。Consumer 採用: AudioLens PR #188（merged、#464 Step C） |
| 検証 lane | `mise run test-ui-scenario-contract` / `-fake` / `-gpui` |

### G. Focus / Keyboard / Layout 補遷契約（#451・#478・#479）

| 領域 | 正本 |
| --- | --- |
| #451 Tabs pointer 切替 + descriptor 更新の ViewSpec 契約 | [designs/issue-451-tabs-pointer-descriptor.md](issue-451-tabs-pointer-descriptor.md) + spec/plan/task/test-spec 同名鎖。実装 PR #457 |
| #451 FU-1 Tabs キーボード切替 | [designs/issue-451-keyboard-tab-nav.md](issue-451-keyboard-tab-nav.md) + 鎖。実装 PR #460 |
| #451 FU-1b Tab/Shift-Tab フォーカス巡回配線 | [designs/issue-451-focus-next-wiring.md](issue-451-focus-next-wiring.md) + 鎖。実装 PR #484 |
| #478 Fader/Knob 縦寸法契約 | [designs/issue-478-fader-knob-height-contract.md](issue-478-fader-knob-height-contract.md) + [adr/issue-478-…](../adr/issue-478-fader-knob-vertical-size-contract.md) + spec/test-spec/plan/task/IR 鎖。docs PR #485、実装 PR #491 |
| #479 値制御の共通 keyboard 規約 | [designs/issue-479-keyboard-value-controls.md](issue-479-keyboard-value-controls.md)（§2 が key 規約の正本）+ spec/plan/task/test-spec 鎖。実装 PR #486 |

これらは driver-neutral 契約の**補遷領域**（focus traversal・
keyboard・layout sizing）であり、GPUI driver 側実装を含むが契約の
意味論は `ruviform-ui` / scenario contract に接続する。

### H. Native Surface Slot / Plugin Editor Session（#388・#389 — Open・部分のみ）

契約層の型（`native_slot.rs`）と #387 の NativeSlot glue は merge
済みだが、**Native Dock の実機採用 Gate は未達**である。

- #388（OPEN）: GPUI Panel/Tab layout 内 VST3 editor 埋め込みの実機
  検証は PR #438 で部分達成（Dock PASS 不成立）。test-spec:
  [issue-388-389-native-dock-contract.md](../test-specs/issue-388-389-native-dock-contract.md)
  / [issue-388-native-dock-layout-poc.md](../test-specs/issue-388-native-dock-layout-poc.md)。
- #389（OPEN）: Plugin Editor Session / L0/L1 adapter。PR #427 で
  refused open の instance 欠陥修正・L0 pass-through 廃止。
  test-spec: [issue-389-vst3-l0-editor-attach.md](../test-specs/issue-389-vst3-l0-editor-attach.md)。
- `PluginEditorSessionId`/`ParentSlotLease` の共通契約骨格は
  [issue-384 design](issue-384-driver-neutral-gui-contract.md)
  「PluginEditorSession 契約」節が正本（実装は #389 側）。

## Presentation Driver 抽象と層区別

契約の層構造（正本: #384 spec/design/ADR）:

```text
Consumer ViewSpec 構築（typed builder / 構造化 Rust 型）
  → ResolvedTheme（#385、ruviform-ui-theme）+ Component/View spec（#387、ruviform-ui::view/behavior）
  → Driver へ mount（Window/Dock 契約、#386、ruviform-ui::driver/dock）
  → Driver が input を正規化 → Headless Behavior（Gesture/Receipt/generation）
  → 共通契約経由で Consumer Command/Query/Binding へ
```

- **Presentation Driver** = `WindowDriver` trait + `DriverEvent`/
  `WindowLifecycle`/`WindowCapabilities`（`ruviform-ui::driver`）。
  Driver は OS 入力を正規化して共通契約へ渡し、GPUI 型・OS handle・
  DOM を共通 API に漏らさない。
- **GPUI の位置づけ** = 標準 Presentation Driver の**仮決定**
  （#384 ADR status: proposed）。実装は `crates/ruviform-ui-gpui`
  （feature `gpui`/`gpui-test`、default off で headless `cargo
  check --workspace` を維持）。**最終採否は未確定**（#262 verdict
  pending → #419 後継 ADR）。
- **Fake Driver** = `ruviform-ui-test` の in-memory 実装
  （`fake_driver.rs`/`fake.rs`/`lifecycle.rs`）。契約 fixture と
  scenario contract の semantic oracle を担う。
- **Scenario 層** = `ruviform-ui-test::scenario` の
  `SemanticGuiTest` trait。Fake（`FakeGuiTest`）と GPUI
  （`GpuiGuiTest`、`gpui-test` feature）と将来の実 driver が同一
  scenario 関数を実行する薄い契約。実 OS / pixel / IME 等の能力は
  `ScenarioCapability` で区別し、Fake pass を実機証拠へ昇格しない。
- **実 driver 層** = `ruviform-ui-gpui`（標準候補）と
  `ruviform-ui-webview`（legacy。下記「歴史的参照」）。実 OS window・
  Native Dock・IME・a11y の検証は `not_run`/`unsupported` として
  owner Issue（#313/#243/#314/#262/#388/#389）へ委譲済み。

## Issue #257 受入項目の証拠対応表

Issue #257 本文「受入条件」3 項目 + 2026-11-07 DoD を、merge 済み
PR / 文書 / 型へ対応付ける。**検証列は文書・fixture レベルの
確認であり、実 OS 実機 Pass を意味しない**（実機 Gate は各 owner
Issue に残る）。

| # | 受入項目（Issue #257） | 証拠（PR / 文書 / 型） | 状態 |
| --- | --- | --- | --- |
| AC1 | 型・一意性/owner・generation、View build/update/mount/unmount、Focus/IME/Action、Unsupported の Rust シグネチャと state transition を #386/#387 で決定 | 型: `driver.rs`（`WindowDriver`/`WindowLifecycle`/`WindowCapabilities`/`UiError::Unsupported`）、`window.rs`（`WindowId`）、`dock.rs`（`ViewInstanceId`/`DockWorkspace` 2-phase）、`view.rs`（`ComponentKey`/`ViewSpec`/`ViewHost`/`AttachEpoch`/`generation`/`ViewError::StaleGeneration`）、`native_slot.rs`（`SurfaceSlotId`/`NativeSurfaceSlot`）。文書: #386/#387 spec・design・test-spec（上表 C/D）。PR: #394・#406・#420・#422。Focus/IME/keyboard 補遷: #451/#478/#479 文書鎖 + PR #457/#460/#484/#485/#486/#491 | **契約レベルで決定・merge 済み**。IME/実 OS focus の実機検証は未実施（owner: #262/#313 系） |
| AC2 | GPUI なしの common API / 2 Consumer minimal Slice の型検査および Fake/GPUI 同一 semantic oracle | 型検査: `check-ui-driver-boundary` guard（`driver_boundary.rs`/`gui_boundary_guard.rs`、PR #396/#425）— `ruviform-ui`/`ruviform-ui-theme` の公開 signature に `gpui`/Toolkit/OS handle を出さないことを機械検査。semantic oracle: `SemanticGuiTest` + `parity_gesture_scenario`（`scenario.rs`）を `FakeGuiTest`/`GpuiGuiTest` が共有実行（PR #477、test-spec issue-464）。2-consumer: AudioLens は #464 採用済み（AL PR #188 merged）。Synvelis 側 slice は **SV#112（Open）本体** | **契約 + Fake/GPUI oracle は達成**。SV consumer slice は未完了（SV#112 が owner） |
| AC3 | 元 Issue の WebView/DOM/CustomEvent 決定は移行前の歴史的実装に限定し、本節に反する新規 UI 要求の正本ではない | 文書: #384 後継 ADR が旧 2 ADR（`ui-architecture-webview-hybrid.md`・`ui-tech-selection-poc.md`）を supersede（双方 `superseded`/`supersedes` リンク済み）。#465 P0/P1 で legacy WebView 文書の歴史化（PR #475、spec/ADR `issue-465-*`）。#455 は WebView 新規 kind/intent を不採用で Closed。**本書「歴史的参照」節が区別の正本** | **文書契約として達成**（本 PR が最終ピース） |

補足: Issue #257 の 2026-11-07 DoD 3 項目は上記 AC1〜AC3 と同一
内容であり、同じ証拠でカバーされる。

## 歴史的経緯 / 廃止済み参照の区別

以下は**現行契約の正本ではない**。Issue #257 本文・旧 ADR・旧
Issue 記述に残る WebView 時代の前提は、2026-11-07 v2 / 2026-11-08
補足 / 2026-11-09 方針により「移行前の歴史」として保存される。

| 参照 | 状態 | 正本 / owner |
| --- | --- | --- |
| `docs/adr/ui-architecture-webview-hybrid.md`（WebView-first hybrid） | **superseded**（#384 後継 ADR が supersede） | [driver-neutral-gui ADR](../adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md) |
| `docs/adr/ui-tech-selection-poc.md`（wry/tao・FW 選定 PoC） | **superseded**（同上） | 同上 |
| Issue #257 本文の旧節（§12 Reference Skin/Theme、§13 Framework-neutral Web UI / wry+tao・IPC 三経路・Canvas2D/WebGL2、RF-GT-01〜06 の Playwright/DOM/CustomEvent 前提、Synvelis SolidJS 境界節） | **歴史的経緯**。新 GUI の必須依存・完了条件ではない | Issue #257 本文冒頭の優先順位節 + 本書 |
| `crates/ruviform-ui-webview`（wry/tao adapter） | **legacy/preview**。新規 ViewSpec 描画・同一 Intent 追加の義務なし。物理削除は #465 Owner、Gate は #419 採否 + #294 非回帰 | [specs/issue-465-…](../specs/issue-465-legacy-webview-p0p1.md) / [adr/issue-465-…](../adr/issue-465-legacy-webview-retirement-status.md) / discovery inventory `issue-465-legacy-webview-inventory.md` |
| `ruviform-ui` 内の旧 WebView 残置 API（`HostHandle::eval`/`WindowHandle::eval`/`HostEvent::Ready{url}`/`HostEvent::Message(String)`/`WindowConfig{url,devtools}`/`RenderTier`/`NativeSurfaceHost`/`bridge::dispatch_script`/`BRIDGE_JS`） | **残置（legacy 経路）**。共通契約の WebView 実装として段階適合中。削除/移動は issue-386 design「移行段階」Phase 2 の別明示判断 | [issue-386 design 移行表](issue-386-driver-neutral-gui-host.md) |
| Issue #455（WebView への新規 kind/intent 実装） | **Closed（not planned = 不採用）** | GitHub Issue #455 |
| Issue #258〜#263・#310〜#312（旧 WebView/TS 系の実装 Issue） | **Closed 実装履歴**。GPUI 実装 Issue を再オープンしない | Issue #257 本文 2026-11-07 節 |
| `RenderTier`/`NativeSurfaceHost` 等の「旧 WebView を現行 Parity Matrix の必須 Driver として数える」解釈 | **廃止**。旧 WebView を Parity Matrix の必須 driver として数えない | Issue #257 本文 2026-11-08 補足 |

## 未確定事項（正本が決定権を持つもの）

- **GPUI 最終採用判定**: #262（Open、verdict pending）→ #419
  （Open、後継 ADR 最終採用）。#388/#389 の Native Dock 実機証拠が
  入力。**本書を GPUI 採用確定の根拠に使用しないこと**。
- **Native Dock PASS**: Floating/Generic Editor は代替不可。
  L1（cross-process）は現行 `HostError::Unsupported` が出発点で
  #389 が診断。
- **Synvelis 2-consumer slice**: SV#112（Open）が受入 owner。
  本書は参照先の索引に留め、受入を代行しない。AudioLens 側は
  #464 採用済み（AL PR #188）。
- **旧 WebView 物理削除**: #465 が Owner、#419 + #294 確認まで
  破壊的削除しない。

## Data and Control Flow

本書自体は索引であり、実行時の data/control flow は持たない。
読者の辿り方:

```text
Issue #257 受入項目 / 契約領域
  → 本書（領域別索引 + 証拠対応表）
  → 参照先の正本文書（spec/design/ADR/test-spec）
  → 実装型（crates/ruviform-ui*/src/）・fixture（driver-lab/scenario）
  → 検証 lane（mise run …）
```

契約の実行時 flow（ViewSpec → ResolvedTheme → Driver mount →
正規化 input → Headless Behavior → Command/Binding、NativeSlot は
capability gate 経由）は #384 design「Data and Control Flow」が
正本。

## Non-Functional Constraints

- **再定義禁止**: 本書は契約を再定義しない。各契約の意味論・
  不変条件は参照先正本が所有し、本書は所在と証拠のみを記録する。
- **証拠の誠実性**: merge 済み PR・実在する型・実行済み lane のみを
  証拠とし、未実施は `NotRun`/`unsupported`/`blocked` として記す。
  headless/Fake の green を実 OS/実 Plugin の Pass とみなさない。
- **陳腐化の隔離**: 歴史的参照は「歴史的経緯 / 廃止」節に集約し、
  現行契約と混在させない。
- **採否の非確定性**: GPUI 最終採用は未確定であることを本文・
  front matter 双方で明示する。

## Risks and Trade-offs

- **索引の陳腐化**: 契約は進化する。本書は参照型のため各正本の
  更新に自動追従しない — 参照先の status 変更時は本書の「状態」
  記述を見直す責務を残す（本書は 2026-11-09 時点の snapshot）。
- **証拠の誤読**: 「契約 merge 済み」と「実機 Pass」を混同する
  リスク → 証拠対応表に状態列を設け、実機 Gate の owner Issue を
  明記した。
- **リンク切れ**: 参照は doc-status の link/relation 検査で検証する
  （Verification Notes）。

## Deferred Design Concerns

- GPUI 採否の最終判定と後継 ADR への逆反映 → #262/#419（本書は
  決定しない）。
- Native Dock / Plugin Session の実機契約 → #388/#389。
- Synvelis consumer slice の受入 → SV#112。
- 旧 WebView 残置 API の削除/移動判断 → issue-386 design Phase 2 +
  #465/#419 gate。
- `RenderTier` 系の後継表現 → #387 契約 + driver capability で
  代替済みかは旧経路縮退判断時に再評価。

## Verification Notes

- [ ] `docs/designs/issue-257-driver-neutral-ui-contract.md` が存在し、
  領域別索引・証拠対応表・歴史的参照区別・未確定事項を含む。
- [ ] 証拠対応表の各行が実在する merge 済み PR / 文書 / 型ファイルを
  指す（本書作成時に `git log`・`gh issue view`・ソース走査で確認）。
- [ ] 参照リンク・front matter relations が doc-status 系 lint を
  通る（broken relation なし）。
- [ ] GPUI 採否未確定（#262 pending）の明記が本文に存在する。
- [ ] 既存他 Issue 文書に diff がない（本 PR の変更は新規
  `issue-257-*` 文書チェーン + 生成 index のみ）。
- [ ] `mise run fmt-check` が pass（docs のみ変更のためコード
  lane への影響なし）。
