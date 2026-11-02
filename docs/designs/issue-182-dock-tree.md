---
id: "DESIGN-iYLA7E6C3gMZXNqMyIqMw5"
type: "design"
status: "approved"
title: "Issue #182 Dock Tree / View Move / Layout Restore — reusable multi-window docking foundation"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/182"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui/src/dock.rs"
      - type: "file"
        path: "crates/ruviform-ui/tests/dock.rs"
      - type: "file"
        path: "crates/ruviform-ui/examples/demo_dock.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/window.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui/README.md"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/session.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/host.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/README.md"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/specs/issue-182-dock-tree.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/adr/plugin-gui-embedding.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-182-dock-tree.md"
    - "docs/adr/issue-182-dock-tree-placement-move-contract-and-persistence.md"
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
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 182
---

# Issue #182 Dock Tree / View Move / Layout Restore — reusable multi-window docking foundation

## Context

Issue #181 で導入した multi-window 契約（`WindowId`/`WindowHandle`/`run_multi`/`MonitorInfo`/`CloseDecision`）の上に、Issue #182 は再利用可能な Docking レイアウト・View 移動・レイアウト復元の基盤を置く。ADR `issue-182-dock-tree-placement-move-contract-and-persistence`（accepted）で以下が確定済み: 配置は `ruviform-ui::dock`（A1）、move は「View を source に残した transfer lock + commit 時原子切替」（B1）、所有不変条件は `WindowId → tree` map 単一真実 + 走査導出（C1）。

## Scope

### In Scope

- `ruviform-ui::dock`（新規モジュール・1 ファイル `src/dock.rs`）:
  - モデル: `ViewInstanceId`・`Axis`・`DockNode`・`DockTree`・`DockPath`・`ViewLocation`・`DockError`。
  - workspace: `DockWorkspace`・`DockTarget`・`DropZone`・`MoveToken`・`MoveReport`・`WindowCloseDisposition`・`DockEvent`。
  - 永続化: `LayoutSnapshot`・`WindowSnapshot`・`MonitorHint`・`WindowGeometry`・`RestoredLayout`・`WindowPlacement`・`LAYOUT_SCHEMA_VERSION`。
- `WindowConfig.position: Option<(i32,i32)>`（物理 px）追加と adapter 生成経路への反映（`session.rs` `create_entry` + `host.rs` `run`）。
- `crates/ruviform-ui/tests/dock.rs`（headless 契約テスト）+ `examples/demo_dock.rs`（fake-host デモ）。
- doc チェーン・索引・README の整合。

### Out of Scope

- Synvelis 固有（View factory・View State 内容・Project/Selection・プリセット・Command Router）。
- `WindowHandle::set_position` 等の実行時移動 API（ADR「Deferred」）。
- OS ネイティブ cross-window drag&drop、シェル機能、`rvf.ipc.v1` 変更。
- 全 UI コンポーネントへの Docking 抽象の強制。

## Components and Boundaries

### `ruviform-ui::dock`（新規モジュール）

#### Identity / Model

```rust
/// consumer 採番の論理 View 識別子。非ゼロ・OS Window/DOM から独立。
/// WindowId と異なり consumer が発行し、移動・detach/reattach・永続化で同一性を維持する。
pub struct ViewInstanceId(NonZeroU64);  // Serialize/Deserialize + Display "v{n}"

#[non_exhaustive]
pub enum Axis { Horizontal, Vertical }  // Horizontal = 子を左右に、Vertical = 上下に

/// 再帰 Dock ノード。#[non_exhaustive] で外部直接構築を禁止し、
/// コンストラクタが不変条件を拒否で守る。
#[non_exhaustive]
pub enum DockNode {
    /// axis 方向に children（≥2）を並べる。ratio は children[0] の占有率
    /// （(0,1) 有限）、残り 1-ratio を他の子が等分。
    Split { axis: Axis, ratio: f64, children: Vec<DockNode> },
    /// タブ葉。views ≥ 1、active < views.len()。
    Tabs { views: Vec<ViewInstanceId>, active: usize },
}

/// 正規化済み・検証済みの木。空 window は tree なし（Option）。
pub struct DockTree { /* root: DockNode（不変条件保持） */ }

/// ルートからの child index 列。構造変更で無効化する（doc 明記）。
pub struct DockPath(Vec<usize>);

/// locate の返り値 — 葉のパスと葉内タブ index。
pub struct ViewLocation { pub leaf: DockPath, pub tab: usize }
```

- `DockNode::tabs(views, active) -> Result<Self, DockError>` — 空 views は `EmptyTabs`。`active` は len へクランプ。
- `DockNode::split(axis, ratio, children) -> Result<Self, DockError>` — children < 2 は `InvalidChildren`、ratio が非有限または (0,1) 外は `InvalidRatio`。
- `DockTree::new(root)` は `normalize()` + `validate()` を通す（新規構築はコンストラクタ済みのため normalize は自明に no-op）。`normalize()` の規約: 空 Tabs を除去・<2 子 Split を畳み込み（0 子は除去・1 子は子に置換）・active をクランプ — 再帰ボトムアップ。`validate()`: ratio 有限 (0,1)、全葉 views≥1、全 split children≥2、tree 内 View 重複なし。
- `DockTree` 付随 API: `root()`/`views()`（全 View 列挙）/`locate(view) -> Option<ViewLocation>`/`node(path)`。

#### Workspace（操作層）

```rust
/// 全 Window の dock 状態を管理する唯一の真実層。
/// host（WindowHandle）は呼ばない純粋モデル — 生成・Ready・破棄の
/// 通知は consumer が register/remove で注入する。
pub struct DockWorkspace {
    windows: BTreeMap<WindowId, Option<DockTree>>,  // 登録済み window（None = 空 window）
    detached: BTreeSet<ViewInstanceId>,             // 未所属 View pool
    pending: HashMap<ViewInstanceId, PendingMove>,  // transfer 中
    next_move: u64,
    events: VecDeque<DockEvent>,
}
```

- **Window 管理**: `register_window(w)`（冪等・空 `None` で登録）、`remove_window(w, WindowCloseDisposition) -> Result<Vec<ViewInstanceId>, DockError>`、`windows()`、`tree(w)`、`is_registered(w)`。
- **View 管理**: `attach_tree(w, tree)`（登録済み w・tree 検証・全 View 無所属必須）、`attach_view(view, target)`（未所属 View の配置）、`detach_view(view)`（detached pool へ）、`remove_view(view)`（workspace から除去）、`owner_of(view) -> Option<WindowId>`、`is_detached(view)`、`detached_views()`、`all_views()`。
- **op 語彙**: `set_ratio(w, path, ratio)`・`move_tab(view, index)`・`activate_view(view)`。`DockPath` は構造変更で無効化 — consumer は `locate` で再取得する。
- **配置語彙**: `DockTarget::{ Window(WindowId), Zone{window, anchor: ViewInstanceId, zone: DropZone} }`、`DropZone::{Left,Right,Top,Bottom,Tab}`。`Zone` は anchor を含む葉への相対配置: `Tab` は葉へ追加+活性化、`Left/Right` は Horizontal split、`Top/Bottom` は Vertical split で葉を 2 子 split に置換（新葉が先/後は方向に従う、ratio 0.5）。`Window(w)` は空なら新 Tabs 木、root が Tabs なら tab 追加、root が Split なら最後尾に新 Tabs 子を挿入。

#### Move 契約（two-phase）

```rust
pub struct MoveToken { /* id: u64, view — private */ }
pub struct MoveReport { pub view: ViewInstanceId, pub from: Option<WindowId>, pub to: WindowId }
```

- `begin_move(view, target) -> Result<MoveToken, DockError>` — View の存在（owned or detached）と target 妥当性（window 登録済み・anchor がその window 内）を検証し `pending` に登録。**構造は変えない**。以後その View への変更 op は `ViewBusy`。
- `commit_move(token) -> Result<MoveReport, DockError>` — target を再検証（消失していれば pending 解除 + `MoveAborted` + `Err(TargetUnavailable)`）。detach → attach → normalize → `MoveCommitted`。
- `abort_move(token) -> Result<(), DockError>` — pending 解除 + `MoveAborted`。構造不変。
- `move_view(view, target) -> Result<MoveReport, DockError>` — begin+commit の fused one-shot。**menu-move・in-window drop の正規経路**（drag 機構を使わない）。
- `is_transferring(view)` / pending 中の `remove_window` は対象への pending を `MoveAborted` で自動解除してから policy を適用。

```rust
#[non_exhaustive]
pub enum DockEvent {
    TreeChanged { window: WindowId },
    MovePrepared { view: ViewInstanceId },
    MoveCommitted { view: ViewInstanceId, from: Option<WindowId>, to: WindowId },
    MoveAborted { view: ViewInstanceId },
    WindowRemoved { window: WindowId, views: Vec<ViewInstanceId> },
}
// drain_events() で排出。TreeChanged = 当該 webview の再描画、
// MoveCommitted = view state の source→dest handoff 点。
```

```rust
#[non_exhaustive]
pub enum WindowCloseDisposition {
    /// View を detached pool に保持（後で再 dock 可能）。
    DetachViews,
    /// 別 window の root へ View を tab 集約（レイアウト構造は保持しない）。
    RelocateTo { target: WindowId },
    /// View を除去（consumer が state 保存済み — 終了経路）。
    ReleaseViews,
}
```

#### `DockError`

`#[non_exhaustive]` enum: `UnknownWindow{window}`・`UnknownView{view}`・`DuplicateView{view,owner}`・`EmptyTabs`・`InvalidChildren{children}`・`InvalidRatio{ratio}`・`BadPath`・`InvalidTarget{reason}`・`ViewBusy{view}`・`NotInTransfer`・`TargetUnavailable{window}`・`InvalidSnapshot{reason}`・`UnsupportedSchema{found}`。`Display`+`Error` impl（`UiError` 流儀）。

#### Persistence

```rust
pub const LAYOUT_SCHEMA_VERSION: u32 = 1;

pub struct LayoutSnapshot { schema, windows: Vec<WindowSnapshot>, detached: Vec<u64> }
pub struct WindowSnapshot { x, y, width, height /*物理px*/, monitor: Option<MonitorHint>, tree: Option<DockNode> }
pub struct MonitorHint { name: Option<String>, width, height, scale_factor }  // signature のみ
pub struct WindowGeometry { x, y, width, height, monitor: Option<MonitorHint> }
pub struct RestoredLayout { placements: Vec<WindowPlacement>, detached: Vec<ViewInstanceId> }
pub struct WindowPlacement { x, y, width, height, monitor: Option<usize>, tree: Option<DockTree> }
```

- `LayoutSnapshot::capture(ws, geometry: &[(WindowId, WindowGeometry)])` — `windows` は geometry 配列順（復元順 = consumer が再作成する順）。
- `to_json`/`from_json`（serde_json、`schema` 必須）。
- `restore(monitors) -> Result<RestoredLayout, DockError>`: schema ≠1 → `UnsupportedSchema`；各 tree clone → `normalize`+`validate`；全 window+detached 横断の View 重複・w/h==0 → `InvalidSnapshot`。幾何補正: rect の center がいずれかの `MonitorInfo` 内 → 採用（`monitor=Some(i)`）；外なら signature 一致モニター → なければ先頭モニターへ中央配置（サイズはモニターにクランプ）。monitors 空 → rect 保持・`monitor=None`。
- `restore_or_default(monitors)` — `Err` を空 `RestoredLayout` に畳む（consumer の既定レイアウト方針へ）。
- `DockWorkspace::apply_restored_tree(w, tree)` — `attach_tree` の薄い別名（restore 語彙）。

### `WindowConfig.position`（`window.rs`）

```rust
/// 生成時の物理位置（仮想デスクトップ座標）。None = OS/backend 既定。
/// 復元した `WindowPlacement` の適用経路。size は従来どおり論理値。
pub position: Option<(i32, i32)>,   // Default: None
```

`session.rs::create_entry` と `host.rs::run` で `with_position(PhysicalPosition)` を条件付き適用。`SessionConfig::single` は `position: None` を写す。

## Data and Control Flow

### Detach → Reattach（new window への移動）

```text
consumer (menu/drop intent)
  → host.create_window(cfg)                      # 移動先準備（sync 部分）
  → ws.register_window(w2)                       # 失敗時は何も staging されない
  → ws.begin_move(v, DockTarget::Window(w2))     # v は transfer lock（ViewBusy）
  ... WindowEvent::Windowed{w2, Ready} ...
  → ws.commit_move(token)                        # 原子切替 + MoveCommitted{v, w1→w2}
  → consumer が source webview の view state を dest webview へ再構築（DOM 移送なし）
失敗系: create_window Err → begin 前で source 不変 / Ready 未到達や Destroyed
  → ws.abort_move(token) → source 完全復元 / commit 時消失 → MoveAborted+Err
```

### Window close（docked views）

```text
WindowEvent::Windowed{w, CloseRequested}
  → consumer の close policy が決定（dock 層は決めない）:
      keep   → respond_close(Veto)
      close  → ws.remove_window(w, DetachViews|RelocateTo|ReleaseViews)
             → respond_close(Approve) → Destroyed
  — dock 層は Job cancel / Project close を発火しない（R4）。
```

### Snapshot round-trip

```text
save:   LayoutSnapshot::capture(&ws, &[(w, geom)...]) → to_json
load:   LayoutSnapshot::from_json → restore(&host.monitors())
        → placements[i] の bounds を consumer の i 番目 window へ
          （WindowConfig.position + logical size = physical/scale）
        → ws.register_window(w_i) + apply_restored_tree(w_i, tree)
        → detached を ws へ登録
        → 失敗は restore_or_default → consumer の既定単一 window
```

## Non-Functional Constraints

- 非 RT 専用（UI スレッド）。全公開 enum `#[non_exhaustive]`、unsafe 不使用、新規依存なし（serde/serde_json 既存）。
- `DockWorkspace` は `WindowHandle`/`UiRuntime` を呼ばない純粋モデル — headless で全契約を検証可能にする分離。
- DAW/Project/Synvelis 語彙を持たない。View State は consumer 所有（`MoveCommitted` で handoff）。
- `DockPath` は構造変更で無効化 — op 引数は基本 `ViewInstanceId` で、path は `set_ratio`/汎用 `node` 参照用。

## Verification

- `tests/dock.rs`: モデル（構築/正規化/拒否/serde round-trip）、workspace（所有一意性・op 語彙・close 3 ポリシー）、move（commit・abort・commit 時消失・ViewBusy・menu-move one-shot）、persist（round-trip・schema 不一致・壊れた JSON・重複 View snapshot・画面外/モニター消失補正・空 monitors）、detach→reattach 保持。
- `examples/demo_dock.rs`: fake host で 2 window detach→reattach を script 実行。
- `mise run fmt-check`/`check`/`lint`/`test` + `check-ui-webview`/`lint-ui-webview`/`test-ui-webview`。

## Deferred Design Concerns

- `set_position`/`set_geometry`、floating View、プリセット語彙、native cross-window DnD、view-state 型付け port、DPI 変化時の比率再配置 — ADR/spec の Deferred 節どおり。
