# Issue #532 presentation probe — 判別記録

Issue `scarecrow173/ruviform#532` の presentation-probe slice
（`real-dcomp-present` case + `src/ospres.rs` + `ospix` capture
variants）の実走結果と、foreground-probe の残余 3 候補に対する
判別解釈の記録。**Issue #532 は open のまま** — 本書は診断証拠で
あり、compositor 修正・方式選定ではない。

- Date: 2026-11-16
- OS: Windows (x86_64)、interactive desktop
- Lane JSONL（repo_sha = `81308574` 系、実装 HEAD `81308574`）:
  - `evidence/rf631/plugin-dock-real-scenario.jsonl`
    （baseline、DComp 有効 — `mise run test-ui-plugin-dock-real`）
  - `evidence/rf631/plugin-dock-real-scenario-dcomp-disabled.jsonl`
    （`GPUI_DISABLE_DIRECT_COMPOSITION=1`、`--pixel-artifacts
    evidence/rf631/presentation-probe` 付き）
  - `evidence/rf631/plugin-dock-real-headless.jsonl`
    （`--blocked-probe`、15 case 全て `not_run` + fail-closed）
  - 複写: `evidence/rf532/plugin-dock-real-presentation-dcomp-on.jsonl`
    / `-dcomp-off.jsonl`（同一 run の slice 側コピー）
- Pixel artifacts: `evidence/rf631/presentation-probe/*.png`（22 枚、
  DComp-off run のみ — baseline run は artifact 未指定で非保存）

## Probe 概略

editor child attach 済み・lease 生存中に、観測専用で以下を実行する
（window 作成/破棄/移動/restyle/z-order/foreground 変更は一切
行わない。`PrintWindow` のみ `WM_PRINT`/`WM_PRINTCLIENT` を対象
window に送る — 設計承認済みの read path であり、suspect raster
取得に必要）:

1. `ospres::inventory` — `EnumWindows` top-level 全列挙
   （z-order/class/rect/visible/alive/disabled/exstyle/owner/pid、
   cap 256 + truncated 記録）
2. `ospres::attributes` — child/container/root 各々の
   `DWMWA_CLOAKED`・`GetWindowRgn`/`GetRgnBox`・`GetUpdateRect`・
   `GetLayeredWindowAttributes`（属性ごとに独立 degrade）
3. `osoccl::probe` — 既存 ancestry/z-order/`WindowFromPoint`
   hit-test facts（suspect 選定に使用）
4. capture variants — child/container/root 各々に
   `PrintWindow` flags ∈ {0, `PW_CLIENTONLY`,
   `PW_RENDERFULLCONTENT`, both} + window-DC（`GetDC`+`BitBlt`）
5. `pres_screen` — child rect の screen `BitBlt`（最終合成 oracle）
6. `pres_fingerprint` — screen raster vs chain/suspect raster の
   5bit/channel quantized bin fingerprint（overlap 領域のみ、
   cap 8）

`real-dcomp-present` の verdict は「baseline 評価可能・計画した
観測 family が全て実行・lifecycle/cleanup 収束」のみで判定する。
green 画素・hit-test・fingerprint match は **verdict 入力にしない**
（note 級 evidence data）。両 run とも `pass`。

## 実走結果

### `real-dcomp-present` verdict: 両 run とも `pass`（34 steps 完走）

| family | baseline (DComp on) | DComp disabled |
| --- | --- | --- |
| inventory | supported, total=431, truncated=true（cap 256） | 同左 |
| attributes | cloaked=0、region query error(0)、update=child のみ has_update | 同左 |
| cover candidates | 16 件（`covers_child` 判定付き、全て foreign process） | 同左 16 件 |
| captures | `pres_capture` 15 rows（flags 4 種 × 3 target + windowdc × 3、一部 error 記録）+ `pres_screen`/`pres_fingerprint` 各 1 | 同左 |
| screen | `win32-bitblt-screen` green=0/1,019,844 | 同左 |
| fingerprint | chain 2 + suspect 4、全て match_fraction=0.0 | 同左 |
| lifecycle | close→detach→concluded→events 全 pass | 同左 |

### Capture variants（child rect 1,278×798、green=EDITOR_COLOR 判定）

| target | path | flags | DComp on green | DComp off green |
| --- | --- | --- | --- | --- |
| editor_child | printwindow | 0 | **1.0000** | **1.0000** |
| editor_child | printwindow | `PW_CLIENTONLY` | **1.0000** | **1.0000** |
| editor_child | printwindow | `PW_RENDERFULLCONTENT` | 0.0000 | 0.0000 |
| editor_child | printwindow | both | 0.0000 | 0.0000 |
| editor_child | windowdc | — | 0.0000 | **1.0000** |
| container | printwindow | 0 / `PW_CLIENTONLY` | error 87 | error 87 |
| container | printwindow | `PW_RENDERFULLCONTENT` / both | 0.0000 | 0.0000 |
| container | windowdc | — | 0.0000 | **1.0000** |
| root | printwindow | 0 | 0.0000 | **0.9379** |
| root | printwindow | `PW_CLIENTONLY` | 0.0000 | **0.9959** |
| root | printwindow | `PW_RENDERFULLCONTENT` / both | 0.0000 | 0.0000 |
| root | windowdc | — | 0.0000 | **0.9959** |
| screen (child rect) | bitblt-screen | — | 0.0000 | 0.0000 |

読み取り:

- `PrintWindow` flags=0/`PW_CLIENTONLY`（WM_PRINT 系）は child の
  raster に **全面 green** を返す — child は需要駆動で green を
  描ける。
- `PW_RENDERFULLCONTENT` 系は **全 target で green=0**（DComp on/off
  共通）— この flag path は本環境で有効な画素を返さない
  （flat dark `(0,40,40)` raster — artifact PNG で確認）。
- window-DC: DComp **off** では child/container/root 全て green 優位
  （child 1.0、container 1.0、root ~0.996）。DComp **on** では
  child/container/root の window-DC は全て green=0。
  （run 間変動の正直な記録: review 修正前の初回 run では DComp-on
  の container window-DC が 1.0 を返した — window-DC の内容は
  paint state 依存で run ごとに変わり得る。いずれの値も verdict
  入力ではない。）
- screen BitBlt は両 mode で green=0 — child rect の最終合成に
  green が存在しない。

### Cover inventory / hit-test / fingerprint

- `pres_inventory`: top-level 431 windows（cap 256 で truncated）。
  child rect `{321,153,1278×798}` と交差する visible foreign
  cover candidate を複数記録（例: `Chrome_WidgetWin_1` z=22
  hwnd=67906 rect `{-8,-8,1936×1048}` `covers_child=true`、
  `CodexComputerUseSwiftOverlay`・`Wnd_StartFeed` 等 — いずれも
  `visible=false` または fingerprint 対象外）。
- **inventory の限界（正直な記録）**: cap 256 により自 process
  window（`own_process=true`）は今回の snapshot に **0 件** —
  431 windows 中の後方 z-order が切り捨てられ、GPUI root 自身の
  z_index は本 inventory からは読めない。cover 判定は記録済み
  256 件の範囲内でのみ成立する。
- `pres_attributes`: child/container/root 全て `cloaked=0`、
  `GetWindowRgn` error(0)、`GetLayeredWindowAttributes`=null —
  DWM cloak/region/layered による隠蔽の兆候なし。
- `pres_fingerprint`: screen top colors は `(248,248,248)` 系
  light-gray 優位（87.6 万画素）— chain/suspect 全 raster と
  `match_fraction=0.0`。screen が写しているのは suspect のいずれの
  raster でもない（Chrome 系 window の `PrintWindow` は実画素を
  返さないため、Chrome そのものとの一致は本 fingerprint では
  判別不能 — 限界として記録）。
- `osoccl` facts（suspect 選定元）: `WindowFromPoint` の hit-test
  window は foreign `Chrome_RenderWidgetHostHWND`（hwnd=67908、
  rect `{0,0,1920×1032}`）— child/parent/root のいずれでもない。

### `real-dcomp-foreground` stage data（既存 case、note 級）

| stage | baseline (DComp on) | DComp disabled |
| --- | --- | --- |
| baseline snapshot | `above_count=399`, topmost=false, foreground=264620（inventory 上 `Chrome_WidgetWin_1` vis=true） | 同左（`above_count=399`） |
| baseline hits | `Chrome_RenderWidgetHostHWND` ×5 | `Chrome_RenderWidgetHostHWND` ×5 |
| raise (`HWND_TOP`) | hits 変わらず Chrome、`above_count` 398 | 同左 |
| foreground | `SetForegroundWindow` refused（foreground lock、`applied:false`） | 同左 |
| **topmost** | **hits = `RuviformTestEditor`（child）×5**、`above_count` 7 | 同左 |
| restore | hits Chrome に復帰 | 同左 |
| screen green 全 stage | 0.0 | 0.0 |

### Pixel artifact による画素レベル確認（DComp-off run）

`evidence/rf631/presentation-probe/` の PNG を画素解析:

- baseline/raise/foreground/restore の screen capture は **全て同一
  histogram**（light-gray `(240,240,240)` 優位 ≈86%）— Chrome window
  の静的画面を捉えている。
- **topmost の screen capture は全面 flat dark `(32,32,32)`**
  （`(0,40,40)` bin で 19,242/19,243 画素が単一色）— Chrome ではなく
  **別の新規 surface**。framebuffer が topmost 化に追従して更新
  されたことを示す（stale frame ではない）。
- child window-DC / child printwindow f0・f1 / container windowdc /
  root printwindow f0・f1 / root windowdc は green 優位。
- `PW_RENDERFULLCONTENT` 系と `real-dcomp-pixel` の printwindow
  （別 child instance）は flat dark — 「描画要求に応答しない/内容を
  持たない」capture 結果として記録（verdict には不使用）。

## 過去 run との対比（HEAD 保存の旧 JSONL より）

foreground-probe slice 当時の 2 run（いずれも screen green=0 全
stage）:

- 旧 baseline (DComp on): `above_count=31`、baseline hits =
  `Chrome_RenderWidgetHostHWND`、topmost hits = child。
- 旧 dcomp-disabled: `above_count=28`、baseline hits =
  **`Zed::Window`（role=other — 自 process ではない別 GPUI
  アプリの window）**、topmost hits = child。

→ **4 run 全てで「topmost stage では hit-test が自 child を返すのに
screen green=0」**。baseline の cover 相手は環境ごとに異なる
（Chrome / 別 GPUI アプリ / 今回は maximized Chrome + 399 windows
above）が、topmost での screen=0 は環境非依存に再現する。

## 判別解釈（観測事実と仮説を分離）

対象 3 仮説:

- **(a) hit-test に透明な covering surface/visual**
- **(b) child HWND 自身の presentation/redirection/flip-model 経路**
- **(c) screen capture の blind spot**

### (c) screen capture blind spot — 反証あり（ほぼ棄却）

`BitBlt` screen capture は topmost 化の前後で Chrome light-gray →
flat dark へ **内容を追従して更新**されており、最終合成を忠実に
捉えている。capture 系そのものの盲点を示す証拠はない。

### (a) 外部 covering surface — baseline では証明済み、compositor 層は未解決

- baseline/raise/foreground/restore の全 stage で、child rect 内
  5 点の `WindowFromPoint` は foreign `Chrome_RenderWidgetHostHWND`
  を返し、`above_count=399`・foreground owner も Chrome — **HWND
  z-order 層の外部遮蔽は観測事実として確定**。この状態では screen
  oracle は GPUI window の合成結果を原理的に反映しない。
- ただし topmost stage では hit-test が自 child に切り替わる =
  HWND 層の cover は除去される。それでも screen green=0。
- GPUI 自身の DComp visual tree 内部に hit-test 非可視の covering
  visual がある可能性は HWND inventory/hit-test の原理的射程外で
  **残る**（positive/negative いずれの証拠もなし）。

### (b) child の presentation 経路 — 最有力だが settle 未検証の留保あり

支持事実:

1. child の green は `WM_PRINT` 需要駆動 capture と（DComp-off の）
   window-DC にのみ存在し、**持続 surface としては DComp-on で
   window-DC green=0** — child が自身の back surface に green を
   保持していない可能性。
2. topmost で自 window が hit-test 最上位でも screen 合成に green
   が現れないことが **4 run・DComp on/off 双方で一貫**。
3. topmost の screen PNG は flat dark の**新規 surface** —
   「覆われている」のではなく「自 window の合成結果に child 画素が
   含まれない」ことを示唆。

留保（正直な記録）:

- topmost 観測は `SetWindowPos` 直後・settle 無しの即時 capture。
  Windows の occlusion culling / DWM の再合成・初回 paint には
  数 frame の lag があり得るため、flat dark は **pre-paint frame
  （未描画 surface）** の可能性を排除できない。この場合 green=0 は
  「まだ描かれていない」だけで compositor occlusion の証拠にならない。
- DComp-off で child window-DC が green=1.0 を持つ以上、settle 後に
  topmost が維持されれば screen に green が出る可能性は残る —
  その確認は未実施。

### 総合判別

- 観測された screen green=0 症状は **(1) 外部 HWND z-order 遮蔽**
  （今回 run で実証: foreground 化されない test window + 前面の
  foreign window）で**完全に説明可能**であり、**(2) uncovered
  状態でも child 画素が最終合成に到達しない可能性**が topmost
  データから示唆されるが settle 未検証のため未確定。
- `GPUI_DISABLE_DIRECT_COMPOSITION=1` は exstyle
  （`noredirectionbitmap` true→false）と window-DC green に実効が
  あったが、**screen/hit-test の結果は一切変えなかった** — 少なく
  とも本環境で観測された失敗は GPUI の DComp presentation 経路の
  切替では説明できない。
- compositor 修正の方式選定に必要な「uncovered baseline」が未取得
  — 外部遮蔽を除いた状態での screen 観測が Issue #532 判別の
  次の必須実験。

## 残課題（後続 slice 候補、修正ではない）

1. **settle 付き topmost 再観測**: `real-dcomp-foreground` の
   topmost stage に数 frame 分の settle（sleep/pump）を入れてから
   screen capture — pre-paint frame か真の非提示かを判別する
   最小の追加実験。
2. **uncovered baseline の環境確保**: test window を foreground 化
   するか、実行前に foreground foreign window を退避させた環境で
   baseline を取り直す（環境準備の問題であり probe 変更とは別）。
3. inventory cap の引き上げまたは own-process 優先列挙 —
   自 window の z_index を inventory から直接読むため
   （現状は `osoccl` snapshot の `above_count` で代替）。
4. `PW_RENDERFULLCONTENT` が全 target で flat dark を返す理由の
   切り分け（flag path 自体の非対応か、対象 window の状態か）。

## Lane verdict サマリ

| case | baseline | DComp disabled |
| --- | --- | --- |
| `real-dcomp-pixel` | fail（既知の正直な Fail、screen green=0） | fail（同左 — 変化なし） |
| `real-dcomp-foreground` | pass | pass |
| `real-dcomp-present` | **pass**（probe 完走） | **pass**（probe 完走） |
| その他 12 case | pass/unsupported（前 slice と同一分布） | 同左 |

lane exit code は `Fail` verdict による正常な 1（設計どおり）。
`real-dcomp-pixel` の verdict ロジックは本 slice で不変。
