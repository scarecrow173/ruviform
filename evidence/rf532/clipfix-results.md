# Issue #839 GPUI fork clip-partition fix — 実測結果

GPUI fork 側 fix（`rf839-dcomp-child-raster` branch `b3144f2`）
適用後の real Windows lane 実測記録。ruviform 側はコード無
変更、`[patch]` override で fork を参照して検証。
**Issue #839 / #532 は open のまま**（本 doc は `Refs` のみ）。

## 判定 oracle

`issue-532-rasterfix` と同一: `rf_screen`（`win32-bitblt-screen`）
による screen-composited `EDITOR_COLOR` green 比率、
`green_fraction >= 0.5` で「ユーザー可視 pixel が合成画面に
到達」。`rf_raster`（window-DC `BitBlt`）は raster 存在の
診断であり verdict 入力ではない。

## 実測結果（patched gpui、DComp on）

Lane: `mise run test-ui-plugin-dock-real`
出力: `evidence/rf532/plugin-dock-real-clipfix.jsonl`
（canonical 実出力は `evidence/rf631/plugin-dock-real-scenario.jsonl`）

| arm | window-DC raster | screen green |
| --- | --- | --- |
| baseline（docked-in-place、治療 arm 不使用） | — | **0.999994** |
| pump / reparent / layered | （未到達 — baseline で閾値達成、`rf_arms` skip） | — |

verdict: **Pass**（`fix_arm = "baseline"` — 治療 arm を
一切使わない in-place 達成）。`rf_baseline_state` で
container/child とも exstyle `0x0`（layered/topmost/
noredirectionbitmap 全て false）を確認 — child の style /
parentage は変更していない。

PNG: `evidence/rf532/clipfix-pixels/`
- `real-dcomp-rasterfix-screen-baseline-c0.png` — 全面
  `EDITOR_COLOR` 緑（baseline screen 0.999994、
  docked-in-place でユーザー可視）。前 slice の同名 PNG
  （暗色面のみ、0.000）との対比が fix の効果。

## 実測結果（patched gpui、DComp off 補助 run）

`GPUI_DISABLE_DIRECT_COMPOSITION=1`
（出力: `evidence/rf532/plugin-dock-real-clipfix-dcomp-off.jsonl`）

| arm | window-DC raster | screen green |
| --- | --- | --- |
| baseline | — | 0.000 |
| pump | 1.000 | 0.000 |
| reparent | 1.000 | 1.000 |

verdict: **Pass**（`fix_arm = "reparent"`）。HWND flip
swapchain 経路は本 fix の対象外で、挙動は前 slice と同一
（raster はあるが presentation plane に覆われる）。debug
escape hatch としての位置づけは変わらない。

## fix の 2 要素と独立検証

| 要素 | 検証 | 結果 |
| --- | --- | --- |
| `WS_EX_NOREDIRECTIONBITMAP` 除去（raster 復元） | flag あり/なしで child window-DC raster 測定 | あり `0.000` → なし `1.000` |
| swapchain visual の clip partition（plane の穴あけ） | clip あり/なしで screen green 測定（child raster 存在の条件で） | なし `0.000` → あり `1.000` |

片方だけでは復元しない（flag あり + clip = raster 自体が
無く 0.000；flag なし + clip 無し = raster あるが覆われ
0.000）。両方で baseline `0.999994`。

## 棄却した経路（bisect で確定）

- `CreateSurfaceFromHwnd`: contract 上 layered window 専用、
  非 layered child には空 surface。
- `WS_EX_LAYERED` 化: foreign child に適用不能
  （`SetWindowLongPtrW` silent no-op、`WS_CHILD|LAYERED`
  作成 `0x80070006`）。
- blit-model swapchain（`SEQUENTIAL`+`STRETCH`）: 実験的に
  child は見えるが flip-model を捨てる製品 fix にしない。

## Lane 実行記録

- `mise run test-ui-plugin-dock-real` — `real-dcomp-rasterfix`
  Pass（`fix_arm = "baseline"`）、
  `real-dcomp-pixel`/`bandescape`/`childinv` は既知未解決の
  honest Fail（exit 1 は設計どおり）。3 独立 run で
  baseline `0.999994` を再現。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` aux run — Pass
  （`fix_arm = "reparent"`、前 slice と同一挙動）。
- `--pixel-artifacts evidence/rf532/clipfix-pixels` aux run
  — Pass + PNG 証跡。
- `mise run test-ui-plugin-dock-real-headless` — fail-closed
  mapping 自己検証、exit 0。
- `cargo fmt --all -- --check` / `mise run check-ui-gpui` /
  `test-ui-native-slot`（13 tests）/ `test-ui-plugin-dock` /
  `test-ui-plugin-dock-console` / `check-ui-driver-boundary` /
  `cargo test -p ruviform-ui-gpui --features "gpui-test
  uitest"`（78+22+4+7+2+1）— 全 pass。
- `mise run lint-ui-gpui` — **fail（scope 外の既存 issue）**:
  `surface.rs:1644` `clippy::cast_sign_loss`。main 上の既存
  コード由来で本 fix とは無関係、ruviform 側コード編集は
  編集範囲外のため未修正で記録。
- `cargo check -p gpui`（gpui clone 側）— pass。

## 残課題（次 slice へ defer）

- gpui fork の push / tag `v0.2.2-rf2`。
- ruviform `Cargo.lock` の gpui pin 更新（現在は
  uncommitted `[patch]` override で参照）。
- env-off（HWND flip swapchain）経路での child 提示 —
  現状 baseline 0.000 のまま、対象外として記録。
