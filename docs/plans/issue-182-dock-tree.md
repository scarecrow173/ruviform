---
id: "PLAN-0H8Jzc5oPC9umJnYUWKcbN"
type: "plan"
status: "approved"
title: "Issue #182 Dock Tree / View Move / Layout Restore — implementation plan"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
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
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/designs/issue-182-dock-tree.md"
  implemented-by:
    - "../tasks/issue-182-01-dock-model.md"
    - "../tasks/issue-182-02-workspace-move.md"
    - "../tasks/issue-182-03-persistence-position.md"
    - "../tasks/issue-182-04-tests-demo-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-182-dock-tree.md"
    - "docs/adr/issue-182-dock-tree-placement-move-contract-and-persistence.md"
    - "docs/test-specs/issue-182-dock-tree.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-182-dock-tree.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  gate: "M7"
  issue: 182
---

# Issue #182 Dock Tree / View Move / Layout Restore — implementation plan

## Scope Split

| Task | スコープ | 依存 |
| --- | --- | --- |
| T1 | dock モデル層（ViewInstanceId/Axis/DockNode/DockTree/DockPath/ViewLocation/DockError + normalize/validate + serde） | — |
| T2 | `DockWorkspace`（window 管理・close ポリシー・View 管理・op 語彙・two-phase move・DockEvent） | T1 |
| T3 | 永続化（LayoutSnapshot/restore/幾何補正）+ `WindowConfig.position` + adapter 反映 | T1, T2 |
| T4 | 統合テスト `tests/dock.rs` + `demo_dock` example + README/索引/AGENTS 整合 + 検証 lane | T1–T3 |

各タスクは TDD: 失敗するユニットテストを先に書いてから実装し、`mise run check`/`lint`/`test` で green にする。T4 の統合テストは cross-cutting シナリオ（detach→reattach・close・restore end-to-end）を担う。

## File Map

### T1 — dock モデル層

- 追加: `crates/ruviform-ui/src/dock.rs`（モデル節のみ — workspace/persist は後続タスクで同ファイルへ追加）・`crates/ruviform-ui/src/lib.rs`（`pub mod dock` + re-export）。
- 型: `ViewInstanceId`（NonZeroU64・`new(u64)->Option`・`get`・`Display`・serde transparent）・`Axis`・`DockNode`（`#[non_exhaustive]`・`tabs`/`split` コンストラクタ・`views()`/`is_leaf`）・`DockTree`（`new`/`root`/`views`/`locate`/`node`/`validate`/`normalize`）・`DockPath`・`ViewLocation`・`DockError`（Display+Error）。
- serde: `DockNode` は `#[serde(tag="type",rename_all="lowercase")]` — `{"type":"split","axis":"horizontal","ratio":..,"children":[..]}`/`{"type":"tabs","views":[..],"active":..}`。
- ユニットテスト（dock.rs 内 `#[cfg(test)]`）: 構築拒否（空 Tabs/<2 子/無効 ratio）・normalize（空葉除去・単一子畳み込み・active clamp）・重複 View 検出・locate/node/path・serde round-trip。

### T2 — workspace + move 契約

- `dock.rs` に `DockWorkspace`・`DockTarget`・`DropZone`・`MoveToken`・`MoveReport`・`WindowCloseDisposition`・`DockEvent` を追加。
- API: `register_window`/`remove_window(w,policy)`/`attach_tree`/`attach_view`/`detach_view`/`remove_view`/`owner_of`/`is_detached`/`detached_views`/`all_views`/`set_ratio`/`move_tab`/`activate_view`/`begin_move`/`commit_move`/`abort_move`/`move_view`/`is_transferring`/`drain_events`。
- ユニットテスト: attach/detach/remove・Zone 4 方向 + Tab・close 3 ポリシー・重複 View 拒否・begin/commit/abort・commit 時 target 消失 rollback・transfer 中 `ViewBusy`・pending 中 remove_window の自動 abort・`MoveCommitted{from,to}`・event 排出。

### T3 — persistence + position 適用

- `dock.rs` に `LAYOUT_SCHEMA_VERSION`・`LayoutSnapshot`（capture/to_json/from_json/restore/restore_or_default）・`WindowSnapshot`・`MonitorHint`・`WindowGeometry`・`RestoredLayout`・`WindowPlacement`・`DockWorkspace::apply_restored_tree` を追加。
- `window.rs`: `WindowConfig.position: Option<(i32,i32)>`。`session.rs::create_entry`/`host.rs::run` で `with_position(PhysicalPosition)`。
- ユニットテスト: capture/round-trip・schema 不一致・壊れた JSON・tree 無効・跨 window+detached 重複拒否・center-in-monitor 判定・signature 照合・モニター消失/空・w/h=0 拒否・`restore_or_default`。

### T4 — 統合テスト・demo・整合

- `crates/ruviform-ui/tests/dock.rs`: spec の AC-4/AC-5/AC-7 を網羅する headless シナリオ（fake host で detach→reattach・view state 保持・menu-move・failure rollback）。
- `examples/demo_dock.rs`: 2 window detach→reattach の script デモ（headless）。
- `crates/ruviform-ui/README.md`・`crates/ruviform-ui-webview/README.md`・`crates/README.md`・`AGENTS.md`（dock 契約 + position フィールド）・docs 索引再生成。
- 検証: `fmt-check`/`check`/`lint`/`test` + `check-ui-webview`/`lint-ui-webview`/`test-ui-webview`。

## Dependency Graph

```text
T1 (model) ──→ T2 (workspace+move) ──→ T3 (persist+position) ──→ T4 (tests/demo/verify)
```

## Risk Register

| Risk | 対策 |
| --- | --- |
| `WindowConfig` フィールド追加が外部構築リテラルを破壊 | ADR/design で明示・`..Default::default()` 必須を文書化・repo 内全構築点を確認 |
| `DockPath` の構造変更無効化が consumer を躓かせる | op は view 基底・path API は補助位置づけ + doc 明記 |
| serde restore が `#[non_exhaustive]` 構築子を迂回 | Deserialize は素の形を復元し `normalize`+`validate` で再保証（`DockTree::restore` 経路） |
| two-phase の「rollback が no-op」は B1 モデルの性質 | B2/B3 を採らない ADR 記録 + commit 時消失経路のテスト |

## Rollback

- 各タスクは `ruviform-ui` への additive 変更。revert はコミット単位（`WindowConfig.position` + adapter 反映は T3 コミットに同梱）。`dock` 未使用の consumer には影響なし。

## Verification Matrix

| AC | 検証 |
| --- | --- |
| AC-1/AC-2 | doc チェーン + `audit_docs` 各型 |
| AC-3 | `check`/`lint` + dock.rs ユニットテスト |
| AC-4 | `tests/dock.rs`（シナリオ一式）+ `test` |
| AC-5 | `demo_dock` example + `tests/dock.rs` の detach→reattach（fake host） |
| AC-6 | `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` |
| AC-7 | `fmt-check`/`check`/`lint`/`test` + menu-move テスト |
| AC-8 | API 語彙レビュー + 消費者影響分類（impl record）|
