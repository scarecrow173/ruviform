---
id: "DISC-034ZaaUncoveredBaseline01"
type: "discovery"
status: "approved"
title: "Issue #532 uncovered-baseline bisect — 調査記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-532-uncovered-baseline.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-532-presentation-probe.md"
    - "docs/discovery/issue-532-foreground-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-uncovered-baseline.md"
    - "docs/designs/issue-532-uncovered-baseline.md"
    - "evidence/rf532/presentation-probe.md"
    - "evidence/rf532/foreground-probe-bisect.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-uncovered-baseline.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 uncovered-baseline bisect — 調査記録

## 調査目的

`evidence/rf532/presentation-probe.md` §残課題の 4 項（settle 付き
topmost 再観測・uncovered baseline 環境・自 window z_index 直接
観測・`PW_RENDERFULLCONTENT` 切り分け）を消化し、「残る
no-green が pre-paint/unsettled frame 由来か、compositor-internal/
presentation 層の遮蔽か」を確定する。

## 前提となる確定事実（前 3 slice の蓄積）

- 外部 foreign HWND（Chrome 等）が child rect を覆い、
  `WindowFromPoint` がそれを返す（dcomp-occlusion slice）。
- `SetWindowPos(HWND_TOP/HWND_TOPMOST)` で外部遮蔽を除去でき、
  hit-test は自 child を返すようになるが、screen green は 0 の
  まま（foreground-probe-bisect）。
- topmost 直後の screen capture は flat dark な新規 surface。
  これが「pre-paint frame の捕捉」か「真の非提示」かは settle
  未検証のため未判別（presentation-probe §残課題 a）。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` は root exstyle と
  window-DC 挙動を変えるが、final screen 画素・hit-test の結果を
  変えなかった（dcomp-pixel slice）。
- `WindowFromPoint` は HWND レベルの hit-test であり、DComp
  visual・flip-model plane は HWND でないため観測不能
  （osoccl の設計上の制約 — 明示済み）。

## 残課題と本 slice の対応

| 残課題 | 本 slice の措置 |
| --- | --- |
| (a) settle 未挿入の topmost capture | `std::thread::sleep` による累積 200/500/1200ms の 3 段 settle + 各段の hit-test/screen/`has_update` 観測（`real-dcomp-uncovered`） |
| (b) uncovered baseline 不在 | `osmut` staged mutation（raise→foreground→topmost）で遮蔽を除去し、hit-test の role 集計で `uncovered` を判定してから baseline screen を取り直す。作れない場合は失敗を実測として記録 |
| (c) inventory cap で自窓 z_index 不能 | `ospres::Inventory.own_windows` 追加 — 自 process top-level を cap 非依存で全件、真の z_index 付き |
| (d) `PW_RENDERFULLCONTENT` 未切り分け | post-settle 状態で `PrintWindow` flag 変種 + window DC を chain target に記録 |

## 設計上の確認（実装前に確定）

- case body は caller thread で走り、GPUI app thread は独自に
  pump を継続する。`observe_open` のシグネチャが `&RealOsGuiTest`
  で `settle()`/`settle_until`（`&mut`）に到達しないことからも
  確認できる。よって caller thread の `std::thread::sleep` で
  待っても child repaint・DWM 再合成は止まらず、sleep が
  presentation 経路を枯渇させる心配はない。message pump の
  自作は観測対象に寄与しないので不採用。
- settle は `settle_ms`・`settle_kind` を必ず記録する（暗黙待機
  禁止）。
- verdict 意味論は不変。`real-dcomp-pixel` の既知 `Fail` を
  regression として維持する。

## 実走結果（2026-11-20、interactive Windows x86_64）

詳細は `evidence/rf532/uncovered-baseline-bisect.md` を参照。
要点のみ記す。

- DComp on/off 両 run で `real-dcomp-uncovered` = `pass`
  （47 steps 完走、topmost 帯の verified restore 成功）。
- topmost 適用で 5 点の `WindowFromPoint` が全て `editor_child`
  に解決（`uc_uncovered` = `uncovered=true`）— **外部 HWND
  遮蔽は除去できた**。
- settle 0/200/500/1200ms の全点で screen green=0.0・同一
  sha256 — **pre-paint/unsettled frame 仮説は棄却**。
- DComp off では child windowdc=1.0・root windowdc=0.996 と
  window bitmap に green が存在するのに、同一 rect の screen
  composite は flat dark のまま — **最終合成は window bitmap
  以外の surface を源泉**とする。
- 結論: 残余 no-green は **compositor-internal /
  presentation-layer occlusion**。外部遮蔽・unsettled frame の
  いずれでも説明不能。遮蔽 surface の内部機構の同定は HWND 層
  probe の射程外で未解決として記録。
- `own_windows` で自 window の真の z_index を cap 非依存に取得
  （visible 4 件 z=22–25・invisible 3 件 z=287–289）— 残課題
  (c) を解消。`PW_RENDERFULLCONTENT` は post-settle でも全
  target flat dark（残課題 d は「同 flag path が uncovered かつ
  settled な状態でも有効画素を返さない」と確定）。
