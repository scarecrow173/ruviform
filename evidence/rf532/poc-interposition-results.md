# Issue #532 スロット合成 PoC — fork-free 介入の実測結果

Issue #532 (GPUI 0.2.2 DirectComposition の topmost ビジュアルがネイティブ子
HWND の画素を覆い、`real-dcomp-pixel` が fail する既知問題) への、GPUI fork を
伴わない 2 つの介入方式の実測記録。実験機構は `RVF_SLOT_HOLE` env で選択する
(`none|region|alpha|both`、既定 `none`)。

## 測定環境

- OS: Windows 11 (x86_64)、対話セッション (SESSIONNAME=Console)
- 実施: 2026-11-20、本 worktree `scarecrow173/532-slot-compositing-poc`
- コマンド: `RVF_SLOT_HOLE=<mode> cargo run --locked -p ruviform-plugin-dock-test
  --features real-dock --bin plugin-dock-real-scenario -- --out <jsonl>
  [--pixel-artifacts <dir>]`
- 証跡: `plugin-dock-real-baseline.jsonl` / `-poc-region.jsonl` /
  `-poc-alpha.jsonl` + `baseline-pixels/` `region-pixels/` `alpha-pixels/`
  `alpha3-pixels/`

## 結果サマリ

| mode | 機構の実効確認 | real-dcomp-pixel | 他ケース |
|---|---|---|---|
| baseline (変更前) | — | fail (green 0.000) | 20 pass / 2 unsupported |
| `region` | 適用済(後述の2証拠) | **fail** (green 0.000) | `real-os-input` が pass → not_run に回帰 |
| `alpha` | 適用済(描画抑制を計測確認) | **fail** (green 0.000) | 20 pass / 2 unsupported (baseline 同等) |
| canonical run (既定 `none`) | — | fail (green 0.000) | 20 pass / 2 unsupported = baseline 完全一致 |

## 方式1: root HWND `SetWindowRgn` hole (region)

### 機構

`project_slot_frame` が各フレーム投影する geometry/clip を source of truth に、
可視リース済スロットのクライアント矩形を GPUI root HWND の window region から
除外する。`sync_surface_regions` が suppression パス末尾で差分適用
(`AppliedRegion` 台帳で冗長 `SetWindowRgn` を抑止、frame resize も差分検出)。
hide/detach/抑制で既定 region へ復帰する。単体テスト
`region_hole_apply_and_clear` /
`sync_surface_regions_applies_and_restores_parent_hole` が
`GetWindowRgn`/`PtInRegion` で適用・復帰を決定論的に検証済。

### 適用の実証

1. `real-os-input` が baseline/alpha では pass する同一ケースで `key_tab` が
   `not_run`("window w1 is not the focused window") に変化。(960,552) への
   クリックが hole を素通りして背後の別 GPUI ウィンドウ (w4) へ届きフォーカスを
   奪取した — hole のヒットテスト透過を示す mode 固有の回帰。
2. `occlusion_hit_test` の子矩形内プローブ点が、editor child/container ではなく
   背後の `Zed::Window` (w4、184,89,1552x903) を返した。

### 失敗の構造

`SetWindowRgn` で親に開けた hole は **子 HWND ごと** ウィンドウバンドをクリップ
する。hole は描画とヒットテストの両方を透過し、子の画素は band 内に残らない。
結果として hole が見せるのは **背後のウィンドウ** であり、子ではない。
`real-dcomp-pixel-screen-0.png` は `alpha` 版と画素完全一致
(主色 (20,24,29) + 少数 (29,35,42)) — どちらも背後に居た w4 の描画。

## 方式2: swapchain アルファ hole (alpha)

### 機構

GPUI の合成スワップチェーンは `DXGI_ALPHA_MODE_PREMULTIPLIED`
(`directx_renderer.rs` の `CreateSwapChainForComposition`)、クリア色は
`[0.0;4]` の透過黒。そこで bound かつ placeable なスロットについて
slot div の `.bg(surface_raised)` とプレースホルダ quad を描画しないことに加え、
祖先チェーンの監査として「bound スロットを含む Panel の `.bg(surface_bg)` を
落とす」実装を入れた (PoC のため panel 全体の bg を落とす荒い近似。製品版は
quad 分割 carve が必要)。未 bound スロットと suppression ライフサイクルは
従来通り。

### 適用の実証

`RVF_SLOT_HOLE=alpha` で `native_slot_element` が `alpha_hole=true` を返し、
bound フレームでは `plugin.dock` panel の bg も描画されなかった
(診断 print で確認)。この時点でクライアント領域は 1px 境界線のみが描かれる
全面 α=0 で、ウィンドウは実際に透明化した。

### 失敗の構造

画面キャプチャの子矩形は依然として背後のウィンドウ (w4、同一テーマ故に同色
(20,24,29)+(29,35,42)) の画素を示し、green=0.000。つまり **topmost として
注入されたビジュアルの下に child HWND のコンテンツは存在しない**。
`CreateTargetForHwnd(hwnd, true)` は「子を含むバンド内の最上層」ではなく、
子の合成領域を含むウィンドウバンド自体の **上に** ビジュアルを差し込むため、
ビジュアルを透過にしても見えるのは背後のウィンドウであり、子ではない。

## 結論

2 方式とも「子 HWND が GPUI ウィンドウの合成バンド内・注入ビジュアルの下に
ある」という前提を共有するが、実測はその前提を否定した:

- region hole → 子は親 region でクリップされ、hole は背面を露出・入力も透過。
- alpha hole → ウィンドウは透明化するが、子はバンド外に合成されており露出
  しない。

GPUI 公開 API は DComp ターゲット/ビジュアルツリーに一切触れられない
(`evidence/rf532/gpui-public-surface-audit.md`)。したがって残る道は
**Option-A: GPUI を fork し `directx_renderer.rs` の
`CreateTargetForHwnd(hwnd, true)` を `false` に変更する** (注入ビジュアルを
子コンテンツの下に置く) こと。本スライスでは fork 自体は実施しない。

## 残存不確実性

- `both` mode は未測定 (2 つの失敗機構の合併で新情報なし)。
- alpha 下での画面が「背後の w4」か「デスクトップ」かは画素値からは完全には
  切り分けられないが、いずれにせよ w1 自身でも子でもないことは一致している。
- 環境ノイズ: baseline の一部キャプチャは前面に居た外部アプリ (Chrome) の
  画素を含んでいた。verdict への影響はない (いずれも green=0)。
