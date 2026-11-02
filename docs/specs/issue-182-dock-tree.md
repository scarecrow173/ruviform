---
id: "SPEC-BkHWcVnZOw9qbxHXdfNUzH"
type: "spec"
status: "proposed"
title: "Issue #182 Dock Tree / View Move / Layout Restore — reusable multi-window docking foundation"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/182"
    - "https://github.com/scarecrow173/synvelis/issues/2"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-181-multi-window.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-182-dock-tree-inventory.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/specs/issue-181-multi-window.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-182-dock-tree-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/181"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 182
---

# Issue #182 Dock Tree / View Move / Layout Restore — reusable multi-window docking foundation

## What Should Be Built

**DAW 固有の View 種別や Project model を持たない、複数 Window にまたがる再利用可能な Docking レイアウト・View 移動・レイアウト復元の基盤**を `ruviform-ui` に実装する。1 ネイティブ Window につき 0 または 1 の Dock Tree（`Split(axis,ratio,children)` + `Tabs(view_instance_ids,active)` の再帰木）を対応付け、全 Window を通じて論理 View Instance ID の所有先は高々 1 箇所とする。View 移動は「移動先準備 → 所有権切替 → 失敗時は元 Dock 復元」の two-phase 契約とし、Window 幾何・モニター・Dock Tree を版付きスナップショットで保存/復元する。

1. **Dock Tree モデル**: `Split(axis, ratio, children≥2)` と `Tabs(views≥1, active)` の再帰ノード。空 Tabs・単一子 Split・無効分割比・重複所属は正規化または拒否する。
2. **論理 View**: `ViewInstanceId` は consumer 採番の論理識別子で、OS Window/DOM 実体から独立。Window 間移動は view state の保存/再構築で行い、DOM オブジェクトの Window 間移送は保証しない。
3. **View 移動契約**: 移動先 Window/WebView が準備できてから所有権を切り替える two-phase（`begin`/`commit`/`abort`）。移行中の View は編集不可（二重編集禁止）。失敗は元 Dock へロールバック。
4. **永続化**: Window 幾何（物理座標・サイズ）・モニター signature・Dock Tree を版付き schema で保存/復元。検証失敗・スキーマ不一致・壊れたデータは安全なデフォルトへフォールバック。画面外座標・消失モニターは可視領域へ補正。ディスプレイ ID の永続性を仮定しない。
5. **操作層の分離**: splitter/tab 並べ替え/drag preview/drop target の操作語彙はアプリ固有 UI から分離した純粋モデル API とし、drag 機構に依存しない明示的メニュー移動経路を同一 API で提供する。
6. **Window Close 時の View 扱い**: consumer が明示ポリシーで選ぶ（Detach/Relocate/Release）。dock 層はプロジェクトを黙って閉じない。

## Why Now

- Synvelis が Arrangement/Mixer/Inspector 等のタブ化・分割・切り離し・複数モニター再配置を初期必須要件とし（Synvelis #2・統合設計書）、#182 は #181（merged・WindowId/`run_multi`/`CloseDecision`）直上の Docking 基盤 Issue。
- Synvelis v0.6 Window Close Contract（#21）・v0.7.1 Window-Job Lifetime（#22）と Ruviform #192/#193（Migration M7 gate）で Window Close・View 移動・Workspace 保存の分担が合意済み。Ruviform 側は「Generic Dock Layout ID/Tree/Move/Restore の不変条件」が担当範囲。
- #181 で Window 識別・ライフサイクル・IPC routing が確定した今、Dock Tree はその上の自然な次層であり、Synvelis 側の Workspace 統合をこれ以上遅らせない。

## Users and Value

- **Synvelis**: 制作 View のタブ/分割/Detach→Reattach・複数モニター配置・Window Close 時の View 退避・レイアウト保存/復元の土台。
- **audio-lens 等の単一 Window consumer**: 既存経路は不変（docking は opt-in の additive 契約）。
- **Ruviform 自身**: M7 gate（Dock Tree 実装）の受入証拠。

## Scope

### In Scope

- `ruviform-ui::dock` 新モジュール: `ViewInstanceId`・`DockNode`/`DockTree`・`DockPath`・`DockWorkspace`・move two-phase 契約・`DockEvent`・`DockError`・永続化 schema（`LayoutSnapshot` 等）・window-close ポリシー語彙。
- `WindowConfig` への `position` additive フィールドと `ruviform-ui-webview` `create_entry` での `with_position` 反映（復元幾何の適用経路）。
- headless/fake-host テスト（Split/Tabs/move/失敗ロールバック/重複 View 拒否/不正レイアウト/復元/close ポリシー/画面外・モニター消失補正）。
- headless demo example（detach→reattach・論理 View ID/状態保持の実演）。
- doc チェーン（discovery/spec/ADR/design/plan/test-spec/tasks/impl record）と索引・README の整合。
- 実機検証の実施範囲と未検証範囲の正直な記録（capability honesty）。

### Out of Scope

- Synvelis 固有: View factory・View State の中身・Project/Selection/Link Group・DAW レイアウトプリセット・保存方針・Command Router。
- 全 UI コンポーネントへの Docking 抽象の強制（Issue 本文で明示的に除外）。
- OS ネイティブの cross-webview drag&drop 実装（wry は提供しない — drag メタファーは各 WebView 内、commit は本 move API）。
- Window 生成後の位置変更 API（`WindowHandle::set_position`）— 本 Issue は生成時 `position` のみ。実行時移動は `Moved` イベントで consumer が追跡する既存契約。
- View State の具体的シリアライズ形式（consumer 所有 — Ruviform は不透明な所有権/イベント境界のみ提供）。
- `rvf.ipc.v1` wire 形式の変更。
- WebView レンダラクラッシュ検出・自動復帰、メニュー/トレイ等シェル機能（#156/#181 と同じ境界）。

## Requirements

- R1: `ViewInstanceId` — 非ゼロ・consumer 採番の安定論理識別子（Serialize/Deserialize・Display）。Window 間移動・Detach/Reattach で同一性を維持する。
- R2: `DockNode` は `Split{axis,ratio,children}`（`axis` = Horizontal/Vertical、`ratio` ∈ (0,1) 有限・children ≥ 2、ratio は先頭 child の軸方向占有率で残りを兄弟が等分）と `Tabs{views,active}`（views ≥ 1・active は有効 index）の再帰木。空 Tabs は正規化除去・単一子 Split は子に畳み込み・active 範囲外はクランプ（正規化）。無効 ratio は拒否。同一 tree 内の重複 View は拒否。
- R3: `DockWorkspace` が `WindowId → Option<DockTree>` を管理し、1 Window ≤1 tree・全 Window 横断で View 所有先は高々 1 箇所。未所属 View は `detached` プール（detach・window-close Detach ポリシー・未 dock View の保留）。
- R4: Window 登録/除去 — `register_window`（冪等・空 Window 許容）と `remove_window(window, policy)`。`policy` は `DetachViews`/`RelocateTo(w)`/`ReleaseViews` の明示引数 — 既定なし。`RelocateTo` は View をターゲット root へ tab として集約（レイアウト構造は保持しない）。docked View の扱いは常に consumer 選択であり、dock 層は Job cancel・Project close を暗黙発火しない。
- R5: 操作語彙 — `set_ratio`（splitter commit）・`move_tab`（並べ替え）・`activate_view`（active tab 選択）・`attach_view`/`detach_view`/`remove_view`・`move_view`（one-shot）・`begin_move`/`commit_move`/`abort_move`（two-phase）。`DropZone{Left,Right,Top,Bottom,Tab}` + `DockTarget`（`Window(w)`/`Zone{window,anchor,zone}`）が drop/メニュー移動共通の配置語彙。
- R6: two-phase move — `begin_move(view,target)` は View を transfer 状態にして構造を変えず、`commit_move(token)` で原子的に detach+attach+正規化し `MoveCommitted` を発行、`abort_move(token)` は source 不変で解除。commit 時に移動先 Window が消失していれば `MoveAborted` 発行 + `Err(TargetUnavailable)`（View は source に残る = rollback）。transfer 中の View への全変更 op は `ViewBusy` で拒否（二重編集禁止）。メニュー移動・in-window drop は `move_view`（begin+commit の fused one-shot）を使う — drag 機構非依存の明示経路。
- R7: `DockEvent` — `TreeChanged`/`MovePrepared`/`MoveCommitted{view,from,to}`/`MoveAborted`/`WindowRemoved{window,views}` を `drain_events` で排出。consumer は `MoveCommitted` で source 側 view state を保存し dest で再構築する（state は consumer 所有・DOM 移送は保証しない）。
- R8: `LayoutSnapshot` — `schema` version + window 配列（復元順）+ `detached` View 群。Window 要素は物理座標 `{x,y,width,height}`・`monitor` signature（name+物理 size+scale — 永続 display ID ではない）・`tree`。`WindowId`/`ViewInstanceId` 以外の session token は載せない。
- R9: 復元 — `restore(monitors)` は schema 検証・tree 正規化+検証・全 Window+detached 横断の View 重複拒否を行い、失敗時は `Err`（`restore_or_default` は安全デフォルトを返す）。画面外 rect は center が現モニター内に収まるよう補正し、保存モニター signature 不一致/消失時は可視モニターへフォールバック配置。
- R10: `WindowConfig.position: Option<(i32,i32)>`（物理ピクセル・仮想デスクトップ座標）を additive 追加し、`run_multi`/`run` の生成時に適用する。既存 builder リテラルは `..Default::default()` 必須となる点を ADR に記録。
- R11: drag/drop 制約 — cross-webview ネイティブ drag は backend 保証外（discovery 調査済み）。drag preview/drop target の視覚層は frontend の責務、commit は本 move API。メニュー移動経路は `move_view` で drag 無しに同等操作が可能であることを設計・テストで示す。
- R12: `ruviform-ui` は DAW View 型・Project model・Synvelis 語彙を持たない。View State・意味論は consumer 所有（`MoveCommitted` で handoff）。依存方向（Primitives のみ）・非 RT 契約・`#[non_exhaustive]` enum・`unsafe` 不使用を維持する。
- R13: 単一 Window/既存 multi-window 経路不変 — `host::run`・`run_multi` の既存イベント意味・`UiRuntime` の挙動・`demo_app`/`demo_cli`/`demo_multi` 非回帰。
- R14: 消費者影響分類（synvelis/audio-lens）と GUI 実機未検証範囲を記録する。

## Acceptance Criteria

- [ ] AC-1: `docs/discovery/issue-182-dock-tree-inventory.md` が存在し、#181 資産の棚卸し・10 項目のギャップ・drag/drop プラットフォーム制約の調査結果を記録済み。
- [ ] AC-2: Dock Tree/move/永続化/close ポリシーの契約が ADR + design doc で確定し、move の two-phase・DOM 移送非保証・view state は consumer 所有であることが明示される。
- [ ] AC-3: `ruviform-ui::dock` にモデル（ViewInstanceId/DockNode/DockTree/DockPath）・`DockWorkspace`（window 管理・op 語彙・two-phase move・close ポリシー・events）・`LayoutSnapshot`（版付き schema・validate・default fallback・幾何補正）が実装される。
- [ ] AC-4: headless テストが Split/Tabs 正規化・無効拒否・move commit/abort・commit 時移動先消失 rollback・重複 View 拒否・transfer 中 ViewBusy・不正レイアウト/スキーマ不一致/壊れたスナップショットの復元拒否→デフォルト・画面外/モニター消失補正・window close 3 ポリシーを検証する。
- [ ] AC-5: 複数 Window での detach→reattach が論理 View ID・（consumer 側 view state 相当の）状態を保持することを headless demo で示す。実機 GUI 未検証は capability 記録で明示。
- [ ] AC-6: `WindowConfig.position` が adapter 生成経路に反映され、`check-ui-webview`/`lint-ui-webview`/`test-ui-webview` が green。
- [ ] AC-7: `mise run fmt-check`/`check`/`lint`/`test` green・既存テスト非回帰・menu-move（`move_view`）経路が drag 非依存で動くことをテストで示す。
- [ ] AC-8: DAW/Project 語彙を含まない API であること・消費者影響分類の記録・doc チェーンと index 整合。

## Deferred Design Concerns

- `WindowHandle::set_position`/`set_geometry` 実行時移動 API（本 Issue は生成時 position のみ — consumer が `Moved` で追跡する既存契約で足りるかの実地確認待ち）。
- Floating/ピン留め View、レイアウトプリセット語彙（consumer 所有域として維持するかの判定は Synvelis 統合後）。
- OS ネイティブ cross-window drag&drop（adapter 拡張・platform helper — wry 不在のため別 Issue での採否対象）。
- View State の受け渡しを型付けする共通 port（`MoveCommitted` 手続きの SDK 化）。
- モニター DPI 変化時の tree 自動再配置ポリシー（本契約は幾何 rect 補正のみ — 比率再計算は consumer 方針）。
- 空 Window の自動閉鎖方針（`run_multi` は window 全滅でも exit しない — dock 層も同じく consumer 判断を残す）。

## Sources and Relations

- Issue: ruviform#182（要件正本・M7 gate）、ruviform#181（前提 multi-window・merged）、ruviform#156（GUI ホスト基盤）、ruviform#192/#193（migration gate・受入枠）、ruviform#219（consumer 影響分類運用）。
- Synvelis: #2（統合設計・Workspace/Docking 設計書）、#21/#22（v0.6/v0.7.1 合意 — View 移動と Project Job owner の分離）、#28（受入先）。
- 棚卸し: `docs/discovery/issue-182-dock-tree-inventory.md`（derives-from 元）。
