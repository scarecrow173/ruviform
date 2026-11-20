---
id: "SPEC-rf532sltcmp0stn1t3rp"
type: "spec"
status: "proposed"
title: "Issue #532 スロット合成 PoC — fork-free 介入の評価と Option-A 判定"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "evidence/rf532/poc-interposition-results.md"
      - type: "file"
        path: "evidence/rf532/option-a-results.md"
      - type: "doc"
        path: "docs/discovery/issue-532-slot-compositing.md"
      - type: "doc"
        path: "docs/designs/issue-532-slot-compositing.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
---

# Issue #532 スロット合成 PoC — fork-free 介入の評価と Option-A 判定

## 目的

GPUI 0.2.2 DirectComposition が topmost 合成ターゲット
（`CreateTargetForHwnd(hwnd, true)`）を生成し、ネイティブ子 HWND プラグイン
画素を覆う問題（`real-dcomp-pixel` fail、green_fraction=0.000）に対し、
GPUI fork を伴わない介入で受入オラクルを通せるかを実測評価し、否なら
Option-A（GPUI fork）判定の根拠を確定する。

## スコープ

- 対象: fork-free 介入 2 方式（root HWND region hole、swapchain alpha hole）
  の実装・実測・判定。実験機構は `RVF_SLOT_HOLE` env opt-in（既定 `none`）。
- 非対象: GPUI fork の実施（別スライス）。Issue #532 は open のまま。
- 受入オラクル: `test-ui-plugin-dock-real` の `real-dcomp-pixel`
  （editor 子矩形のスクリーン `BitBlt` で green_fraction ≥ 0.5）。

## 判定結果

| 方式 | 機構実効 | `real-dcomp-pixel` | 副作用 | 判定 |
|---|---|---|---|---|
| region hole | 適用済（入力透過の回帰で実証） | fail (green 0.000) | `real-os-input` pass→not_run | **却下** |
| alpha hole | 適用済（全抑制実行・透明化確認） | fail (green 0.000) | なし | **却下** |

両方式とも「子 HWND が注入ビジュアルの下の合成バンド内に存在する」という
前提が実測で否定された。region の hole は子ごと band をクリップし、alpha は
透明化しても子画素がバンド内に合成されず、いずれも露出するのは背後の
ウィンドウ。GPUI 公開 API には DComp ビジュアルツリーへの手がかりがない
（`evidence/rf532/gpui-public-surface-audit.md`）ため、fork-free 介入は
構造的に不可能と判定。**Option-A 確定**（詳細・実測証跡:
`docs/discovery/issue-532-slot-compositing.md`、
`evidence/rf532/poc-interposition-results.md`、設計:
`docs/designs/issue-532-slot-compositing.md`）。

## Option-A 実施結果（後続スライスで実施済み）

`gpui` を `CreateTargetForHwnd(hwnd, false)` へ修正した git fork
（tag `v0.2.2-rf1`、`crates/ruviform-ui-gpui/Cargo.toml` で version
`=0.2.2` 制約を維持した git dep）へ切り替え、`RVF_SLOT_HOLE` 機構を撤去し
canonical run を再実走。**`real-dcomp-pixel` は引き続き fail**（green
0.000）で、verdict 構成・topmost ステージ画面 capture SHA256 が
baseline と完全一致 — 注入ビジュアルの z-order は遮蔽機構ではなかった。
遮蔽主体は presentation plane 自体と見るのが全証拠と整合（次候補は
`evidence/rf532/option-a-results.md` 参照）。

## 検証

- canonical run（`mise run test-ui-plugin-dock-real`、既定 `none`）:
  baseline と完全一致（19 pass / `real-dcomp-pixel` fail 既知 / 3 unsupported）。
- 非回帰: `test-ui-gpui`（79 単体 + 各スイート）pass、
  `native_slot_contract` 13 pass、`gui-boundary-guard --offline` pass、
  `fmt-check` pass、新規決定論テスト 2 件 pass。
- 実験 run: `RVF_SLOT_HOLE=region` / `alpha` の JSONL・画素証跡は
  `evidence/rf532/` に記録。
- Option-A run（fork 適用後の canonical run）: 19 pass /
  `real-dcomp-pixel` fail / 3 unsupported — verdict 構成は baseline
  と同一、presentation 系回帰なし（一覧は
  `evidence/rf532/option-a-results.md`）。

## Issue 状態

`real-dcomp-pixel` は Option-A 実施後も引き続き fail。Issue #532 は
**open** のまま — 遮蔽機構の再特定（presentation plane / swapchain flip
経路、または airspace 回避方式）を次スライスへ引き継ぐ。
