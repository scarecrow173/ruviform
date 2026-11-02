---
id: "TSPEC-79iCQBCeCxmbN386U8wYgM"
type: "test-spec"
status: "approved"
title: "Issue #182 Dock Tree / View Move / Layout Restore — test spec"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-182-dock-tree.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-182-dock-tree.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-182-dock-tree.md"
    - "docs/designs/issue-182-dock-tree.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 182
---

# Issue #182 Dock Tree / View Move / Layout Restore — test spec

## Feature: Dock Tree モデル（R1, R2）

### Rule: `ViewInstanceId` は非ゼロの安定論理 ID

- **Example**: `ViewInstanceId::new(0)` は `None`。非ゼロ値は `get()` で元値を返し、Hash/Eq/serde round-trip・Display（`v{n}`）が成立。

### Rule: `DockNode` コンストラクタが不変条件を拒否で守る

- **Example**: `DockNode::tabs(vec![], 0)` → `Err(EmptyTabs)`。`DockNode::split(H, 0.5, vec![leaf])` → `Err(InvalidChildren)`。`ratio` が 0.0/1.0/NaN/負 → `Err(InvalidRatio)`。`ratio=0.5`・children=2 は受理。

### Rule: `normalize` が構造を修復する

- **Example**: 空 Tabs を含む Split はその葉を除去 → 子が 1 つになれば Split 自体を子に畳み込む。`active >= views.len()` は最後尾へクランプ。ルートが空 Tabs だけなら tree は `None` 相当（空 window）へ。

### Rule: `validate` が tree 内重複 View を拒否

- **Example**: 同一 `ViewInstanceId` を 2 葉が持つ木は `validate` で `DuplicateView`。

### Rule: serde schema は版付きタグ形

- **Example**: `{"type":"split","axis":"horizontal","ratio":0.5,"children":[...]}` / `{"type":"tabs","views":[..],"active":n}` で round-trip。

## Feature: DockWorkspace と所有一意性（R3, R4, R5）

### Rule: 1 Window ≤ 1 tree・全 Window 横断で View 所有 ≤1

- **Example**: w1 の tree にある view を w2 の `attach_tree`/`attach_view` で再挿入 → `DuplicateView`。detached pool の view を再 attach は受理（所有移動）。

### Rule: op 語彙が不変条件を保ったまま変形する

- **Example**: `set_ratio` は (0,1) 外を拒否。`move_tab` は葉内 index を変え `active` を追随。`activate_view` は葉の active を対象 view に。`attach_view(view, Zone{anchor,Left})` は anchor 葉を水平 split `[Tabs[v], leaf]` に置換。

### Rule: Window close ポリシーは consumer 選択・既定なし

- **Example**: `remove_window(w, DetachViews)` は views を detached pool へ、`RelocateTo{w2}` は w2 root へ tab 集約、`ReleaseViews` は除去 — 返り値は対象 views。未登録 window は `UnknownWindow`。

## Feature: Two-phase move と失敗ロールバック（R6, R7）

### Rule: `commit_move` は所有権を原子的に切り替える

- **Example**: `begin_move(v, Window(w2))` → `commit_move` で v は w2 の tree へ（`MoveCommitted{v,from:w1,to:w2}`）。w1 の tree は正規化される。

### Rule: `abort_move` は source 不変で解除する

- **Example**: begin→abort で source leaf の view 順・active・split 構造が完全に同一。

### Rule: commit 時に移動先が消えていれば rollback

- **Example**: begin 後 `remove_window(w2, ReleaseViews)` → `commit_move` は `Err(TargetUnavailable)` + `MoveAborted` 発行、v は w1 source に残る。

### Rule: transfer 中の View は二重編集禁止

- **Example**: pending 中に同じ view へ `begin_move`/`move_view`/`detach_view`/`remove_view`/`move_tab`/`activate_view` → `ViewBusy`。別 view の op は受理。

### Rule: menu-move は `move_view` one-shot

- **Example**: `move_view(v, Window(w2))` は begin+commit と同結果を一回で返し、drag 機構を一切使わない。

## Feature: 永続化と復元（R8, R9, R10）

### Rule: snapshot は版付きで tree+detached+幾何を保持

- **Example**: capture→`to_json`→`from_json`→`restore` で tree/位置/モニターが往復。`schema != 1` → `UnsupportedSchema`、壊れた JSON → `InvalidSnapshot`。

### Rule: 不正スナップショットは拒否し安全デフォルトへ

- **Example**: 2 window が同じ view を持つ snapshot → `Err(InvalidSnapshot)` → `restore_or_default` は空 `RestoredLayout`（consumer の既定レイアウトへ）。tree 無効・w/h=0 も同じ。

### Rule: 画面外・消失モニター座標は可視領域へ補正

- **Example**: どの現モニターにも center が入らない rect は signature 一致モニター（なければ先頭）へ中央配置され `monitor=Some(i)` を返す。`monitors=[]` では rect 保持・`monitor=None`。display ID は schema に存在しない。

### Rule: `WindowConfig.position` が生成経路に届く

- **Example**: `position: Some((x,y))` で作った window が `with_position` 経由で配置される（adapter 経路の存在は check/lint で担保、headless ではフィールド保持を検証）。

## Feature: detach→reattach と境界（R6, R7, R11, R12, R13）

### Rule: 複数 Window 間で論理 View ID と state handoff が保たれる

- **Example**（fake-host デモ/tests）: v2 を w1→w2 へ detach（`begin_move`+準備+`commit_move`）後、w2→w1 へ reattach。同一 `ViewInstanceId` が維持され、`MoveCommitted` イベント列が from/to を正しく記録する（view state の serialize/restore は consumer 側実演）。

### Rule: 非回帰 — 既存 multi-window/単一 window 経路は不変

- **Example**: 既存 `multi_window.rs`/`runtime.rs` テストは無修正で green。`dock` 未使用時の consumer 影響なし。

## 実施形式

- `crates/ruviform-ui/src/dock.rs` の `#[cfg(test)]` ユニットテスト（T1–T3 各節）+ `crates/ruviform-ui/tests/dock.rs` 統合テスト（AC-4/AC-5/AC-7）+ `examples/demo_dock.rs`（fake-host script デモ）。全て headless。
