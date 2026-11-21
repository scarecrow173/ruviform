---
id: "SPEC-034ZbbClipPart0000001"
type: "spec"
status: "approved"
title: "Issue #839 GPUI fork clip-partition fix — child HWND raster を DComp 経路に載せる根本 fix"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/839"
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-839-gpui-dcomp.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-839-gpui-dcomp.md"
  depends-on:
    - "docs/specs/issue-532-rasterfix.md"
    - "docs/specs/issue-532-child-inventory.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-839-gpui-dcomp.md"
    - "docs/discovery/issue-839-gpui-dcomp.md"
    - "docs/test-specs/issue-839-gpui-dcomp.md"
    - "evidence/rf532/clipfix-results.md"
    - "evidence/rf532/pinned-dep-results.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-rasterfix.md"
  derived-by:
    - "docs/designs/issue-839-gpui-dcomp.md"
    - "docs/discovery/issue-839-gpui-dcomp.md"
    - "docs/test-specs/issue-839-gpui-dcomp.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-839-gpui-dcomp.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 839
---

# Issue #839 GPUI fork clip-partition fix — spec

## 背景と目的

`issue-532-rasterfix` slice で、ruviform 側の治療（private
container class / paint drain / reparent / layered）が全て
棄却され、根本原因は「GPUI presentation plane が child
raster を覆う」側にあることが確定した。本 slice は
`issue-839` として **GPUI fork 側の根本 fix** を実装・検証
する。ruviform 側のコード変更は持たない（doc chain +
evidence のみ）。

対象リポジトリ: `scarecrow173/gpui`（ruviform が git 依存する
fork。実装 commit は同 repo の `rf839-dcomp-child-raster`
branch `b3144f2`）。

## 受入条件

| # | 条件 | 判定 |
| --- | --- | --- |
| AC1 | `real-dcomp-rasterfix` の **baseline arm**（docked-in-place、治療 arm 不使用）で screen green `>= 0.5` | `evidence/rf532/plugin-dock-real-clipfix.jsonl` |
| AC2 | child HWND の style / parentage を変更しない（reparent・topmost・layered・private class いずれも不使用） | 実装 review + `rf_baseline_state` で exstyle `0x0` のままを確認 |
| AC3 | GPUI が frame を出さない期間でも attach 後の child が uncover される（timer 駆動 sync） | `handle_create_msg` で `CHILD_WINDOW_SYNC_TIMER_ID` を arm、timer handler が `sync_child_windows` を呼ぶ |
| AC4 | `RefCell already borrowed` 等の reentrancy panic なし | real lane 完走 |
| AC5 | device-lost 後に composition 木と partition を再構築する | `handle_device_lost` で `DirectComposition::new` + `set_swap_chain` 再実行、next sync で partition 再計算 |
| AC6 | 既存の pass case を維持（24 pass + 3 unsupported の既知 verdict 集合を悪化させない） | canonical lane JSONL 比較 |

## スコープ外

- `GPUI_DISABLE_DIRECT_COMPOSITION=1`（HWND flip swapchain）
  経路での child 提示 — 同経路は debug escape hatch であり、
  baseline arm の失敗は既知のまま残す（本 slice で対象外）。
- `real-dcomp-pixel`（別 lane window の遮蔽）、
  `real-dcomp-bandescape` / `real-dcomp-childinv`
  （`WS_EX_LAYERED` 適用可否の診断 case）— いずれも
  既知の honest Fail で、本 fix とは別課題。
- ~~gpui fork の push / tag `v0.2.2-rf2`、ruviform `Cargo.lock`
  の pin 更新~~ — 後続 slice で完了: tag `v0.2.2-rf2`
  （`b3144f2`）を `crates/ruviform-ui-gpui/Cargo.toml` +
  `Cargo.lock` に pin し、pinned dep で real lane を再検証
  （`evidence/rf532/pinned-dep-results.md`）。
