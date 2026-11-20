# Issue #532 raster-fix oracle — 実測結果

`real-dcomp-rasterfix` case の実 Windows lane 実測記録。
本 slice は ruviform 側の治療（private container class /
paint drain / reparent / layered）がユーザー可視 pixel を
復元できるかを、screen-composited green を判定 oracle に
して測るもの。**Issue #532 は open のまま**（本 slice は
`Refs #532` のみ）。

## 判定 oracle

`rf_screen` — `ospix::window_screen_rect` +
`capture_screen_rect`（`win32-bitblt-screen`）による
screen-composited 領域の `EDITOR_COLOR` green 比率。
`green_fraction >= 0.5`（`GREEN_VISIBLE_FRACTION`）を
「ユーザー可視 pixel が合成画面に到達した」と定義。
`rf_raster`（window-DC `BitBlt`）は raster 存在の診断であり
verdict 入力ではない。

## 実測結果（DComp on、既定）

Lane: `mise run test-ui-plugin-dock-real`
出力: `evidence/rf631/plugin-dock-real-scenario.jsonl`

| arm | window-DC raster | screen green |
| --- | --- | --- |
| baseline | — | **0.000** |
| pump（`UpdateWindow` + bounded `WM_PAINT`） | 0.000 | **0.000** |
| reparent（desktop reparent + `HWND_TOPMOST`） | 1.000 | **1.000** |
| layered | （未到達 — reparent で閾値達成） | — |

verdict: **Pass**（`fix_arm = "reparent"`）。
lifecycle（attach→open→close→detach→converge）は全 pass、
restore は parent/exstyle/rect の検証付きで完遂
（`parent_restored=exstyle_restored=rect_restored=true`、
全 restore op applied）。

PNG: `evidence/rf532/rasterfix-pixels/`
- `real-dcomp-rasterfix-screen-baseline-c0.png` — 暗色の GPUI
  面のみ、editor 緑なし（baseline screen 0.000）。
- `real-dcomp-rasterfix-screen-reparent-c0.png` — 全面
  `EDITOR_COLOR` 緑（reparent screen 1.000、ユーザー可視）。

## 実測結果（DComp off、補助 run）

`GPUI_DISABLE_DIRECT_COMPOSITION=1` で再実測
（出力: `evidence/rf532/plugin-dock-real-scenario-dcomp-off.jsonl`）。
root `WS_EX_NOREDIRECTIONBITMAP` bit は 0（env 変数が実際に
GPUI の DComp path を off にしたことを確認済み）。

| arm | window-DC raster | screen green |
| --- | --- | --- |
| baseline | — | **0.000** |
| pump | 1.000 | **0.000** |
| reparent | 1.000 | **1.000** |

verdict: **Pass**（`fix_arm = "reparent"`）。

## 解釈

1. **child の raster は実在する**。`reparent` arm で
   window-DC raster 1.000、screen 1.000 — 描画 backing は
   常に存在し、合成経路の問題だけが pixel を落としていた。
   DComp off では `pump` arm の raster も 1.000（drain で
   backing が即座に読める）。

2. **docked のままでは ruviform 側治療では復元しない**。
   private container class `RuviformSlotContainer`
   （`style: 0`、`DefWindowProcW`、`CS_PARENTDC` 排除）を
   入れても baseline screen は 0.000 のまま — `CS_PARENTDC`
   は原因ではなかった。`UpdateWindow` + caller-thread
   `WM_PAINT` pump も `update_pending` を解消するが
   （`update_before=true → update_after=false`）screen は
   0.000 のまま — pending paint 欠落も原因ではない。
   DComp off でも同じ結果 — DComp 切替単独でも原因ではない。

3. **presentation plane からの逸脱で raster は screen に届く**。
   child を desktop reparent（top-level 化）+ `HWND_TOPMOST`
   で可視帯へ固定した瞬間、screen green は 1.000。
   阻害層は「child の raster が GPUI root の presentation
   plane（DirectComposition/topmost 合成ツリー）に載らない
   こと」と確定できる — ruviform 側で presentation を別
   window 経路に逃がせば pixel は届く。

4. **製品 fix の含意**。docked-in-place の container を
   ruviform 側だけで直す道は本 probe の仮説では全て棄却
   された（private class / paint drain / layered）。
   残る実装選択は (a) editor 面を GPUI visual tree 外の
   top-level window として提示する ruviform 側 workaround、
   (b) GPUI fork 側で child raster を visual tree の
   composition 経路に載せる根本 fix。後者が docked-in-place
   を真正に直す唯一の経路であり、gpui-fork workstream の
   根拠として本証拠を使える。

## 実測で得た新規事実（probe 実装上の発見）

- `SetWindowLongPtrW(GWL_EXSTYLE)` は対象 window の parent が
  別 thread 所有（container = GPUI thread）だと win32 error 6
  で拒否される。desktop reparent 中は成功する。restore の
  exstyle write は reparent 解除 **前** に行う必要がある。
- desktop reparent した `WS_CHILD` window は desktop 子 list
  の child band に留まり、`HWND_TOP` では全 top-level
  sibling（実測 37 個）より下に残る。可視 oracle には
  `HWND_TOPMOST` への topmost-band 昇格が必要。
- `WS_EX_TOPMOST` は `GWL_EXSTYLE` write では解除できない
  （書込は error なしに drop される）。解除は
  `SetWindowPos(HWND_NOTOPMOST)` のみ有効。
- `SetWindowLongPtrW` の「成功なのに状態が変わらない」
  silent drop が実在するため、`applied` は API 戻り値では
  なく read-back 一致で判定する（拒否は `win32_error` に
  記録したまま）。

## Lane 実行記録

- `mise run test-ui-plugin-dock-real` — `real-dcomp-rasterfix`
  Pass、`real-dcomp-pixel`/`real-dcomp-bandescape`/
  `real-dcomp-childinv` は既知の未解決診断として Fail
  （exit 1 は設計どおりの honest signal）。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` aux run — 同上、
  `real-dcomp-rasterfix` Pass（`fix_arm = "reparent"`）。
- `--pixel-artifacts evidence/rf532/rasterfix-pixels` aux run —
  同上 + PNG 証跡。
- `mise run test-ui-plugin-dock-real-headless` — 28 行全てが
  `NotRun` + `observed.environment_blocked` の fail-closed
  mapping を自己検証して exit 0。
- `cargo test -p ruviform-plugin-dock-test --features real-dock`
  — 17 + 14 + 5 tests 全 pass。
- `cargo test -p ruviform-ui-gpui --features "gpui-test uitest"`
  — 全 pass（新規 `container_uses_private_class_without_parentdc`
  test を含む）。
- `mise run check-ui-driver-boundary` / `test-ui-native-slot` /
  `test-ui-plugin-dock` / `test-ui-plugin-dock-console` /
  `fmt-check` / `lint`（両 crate）— 全 pass。
