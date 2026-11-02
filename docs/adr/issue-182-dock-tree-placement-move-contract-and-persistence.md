---
id: "ADR-QYZTHdxPLL6w7vQT2qyPuW"
type: "adr"
status: "accepted"
title: "Issue #182 Dock Tree — contract placement, two-phase view move, persistence schema, and close policy"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
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
        path: "crates/ruviform-ui-webview/src/session.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/host.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/issue-181-multi-window-contract-placement-close-decision-ipc-routing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-182-dock-tree.md"
    - "docs/discovery/issue-182-dock-tree-inventory.md"
    - "docs/designs/issue-181-multi-window.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-182-dock-tree-inventory.md"
    - "docs/specs/issue-182-dock-tree.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
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

# Issue #182 Dock Tree — contract placement, two-phase view move, persistence schema, and close policy

## Context and Problem Statement

Issue #182 requires a reusable docking foundation over the merged #181 multi-window host: per-window Dock Trees (`Split`+`Tabs`), a global ≤1-owner invariant for logical `ViewInstanceId`s, a view move contract (prepare destination → switch ownership → roll back on failure), a versioned persistence schema with geometry/monitor correction, a consumer-chosen window-close policy, and an app-agnostic operation layer with a drag-free menu-move fallback. The inventory (`docs/discovery/issue-182-dock-tree-inventory.md`) shows none of this exists today. The decisions to make:

1. Where the dock contract lives (`ruviform-ui` vs adapter vs new crate) and the node schema's exact shape.
2. How the view move is sequenced — and where the view sits between begin and commit (source-dock vs limbo) without violating the single-owner invariant.
3. How ownership uniqueness is enforced (derived scan vs maintained index).
4. What the persistence schema persists (WindowIds are session-scoped and never persistable; display IDs are not stable) and how corrupt/off-screen data is handled.
5. How restored geometry reaches real windows (`WindowConfig` extension vs new `WindowHandle` method).
6. Where the drag/menu operation vocabulary sits relative to app UI and how the menu-move fallback avoids drag machinery.

## Decision Drivers

- Issue 本文の不変条件: 所有先高々 1 箇所・移動先準備後の所有権切替・失敗時元 Dock 復元・移行中の二重編集禁止・DOM 移送非保証・ディスプレイ ID 非永続・consumer 選択の close ポリシー。
- 層境界: 「Generic Dock/Window/Reusable View State を UI 層へ」— Synvelis 意味論（View factory/State/Project）を Ruviform に入れない合意（Synvelis #21/#22、ruviform#192）。
- #181 との整合: `WindowId` は session-scoped（永続化不可）、`CloseRequested` は Window 単位、`WindowHandle` は consumer が駆動する port。
- 暫定規約: 全公開 enum `#[non_exhaustive]`、unsafe 禁止、依存追加理由記録（serde 既存）、TDD。

## Considered Options

### A. 契約の配置

- **A1 `ruviform-ui::dock` モジュール（採用）** — backend 非依存の UI 基盤層責務。`WindowId`/`MonitorInfo`/`WindowHandle` と同じクレートで型整合が閉じる。
- **A2 新規クレート `ruviform-ui-dock`** — 分離は明確だが、`WindowId` への依存で `ruviform-ui` へ戻る依存が必須となり層構造を複雑化するだけ。利益なし。
- **A3 `ruviform-ui-webview` 側** — adapter は backend 実装の置き場であり、汎用契約の所有先として誤り（baseline の adapter 規則に反する）。

### B. View 移動の中間状態（begin〜commit 間）

- **B1 View は source dock に残ったまま transfer ロック（採用）** — 所有権切替は commit の原子操作（detach+attach+正規化）。失敗 = source 不変なので rollback は構造的 no-op。不変条件が全時点で自明。
- **B2 begin 時に limbo へ detach** — 「元 Dock に戻す」を literal に実現するが、移行中 View が一時的に無所属となり、abort は「元の leaf/path へ正確に戻す」復元ロジック（挿入位置・active index の再現）を要する。失敗モードが増える。
- **B3 移動先に staged copy を作成後 swap** — 移行中に View が 2 箇所に存在する期間を作り、単一所有不変条件に直接反する。

### C. 所有不変条件の実装

- **C1 `WindowId → tree` map が唯一の真実・所有検索は走査で導出（採用）** — 単一 source of truth、view 数は小規模（数十）で走査コスト無視可。
- **C2 `ViewInstanceId → owner` 逆引き map を常時維持** — O(1) だが二重管理で不整合リスクを導入する。

## Decision Outcome

### 配置とノード schema

**A1 採用**: `ruviform-ui::dock`。`DockNode` は `#[non_exhaustive]` enum で `Split{axis,ratio,children}`/`Tabs{views,active}`。variant は crate 外から直接構築不可（`#[non_exhaustive]`）なので `DockNode::split`/`DockNode::tabs` コンストラクタが不変条件を拒否で守り、構造変更 op は常に `normalize()`（空 Tabs 除去・<2 子 Split 畳み込み・active clamp）を通る。`ratio` ∈ (0,1) 有限のみ受理 — 最小 pane サイズ等の UI ポリシーは consumer 側。

### Two-phase move

**B1 採用**: `begin_move(view, target) -> MoveToken` は View を `pending` にマークするのみ（構造不変・後続 op は `ViewBusy`）。`commit_move(token)` が detach→attach→normalize を原子的に行い `MoveCommitted{view,from,to}` を発行。commit 時に target Window が消えていれば pending を解除して `MoveAborted` + `Err(TargetUnavailable)`。`abort_move(token)` は解除のみ。one-shot `move_view` = begin+commit（メニュー移動・in-window drop 経路）。consumer は `MoveCommitted` で source 側 view state を serialize → dest webview で再構築（DOM 移送は保証しない）。

### 所有不変条件

**C1 採用**: `windows: BTreeMap<WindowId, Option<DockTree>>` が唯一の真実。`detached: BTreeSet<ViewInstanceId>` が未所属 pool。所有検索・重複検査は走査導出。

### 永続化 schema

- `LayoutSnapshot{schema:u32, windows:Vec<WindowSnapshot>, detached:Vec<u64>}`。`WindowSnapshot{x,y,width,height(物理px), monitor:Option<MonitorHint>, tree:Option<DockNode>}`。
- **WindowId は載せない**: session-scoped なので window は配列順で復元対応付け（consumer が同順で再作成）。
- **display ID は載せない**: `MonitorHint{name,width,height,scale_factor}` の signature 照合 — 不一致/消失は先頭モニターへフォールバック。
- 復元: schema/version 検査 → 各 tree `normalize`+`validate` → 全 window+detached 横断の重複 View 拒否 → 画面外 rect を現モニターへ補正。失敗は `Err`、`restore_or_default` が安全デフォルト（空レイアウト → consumer の既定単一 Window 方針へ）を返す。信頼できないデータから構造を捏造しない。

### Window 幾何の適用経路

`WindowConfig` に `position: Option<(i32,i32)>`（物理 px、仮想デスクトップ）を additive 追加し、`run_multi`/`run` の `WindowBuilder::with_position` に通す。`WindowConfig` は `#[non_exhaustive]` でない構造体（利用側構築）のため、フィールド追加は `..Default::default()` 未使用のリテラルを破壊する — ワークスペース内の全構築点は `..` 付きであり、外部 consumer 影響は ADR/impl record で明示する（pre-1.0・`publish=false`）。`WindowHandle::set_position` は追加しない（YAGNI — 実行時移動は `Moved` で追跡済み）。

### 操作層と drag/drop

splitter/tab 並べ替え/drag preview/drop target の**語彙**（`DropZone`/`DockTarget`/`DockPath`）と op API は `ruviform-ui::dock` 所有。drag の視覚層・hit-test は frontend 責務。cross-webview ネイティブ drag は wry/各 backend が保証しないため（discovery 調査）、Window 間移動の commit は常に本 API 経由 — メニュー移動は `move_view` で drag 機構ゼロの同等経路（フォールバック要件を構造的に満たす）。

### Window close ポリシー

`remove_window(window, WindowCloseDisposition)` は policy を必須引数とし既定を持たない: `DetachViews`（detached pool へ保持）/`RelocateTo(w)`（別 window の root tab へ集約）/`ReleaseViews`（除去 — consumer が state 保存済みの終了経路）。dock 層は Job cancel・Project close を暗黙発火しない。

## Consequences

### Positive

- 不変条件が型とコンストラクタに閉じ、headless で全経路を検証可能。
- rollback が構造的 no-op — 失敗経路に新規状態を持ち込まない。
- snapshot は ephemeral ID を排除した純粋 data — session を跨ぐ永続化に適合。
- menu-move が第一級 API であり、drag 未対応 backend/headless で移動全体を検証できる。

### Negative / 留意

- `WindowConfig` への `position` 追加は `..Default::default()` なしの外部リテラルを破壊し得る（pre-1.0・受入記録で明示）。
- `RelocateTo` は View 集約のみでレイアウト構造を持ち越さない（detached pool で無損失保持は `DetachViews`）。
- `DockPath` は構造変更で無効化する（イベント処理前の再 hit-test が consumer 側の責務 — design で明記）。
