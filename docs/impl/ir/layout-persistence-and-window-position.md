---
id: "IMPL-034QqITmjWC8LcEQVGkeAe"
type: "impl"
status: "completed"
title: "Layout persistence and window position"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/182"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui/src/dock.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/window.rs"
      - type: "file"
        path: "crates/ruviform-ui/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-ui-webview/src/session.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-182-03-persistence-position.md"
  implemented-by: []
  depends-on:
    - "dock-model-layer.md"
    - "dock-workspace-and-two-phase-move.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/issue-182-dock-tree.md"
    - "../../designs/issue-182-dock-tree.md"
    - "../../specs/issue-182-dock-tree.md"
    - "../../adr/issue-182-dock-tree-placement-move-contract-and-persistence.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-182-dock-tree.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Layout persistence and window position

## Summary

Issue #182 Task 3 の persistence/geometry 層を実装: versioned
`LayoutSnapshot`(schema=1)が window geometry・monitor signature・
dock trees・detached pool を JSON へ、restore 時の検証
(schema・geometry・tree 構造・全 window 横断の view 重複)と
off-screen/monitor 消失の補正、破損時は `restore_or_default` の
安全な空 layout へ。`WindowConfig.position` を追加し
`ruviform-ui-webview` が tao `with_position` へ伝達する。

## Context

docs/tasks/issue-182-03-persistence-position.md — ADR の
persistence 契約(versioned schema・display id 非依存・検証→
安全 fallback)を実装。

## Implementation

### Changes

- `LAYOUT_SCHEMA_VERSION = 1`、`LayoutSnapshot{schema,windows,
  detached}`、`WindowSnapshot{x,y,width,height,monitor,tree}`、
  `MonitorHint{name,width,height,scale_factor}`(値一致の
  signature — 永続 display id を仮定しない)。
- `LayoutSnapshot::capture`/`from_json`/`to_json`/`restore`/
  `restore_or_default`。`capture` は `WindowId` と
  `WindowGeometry` の parallel slice を要求。
- `restore`: schema 不一致→`UnsupportedSchema`、zero-size
  geometry→`InvalidSnapshot`、tree は clone→normalize→validate
  (全 view を `seen` で横断 dedup、detached とも衝突検査)、
  geometry は `correct_bounds` が中心点の monitor 所属で
  keep/re-center 判定、signature 一致→その monitor、失敗→
  先頭 monitor へ clamp、monitor 0 台→passthrough。
- `WindowConfig.position: Option<(i32,i32)>`(logical px、
  `None`=platform default)— additive。
- `ruviform-ui-webview/session.rs`: `create_entry` が
  `with_position(LogicalPosition)` を設定。

### Decisions During Implementation

- 全部が空になった tree は「window は空」として `None` に正規化
  (live semantics と一致)— 構造的に壊れた tree のみ snapshot
  全体を失敗させる。
- `WindowPlacement.monitor` は live monitor リストへの index —
  永続 id ではなく、signature 一致→index、それ以外は先頭。
- `DockTree` 復元は内部で `DockTree{root}` を直接構築
  (normalize+validate 済み)— public の `DockTree::new` と
  同じ不変条件を満たす。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

`mise exec -- cargo test -p ruviform-ui dock` — persistence テスト
(roundtrip・schema/構造/重複の reject・off-screen/missing-monitor
補正・dirty tree normalize)込みで green。
`mise run check-ui-webview`/`lint-ui-webview`/`test-ui-webview` も
`position` 配線を含めて green。

## Risks

`WindowConfig.width/height` は logical、snapshot 側は physical px —
consumer が scale_factor で換算する責務(`WindowPlacement` doc に
明記)。混合単位は snapshot 内で完結させている。

## Follow-ups

Task 4 が integration test/demo/verification で全体を覆う。
