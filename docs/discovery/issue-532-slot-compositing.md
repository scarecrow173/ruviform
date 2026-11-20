---
id: "DISC-rf532sltcmp0stn1t3rp"
type: "discovery"
status: "resolved"
title: "Issue #532 スロット合成 PoC — fork-free 介入 2 方式の実測と Option-A 判定"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added:
      - type: "file"
        path: "evidence/rf532/poc-interposition-results.md"
      - type: "doc"
        path: "docs/specs/issue-532-slot-compositing.md"
      - type: "doc"
        path: "docs/designs/issue-532-slot-compositing.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/surface.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
---

# Issue #532 スロット合成 PoC — fork-free 介入 2 方式の実測と Option-A 判定

## 背景

GPUI 0.2.2 の DirectComposition 経路は `directx_renderer.rs:793` で
`CreateTargetForHwnd(hwnd, true)` を呼び、注入ビジュアルを topmost として
差し込む。結果、ネイティブ子 HWND（コンテナ + プラグインエディタ）は
画面合成で完全に覆われ、`real-dcomp-pixel` が green_fraction=0 で fail
する。既存の切り分け（`evidence/rf532/uncovered-baseline-bisect.md` 等）で
「外部 HWND による遮蔽」「描画遅延」は否定済みで、残る原因は
compositor-internal の occlusion。

## 問い

GPUI を fork せずに `real-dcomp-pixel` を green にできるか。

## 試した介入（コスト順）

実験機構: `RVF_SLOT_HOLE` env で `none|region|alpha|both` を選択
（既定 `none`）。実装の詳細は
`docs/designs/issue-532-slot-compositing.md`。

### 方式1: root HWND `SetWindowRgn` hole（`region`）

可視スロットのクライアント矩形を GPUI root HWND の window region から除外。
**実測: FAIL。** hole は適用された（`real-os-input` が pass→not_run に回帰 —
クリックが hole を素通りして背後の w4 ウィンドウへフォーカスを奪取、
`occlusion_hit_test` のプローブも背後の w4 を返す — mode 固有の 2 つの証拠）
が、**子 HWND は親の window region でクリップされる**ため、hole が露出するのは
背後のウィンドウであり子ではない。green=0.000 のまま、かつ入力回帰あり。

### 方式2: swapchain アルファ hole（`alpha`）

`DXGI_ALPHA_MODE_PREMULTIPLIED` の合成スワップチェーン上で、bound/placeable
スロットの `.bg`・プレースホルダ quad・被覆祖先（bound スロットを含む Panel
の `.bg`）の描画を抑制し、スロット矩形を α=0 のままにする。
**実測: FAIL。** 抑制は全て実行されウィンドウは実際に透明化したが、画面
キャプチャが示すのは背後のウィンドウ（w4）の画素であり、子の画素は
**バンド内に存在しない**。green=0.000 のまま（他ケースへの回帰はなし）。

## 得られた構造的理解

両方式は「子 HWND が GPUI ウィンドウの合成バンド内・注入ビジュアルの下に
ある」という前提を共有するが、実測はこの前提を否定した:

- ウィンドウ region の hole は描画・ヒットテスト双方を透過し、子も一緒に
  クリップする。
- 注入ビジュアルを透過にしても、その下に子コンテンツは合成されない。
  `topmost=true` は「バンド内の子ビジュアルの上」ではなく、子の合成領域を
  含むバンド自体の上に乗る、という解釈と整合する。

つまり GPUI 公開サーフェス（`evidence/rf532/gpui-public-surface-audit.md`）の
範囲では、子 HWND を注入ビジュアルの下に届かせる手段が存在しない。

## 判定

**Option-A 確定: GPUI を fork し、`directx_renderer.rs` の
`CreateTargetForHwnd(hwnd, true)` を `false` に変更する**（注入ビジュアルを
ウィンドウコンテンツ＝子 HWND 群の下に置く）。本スライスでは証跡と PoC
機構のみを残し、fork 自体は実施しない。Issue #532 は open のまま。

## 証跡

`evidence/rf532/poc-interposition-results.md`（測定表・機構実証・残存
不確実性）、`plugin-dock-real-{baseline,poc-region,poc-alpha}.jsonl`、
`{baseline,region,alpha3}-pixels/`。canonical run
（`mise run test-ui-plugin-dock-real`、既定 `none`）は baseline と完全一致
（20 pass / 1 既知 fail / 2 unsupported）。
