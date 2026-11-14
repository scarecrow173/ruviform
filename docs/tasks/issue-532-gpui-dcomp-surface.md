---
id: "TASK-034XB3xs8Pxym2V631pZF5"
type: "task"
status: "done"
title: "Issue #532 GPUI DComp public-surface 監査タスク"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-532-gpui-dcomp-surface.md"
    modified:
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-native-dock.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
  implemented-by:
    - "docs/impl/ir/issue-532-gpui-dcomp-surface.md"
  depends-on:
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
    - "docs/specs/issue-532-gpui-dcomp-surface.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
    - "docs/impl/ir/issue-532-gpui-dcomp-surface.md"
    - "docs/impl/exp/issue-532-gpui-dcomp-surface.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
  derived-by:
    - "docs/impl/ir/issue-532-gpui-dcomp-surface.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 GPUI DComp public-surface 監査タスク

## Work

vendored `gpui-0.2.2`（cargo registry、読み取り専用）の Windows
platform 層を 7 項目監査し、docs-only で以下を固定する:
(a) DComp オブジェクトグラフと `CreateTargetForHwnd` の `topmost`
引数の全出現箇所、(b) `GPUI_DISABLE_DIRECT_COMPOSITION` の意味論
と非 DComp 経路、(c) public vs `pub(crate)` 到達可能性、
(d) Option A の patch 面積、(e) Option B の private-internal 要件、
(f) gpui version 情勢、(g) exit-127 のコードパス仮説。
先行 `docs/discovery/issue-532-dcomp-native-dock.md` の Open
Questions を更新する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-532-gpui-dcomp-surface.md`
- `docs/specs/issue-532-gpui-dcomp-surface.md`
- `docs/designs/issue-532-gpui-dcomp-surface.md`
- `docs/discovery/issue-532-gpui-dcomp-surface.md`
- `docs/test-specs/issue-532-gpui-dcomp-surface.md`

## Dependencies

- 先行 headless 監査 slice（`docs/discovery/issue-532-dcomp-native-dock.md`
  status: resolved、merge 済み）。

## Affected Files

- `docs/discovery/issue-532-gpui-dcomp-surface.md`（新規・正本）
- `docs/designs/issue-532-gpui-dcomp-surface.md`（新規）
- `docs/specs|plans|tasks|test-specs|impl/ir|impl/exp/issue-532-gpui-dcomp-surface.md`
  （新規・文書チェーン）
- `evidence/rf532/gpui-public-surface-audit.md`（新規・証拠ノート）
- `docs/discovery/issue-532-dcomp-native-dock.md`（Open Questions 更新）
- `docs/designs/issue-532-dcomp-native-dock.md`（upstream 調査リスク
  記述更新）
- 各 `docs/<type>/README.md` index（`doc-maintenance` で再生成）

## Done When

- [x] 監査 7 項目が `docs/discovery/issue-532-gpui-dcomp-surface.md`
  と `evidence/rf532/gpui-public-surface-audit.md` に記録された。
- [x] `topmost` literal が `directx_renderer.rs:793` の `true` 1 箇所
  のみと確定し、Option A の patch 面積が見積もられた。
- [x] gpui の DComp/DXGI/renderer が全て `pub(crate)` 以下で
  downstream から到達不能と確定した。
- [x] `GPUI_DISABLE_DIRECT_COMPOSITION` の受理値・非 DComp 経路の
  全差分が記録された。
- [x] exit-127 が仮説のみ（H1-H5）で PENDING として残された。
- [x] gpui 0.2.2 が最新 publishable・zed 現行版 publish 不能が
  `` `zed-industries/zed#55271` `` で裏付けられた。
- [x] 先行 discovery の Open Questions が更新され、status は
  resolved を維持した。
- [ ] `doc-maintenance` `index apply` + `doc-status` で relations
  解決を確認。
- [ ] `mise exec -- prek run --files <変更ファイル>` が exit 0。

## Verification

- [ ] `doc-maintenance` `index apply` + `doc-status`。
- [ ] `mise exec -- prek run --files <変更ファイル>`。
- [ ] 未検証事項（exit-127・`topmost=false`・実 plugin 画素）が
  PENDING/未検証として明記されている。
