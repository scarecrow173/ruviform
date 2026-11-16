# Issue #532 foreground/compositor mutation probe — bisect 記録

Issue `scarecrow173/ruviform#532` の foreground/compositor probe slice
（`real-dcomp-foreground` case + `src/osmut.rs`）の実走結果と、得られた
bisect 解釈の記録。**Issue #532 は open のまま** — 本書は診断証拠であり、
compositor 修正ではない。

- Date: 2026-11-16
- OS: Windows (x86_64)、interactive desktop
- Lane JSONL repo_sha 記録値: `513b33450e8cef4b0e41bf1b3250b9c7418c5746`
- Lanes（review 修正後の最終 binary で実走）:
  - `evidence/rf631/plugin-dock-real-scenario.jsonl`（baseline、DComp 有効）
  - `evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl`
    （`GPUI_DISABLE_DIRECT_COMPOSITION=1`）
  - `evidence/rf631/plugin-dock-real-headless.jsonl`（`--blocked-probe`、
    14 case 全て `not_run` + `environment_blocked`）

## Probe 概略

editor child attach 済み・lease 生存中に、child HWND の
`GetAncestor(GA_ROOT)` で GPUI root を解決し、baseline snapshot
（exstyle / topmost / z-order 近傍 / foreground owner）を取得後、
次の staged mutation を各 child・各 stage で実行する。

1. baseline observation
2. `SetWindowPos(HWND_TOP, SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)`
3. `SetForegroundWindow(root)`
4. `SetWindowPos(HWND_TOPMOST, ...)`
5. restore（topmost band 復元を検証、z-order/foreground は best-effort）

各 stage で `WindowFromPoint` hit-test（child rect 内 5 点）と screen
capture（`win32-bitblt-screen`、green 画素数/割合）を記録する。
`real-dcomp-foreground` の verdict は probe 実行・必須 mutation 成功・
restore 検証・lifecycle のみで判定し、green の有無や hit-test 結果は
**verdict 入力にしない**（bisect データ）。

## 実走結果

### `real-dcomp-foreground` verdict: 両 run とも `pass`

| run | raise | foreground | topmost | restore |
| --- | --- | --- | --- | --- |
| baseline (DComp on) | pass | note（foreground lock に拒否、`applied:false`, error 0） | pass | pass（`topmost_restored:true`、SetWindowPos 系全適用） |
| DComp disabled | pass | **pass**（`applied:true` — lock が許可） | pass | pass（`topmost_restored:true` + foreground hand-back も適用、`foreground_final` = baseline owner） |

case `pass` は「probe が設計どおり実行され状態を復元した」ことのみを
意味し、occlusion 解消を主張しない。`SetForegroundWindow` の結果は
run ごとに異なり（baseline では lock 拒否、DComp off では適用）、
いずれも設計どおり記録されている。

### Stage 別 hit-test / screen green（最終 run）

sample 点の `WindowFromPoint` class 名と screen green 画素数
（`win32-bitblt-screen`、child rect 1,278×798 = 1,019,844 画素）:

| stage | baseline (DComp on) | DComp disabled |
| --- | --- | --- |
| baseline | `Chrome_RenderWidgetHostHWND` | `Zed::Window` |
| raise (`HWND_TOP`) | `Chrome_RenderWidgetHostHWND` | **`RuviformTestEditor`（自 HWND）** |
| foreground | `Chrome_RenderWidgetHostHWND` | `RuviformTestEditor` |
| topmost (`HWND_TOPMOST`) | `RuviformTestEditor` | `RuviformTestEditor` |
| restore | `Chrome_RenderWidgetHostHWND` | `Zed::Window` |

screen green は **全 stage・全 run で 0**（baseline run のみ
`real-dcomp-pixel` で ~1px の単発値を観測した前回 run があったが、
最終 run では 0.0）。

補足 snapshot（baseline）:

- DComp on: root `WS_EX_NOREDIRECTIONBITMAP` set、`topmost:false`、
  `above_count:31`、foreground owner は別 window。
- DComp off: `WS_EX_NOREDIRECTIONBITMAP` 消失、`above_count:28`、
  foreground owner = `Zed::Window`（= baseline hit-test の遮蔽物と
  同一 HWND — 覆っている window が foreground でもあった）。
- restore 記録: `zorder_before/after` は `above_count` 一致
  （28→28）だが `above_hwnd` は近似復元のため別値 — best-effort
  であることがそのまま記録されている。

### `real-dcomp-pixel`（対照、authoritative screen-pixel verdict）

両 run とも `fail` を維持 — `green_observed:false`、best fraction
0.0。DComp off run の `windowdc_pixels` note では child 自身の
window DC が **green_fraction 1.0**（1,019,844 / 1,019,844）—
子の surface に green が存在するのに screen / `PrintWindow` には
出ない（baseline run の同 note は今回 0.0 — surface paint の
タイミング差の可能性があり、note 級診断としてそのまま記録）。

## Bisect 解釈

1. **外部 HWND stacking は第一の境界として確定 — ただし環境依存。**
   baseline stage の hit-test は run により `Chrome_RenderWidgetHostHWND`
   または `Zed::Window` を返す。design の想定どおり遮蔽物は
   実行ごとに変わり得るが、外部 window が child rect を HWND レベルで
   覆う事実は一貫している。

2. **HWND z-order の cover は浅い場合 `HWND_TOP` で除去できる —
   しかし screen green は戻らない。**
   DComp off run では `HWND_TOP`（非 topmost band 内の繰り上げ）だけで
   hit-test が自 child HWND（`RuviformTestEditor`）に切り替わった。
   baseline run では `HWND_TOPMOST` まで必要だった（cover がより
   上位にいた）。いずれの場合も、**hit-test が自 HWND を返した
   stage 以降も screen green は 0 のまま** — HWND z-order は
   今回の no-green の十分条件ではない。

3. **残余の遮蔽は compositor/プレゼンテーション層にある。**
   `WindowFromPoint` が自 child HWND を返す = HWND 木上その点で
   最前面が自 window であることの証明だが、screen capture には
   green が出ない。DComp off では child の window DC が完全な
   green（1.0）を持つのに screen / `PrintWindow` は 0 —
   「surface にはあるが合成結果に無い」方向の証拠。
   `GPUI_DISABLE_DIRECT_COMPOSITION=1` はこの残余を解消しない
   ため、GPUI root の DComp swapchain 経路だけには帰属しない。

4. **未解決として残る候補（断定しない）:**
   - hit-test を透過する `WS_EX_TRANSPARENT`/layered/DComp visual が
     視覚的には覆ったまま残る可能性（`WindowFromPoint` はこれらを
     見えない）。DComp off run で `HWND_TOP` 後に self hit-test に
     なっても green 0 だった事実は、この種の「HWND 木に現れない
     視覚 cover」を排除しない。
   - child HWND 自身の surface → DWM 合成の presentation 経路
     （plugin 描画・redirection・flip-model）の問題。
   - screen capture（`BitBlt` screen DC）自体が対象 content を
     捉えられない可能性 — window DC との差分は「surface にはあるが
     合成結果に無い」方向を支持するが、capture 盲点の完全な否定には
     ならない。
   - `WindowFromPoint` は DComp visual/plane を観測できないため、
     「self HWND + no green」は強い外部 stacking 否定証拠だが、
     compositor 内部の直接観測ではない。

5. **`SetForegroundWindow` の結果は環境依存で正直に記録された。**
   baseline run では foreground lock に拒否（`applied:false`,
   `win32_error:0` → note）、DComp off run では適用
   （`applied:true`）。後者では restore の foreground hand-back も
   適用され `foreground_final` が baseline owner に一致した。
   いずれも foreground branch を「解決済み」とは主張しない。

## 結論（bounded）

- Issue #532 の症状は少なくとも 2 段の境界を持つ:
  (a) 外部 HWND stacking（実行ごとに異なる foreign window）—
  probe の mutation で再現・除去・復元を確認、
  (b) HWND z-order では説明できない compositor/presentation 内部の
  occlusion — hit-test が自 HWND に解決した stage でも screen
  green が出ないことで存在を確認（topmost だけでなく raise 段階
  でも再現）。
- (b) の機構特定（hit-test-transparent surface / child presentation /
  capture 盲点の切り分け）は本 slice の範囲外。次 slice の候補:
  `PrintWindow(PW_RENDERFULLCONTENT)`・DWM サムネイル・
  `DComposition` visual tree 走査・child window の WS_EX/転送
  surface 属性の追加診断。
- `real-dcomp-pixel` の `Fail` は引き続き authoritative であり、
  本 probe はそれを Pass に変換しない。Issue #532 は open のまま。
