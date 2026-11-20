# Issue #532 uncovered-baseline bisect — 判別記録

Issue `scarecrow173/ruviform#532` の uncovered-baseline slice
（`real-dcomp-uncovered` case + `ospres::Inventory.own_windows` +
settle 付き再観測）の実走結果と、presentation-probe で未確定だっ
た残余判別の結論記録。**Issue #532 は open のまま** — 本書は診
断証拠であり、compositor 修正・方式選定ではない。

- Date: 2026-11-20
- OS: Windows (x86_64)、interactive desktop
- Lane JSONL（実装 HEAD `2bb902b2` + working tree 差分）:
  - `evidence/rf631/plugin-dock-real-scenario.jsonl`
    （baseline、DComp 有効、`--pixel-artifacts
    evidence/rf631/uncovered-probe-dcomp-on`）
  - `evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl`
    （`GPUI_DISABLE_DIRECT_COMPOSITION=1`、`--pixel-artifacts
    evidence/rf631/uncovered-probe-dcomp-off`）
  - 複写: `evidence/rf532/plugin-dock-real-uncovered-dcomp-on.jsonl`
    / `-dcomp-off.jsonl`（同一 run の slice 側コピー）
- Pixel artifacts: `evidence/rf631/uncovered-probe-dcomp-on/*.png`
  および `…-dcomp-off/*.png`（両 run で保存 — 本 slice では
  screen raster の内容解釈が判別の主証拠のため）

## Probe 概略

editor child attach 済み・lease 生存中に実行する staged
observation + verified-restore mutation:

1. `uc_inventory` — `ospres::inventory` の新 field
   `own_windows`（自 process の top-level window を cap 非依存で
   列挙、full-enumeration の真の `z_index` 保持）
2. `uc_baseline_state` — `osmut::observe` で child→GA_ROOT 解決 +
   exstyle/z-order/foreground snapshot
3. `uc_observe_baseline` — 被遮蔽 baseline の hit-test + screen
   capture
4. env-prep mutation — `uc_apply_raise`（`HWND_TOP`）→
   `uc_apply_foreground`（`SetForegroundWindow`）→
   `uc_apply_topmost`（`HWND_TOPMOST`）、各 `uc_observe_*` で
   hit-test + screen green を記録
5. `uc_uncovered` — 全 `WindowFromPoint` hit が dock chain role
   （`editor_child`/`container`/`root`）に解決するかの集計
6. settle curve — `std::thread::sleep` で累積 {200, 500, 1200}ms
   （case body は caller thread、GPUI app thread は pump 継続）。
   各点で hit-test + screen green + `GetUpdateRect`
   （`update_pending`）を記録
7. `uc_capture` — post-settle の `PrintWindow` flags 4 種 +
   window-DC を child/container/root に実行
8. `uc_restore` + `uc_observe_restore` — baseline の topmost/
   z-band/foreground への verified restore

verdict は「probe 実行 + restore 検証 + lifecycle 収束」のみで
判定し、green/uncovered/update_pending は全て note 級 data。
両 run とも `real-dcomp-uncovered` = `pass`（47 steps 完走）。

## 実走結果

### DComp on（baseline run）

| stage | hit-test role（5 点） | screen green | 備考 |
| --- | --- | --- | --- |
| baseline | `other` ×5（foreign cover） | 0.0 | `above_count=39` |
| raise (`HWND_TOP`) | `other` ×5 | 0.0 | `above_count=36` |
| foreground | `other` ×5 | 0.0 | `SetForegroundWindow` refused（lock、applied=false） |
| **topmost** | **`editor_child` ×5** | **0.0** | `above_count=12`、topmost=true |
| settled 200ms | `editor_child` ×5 | **0.0** | `update_pending=true` |
| settled 500ms | `editor_child` ×5 | **0.0** | `update_pending=true` |
| settled 1200ms | `editor_child` ×5 | **0.0** | `update_pending=true` |
| restore | `editor_child` ×5 | 0.0 | topmost 帯は検証済み復元 |

`uc_uncovered`: `uncovered=true`（全 hit が chain role）。
topmost〜1200ms の 4 screen capture は同一 sha256
（`7dd8975d…`）— topmost 化で baseline capture
（`1393e26b…`、Chrome 内容）から**別の surface に切替後、その
内容が settle 全域で不変**。PNG 実視認: flat dark `(32,34,42)`
系の一様 raster — foreign window の残像でも stale frame でも
ない新規 surface。

post-settle capture（child rect 1,278×798）:

| target | path | DComp on green |
| --- | --- | --- |
| editor_child | printwindow f0/f1 | **1.0 / 1.0** |
| editor_child | printwindow f2/f3（`PW_RENDERFULLCONTENT`） | 0.0 / 0.0 |
| editor_child | windowdc | 0.0 |
| container | printwindow f0/f1 | error 0 |
| container | printwindow f2/f3 / windowdc | 0.0 |
| root | 全 variant | 0.0 |

### DComp off（`GPUI_DISABLE_DIRECT_COMPOSITION=1`）

`uc_baseline_state` で `noredirectionbitmap=false` を確認
（env var は GPUI window に実効あり）。hit-test・green 時系列は
DComp on と**完全に同一形状**（topmost で `editor_child` ×5、
settle 全域で screen green=0.0、`update_pending=true` 持続）。

post-settle capture が決定的差分を示す:

| target | path | DComp off green |
| --- | --- | --- |
| editor_child | printwindow f0/f1 | **1.0 / 1.0** |
| editor_child | windowdc | **1.0** |
| container | windowdc | **1.0** |
| root | printwindow f0 / f1 | **0.938 / 0.996** |
| root | windowdc | **0.996** |
| screen（child rect） | bitblt | **0.0** |

**window 自体の redirection bitmap には plugin 画素が含まれて
いる**（child windowdc=1.0、root windowdc=0.996、PNG 実視認で
全面 green）のに、同一時刻の同一 rect の screen composite は
green=0 かつ flat dark。

### own-process inventory（新機構の実効確認）

`uc_inventory`: `supported=true`、`total=425`、cap 256 で
`truncated=true` のまま、`own_windows` に自 process top-level
7 件を cap 非依存で収録（visible 4 件 z=22–25、invisible 3 件
z=287–289 — `rect=null`・`visible=false` を正直に記録）。前
slice の「own window が cap で切れて z_index を読めない」制約を
解消。

## 判別（観測事実と解釈の分離）

presentation-probe で未確定だった 2 候補への結論:

### (i) 外部 HWND 遮蔽の残存 — **棄却**

topmost 適用後、`WindowFromPoint` 5 点全てが
`RuviformTestEditor`（editor_child）を返し、`uc_uncovered` の
集計も `uncovered=true`。HWND 層では foreign cover は存在しな
い。それでも screen green=0。

### (ii) pre-paint / unsettled frame — **棄却**

topmost 直後・+200ms・+500ms・+1200ms の 4 capture が同一
sha256 の安定 raster。case body は caller thread で sleep し、
GPUI app thread は pump を継続しているため、1200ms は DWM/
compositor に数十 frame の猶予がある。settle で green が現れな
い以上、「まだ描かれていない frame」を理由にはできない。

### (iii) compositor-internal / presentation-layer occlusion — **支持（本 slice の結論）**

- DComp off で root window の redirection bitmap が
  green≈0.996・child window DC が 1.0 を持つ一方、同一時刻の
  screen composite は flat dark のまま — **最終合成は window
  bitmap とは別の surface を源泉にしている**。
- `update_pending=true` が settle 全域で持続 — child の classic
  update region が validate されず、WM_PAINT 駆動の paint 経路は
  on-screen 内容の生成元ではない。
- `GPUI_DISABLE_DIRECT_COMPOSITION` は exstyle と bitmap 内容を
  変えたが screen 結果を一切変えなかった — GPUI の実 presentation
  は env var とは独立の visual/swapchain 経路で、そこに plugin
  child 画素が到達しない。

### 正直な留保

- 「別 surface」の内部機構（DComp visual tree の何が flat dark
  を提示しているか — 空 swapchain、covering visual、flip-model
  plane 等）は HWND 層 probe の原理的射程外。**機構の同定は未
  解決**であり、compositor 内部の direct observation は本 slice
  の scope 外。
- `uc_observe_restore` で hits が `editor_child` のまま残るのは
  実測どおりの記録（`HWND_NOTOPMOST` 復元後も probe 点上で自
  window が foreign cover より上に残った）。topmost 帯の復元
  自体は `topmost_restored=true` で検証済み。

## Lane verdict サマリ

| case | DComp on | DComp off |
| --- | --- | --- |
| `real-dcomp-pixel` | fail（既知 regression、変化なし） | fail（同左） |
| `real-dcomp-foreground` | pass | pass |
| `real-dcomp-present` | pass | pass |
| `real-dcomp-uncovered` | **pass**（probe 完走 + restore 検証） | **pass** |
| その他 18 case | pass/unsupported（前 slice と同一分布） | 同左 |

両 run の lane exit code は `real-dcomp-pixel` の `Fail` による
設計どおりの 1。

## 後続 slice への入力（修正方式の検討材料）

- 残余遮蔽は compositor-internal / presentation-layer に確定 —
  外部 window 管理（z-order/foreground/環境準備）では解消不可。
- remediation 候補の探索対象は GPUI DComp visual tree の生成・
  attach 経路（child HWND 画素が GPUI swapchain に到達しない
  点）であり、hit-test に現れない covering 要因の特定には
  DComp/Visual 層を観測できる別手段が必要。
- `update_pending` の持続は「child の WM_PAINT が走っていない」
  副証拠 — plugin window が需要駆動（`WM_PRINT`）でのみ raster
  を生成し、通常 present 経路では一度も paint されていない可能
  性を示す。
